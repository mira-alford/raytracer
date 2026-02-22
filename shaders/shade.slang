// shade.slang
//
// Performs shading of samples waiting in the shade queue.
// Shading entails computing the changes to sample state by
// using the most recent hit record.
// Shading also determines ray termination or bouncing,
// and will enqueue the sample for another extension accordingly.
module shade;

import common;
import scene;
import pathtracer;
import random;
import queue;
import bvh;
import math;
import ggx;

float pow2(float a) {
  return a * a;
}

float3 conductorFresnel(float3 wi, float3 wo, float3 f0, float3 bsdf) {
  float3 h = normalize(wo + wi); // half vector
  return bsdf * schlick3(abs(dot(wo, h)), f0);
}

float blinnPhongDist(float3 wi, float3 wo, float3 n, float alpha) {
  float3 h = normalize(wo + wi); // half vector
  return (alpha + 2.0) * pow(abs(dot(n, h)), alpha) / (2.0 * float.getPi());
}

float beckmannDist(float3 wi, float3 wo, float3 n, float m) {
  float3 h = normalize(wo + wi); // half vector
  float d = exp(
    (pow(abs(dot(n, h)), 2.0) - 1.0)
    / (pow(m, 2.0) * pow(abs(dot(n, h)), 2.0))
  );
  d /= float.getPi() * pow(m, 2.0) * pow(abs(dot(n, h)), 4.0);
  return d;
}

float cookTorranceGeom(float3 wi, float3 wo, float3 n) {
  float3 h = normalize(wo + wi); // half vector
  float g1 = 2.0 * dot(n, h) * dot(n, wo) / dot(wo, h);
  float g2 = 2.0 * dot(n, h) * dot(n, wi) / dot(wo, h);

  return min(min(1.0, g1), g2);
}

float smithGeom(float3 wi, float3 wo, float3 n, float alpha) {
  // float3 h = normalize(wo + wi); // half vector
  // float a2 = alpha * alpha;

  // float nwi2 = pow(dot(n, wi), 2.0);
  // float nwo2 = pow(dot(n, wo), 2.0);
  // float g = heaviside(dot(h, wi)) / (abs(dot(n, wi)) + sqrt(a2 + (1.0 - a2) * nwi2));
  // g *= heaviside(dot(h, wo)) / (abs(dot(n, wo)) + sqrt(a2 + (1.0 - a2) * nwo2));
  // g *= 4.0 * abs(dot(n, wi)) * abs(dot(n, wo));
  
  // return g;

  return GGXSmithGeom(wi, wo, n, alpha);
}

float microfacetBRDF(float3 wi, float3 wo, float3 n, float Fr, float G, float D) {
  // Fr: fresnel term
  // G: geometric attenuation term
  // D: microfacet distribution term
  float won = dot(n, wo);
  float win = dot(n, wi);
  if (win <= 0.0 || won <= 0.0) return 0.0;
  return Fr * G * D / (4.0 * won * win);
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#specular-brdf
float specularBRDF(float3 wi, float3 wo, float3 n, float alpha) {
  return microfacetBRDF(wi, wo, n, 1.0, smithGeom(wi, wo, n, alpha), GGXDist(wi, wo, n, alpha));
}

float3 diffuseBRDF(float3 colour) {
  return (1.0 / float.getPi()) * colour;
}

float3 specularBTDF(float3 wi, float3 wo, float3 n, float alpha) {
  float3 ht = normalize(wo - 2.0 * dot(n, wi) * n + wi); // half vector
  float a2 = alpha * alpha;

  float3 Dt = a2 * heaviside(dot(n, ht));
  Dt /= float.getPi() * pow((pow(abs(dot(n, ht)), 2.0) * (a2 - 1.0) + 1.0), 2.0);

  float3 Vt = heaviside(dot(ht, wi) / dot(n, wi));
  Vt *= heaviside(dot(ht, wo) / dot(n, wo));
  Vt /= abs(dot(n, wi)) + sqrt(a2 + (1.0 - a2)) * pow(abs(dot(n, wi)), 2.0);
  Vt /= abs(dot(n, wo)) + sqrt(a2 + (1.0 - a2)) * pow(abs(dot(n, wo)), 2.0);

  return Vt * Dt;
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#metal-brdf-and-dielectric-brdf
float3 dielectricBRDF(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  return fresnelMix(
    wi, wo, ms.ior,
    // mix(
      diffuseBRDF(ms.colour.rgb),
      // specularBTDF(wi, wo, n, pow(ms.roughness, 2.0)) * ms.colour.rgb,
      // ms.transmission
    // ),
    specularBRDF(wi, wo, n, max(pow(ms.roughness, 2.0),1e-4)),
  );
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#metal-brdf-and-dielectric-brdf
float3 metallicBRDF(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  float3 h = normalize(wo + wi); // half vector
  return conductorFresnel(
    wi, wo, ms.colour.rgb,
    specularBRDF(wi, wo, n, max(pow(ms.roughness, 2.0),1e-4))
  );
}

float3 material(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  return mix(dielectricBRDF(wi, wo, n, ms), metallicBRDF(wi, wo, n, ms), ms.metallic);
}

float2 unitDiskSample(int rng) {
  // TODO: https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations#SamplingaUnitDisk
  // Something like this in future
  float2 p;
  do {
    p = float2(2.0 * random_gen(randoms, rng) - 1.0, 2.0 * random_gen(randoms, rng) - 1.0);
  } while (length(p) > 1.0);
  return p;
}

float unitDiskPDF(float2 p) {
  return 1.0 / (2.0 * float.getPi());
}

float3 unitSphereSample(int rng) {
  float3 p;
  do {
    float r1 = random_gen(randoms, rng);
    float r2 = random_gen(randoms, rng);
    float r3 = random_gen(randoms, rng);
    p = float3(r1,r2,r3);
  } while (length(p) > 1.0);
  return normalize(p);
}

float unitSpherePDF(float3 p) {
  return 1.0 / (4.0 * float.getPi());
}

float3 cosineHemisphereSample(float3 n, int rng) {
  float2 d = unitDiskSample(rng);
  float z = sqrt(max(0.0, 1.0 - d.x * d.x - d.y * d.y));

  float3 temp = (abs(n.x) > 0.9) ? float3(0,1,0) : float3(1,0,0);
  float3 t1 = normalize(cross(n, temp));
  float3 t2 = cross(n, t1);

  float3 h = float3(d.x, d.y, z);
  return h.x * t1 + h.y * t2 + h.z * normalize(n);
}

float cosineHemispherePDF(float3 wi, float3 n) {
  return abs(dot(wi, n)) / float.getPi();
}

float3 fresnelBlendSample(float3 wo, float3 n, MaterialSample ms, int rng) {
  let f0 = mix(pow(abs((1.0 - ms.ior) / (1.0 + ms.ior)), 2.0), ms.colour.rgb, ms.metallic);
  let fr = schlick3(abs(dot(wo, n)), f0);
  let p_spec = max(fr.r, max(fr.g, fr.b));

  let kd = (1.0 - p_spec) * (1.0 - ms.metallic); // diffuse weight
  let ks = p_spec; // specular weight

  let sum = kd + ks;
 
  float c_specular = select(sum > 0, ks / sum, 1);
  float c_diffuse = select(sum > 0, kd / sum, 0);
  
  float3 wi;
  if (random_gen(randoms, rng) <= c_specular) {
    wi = GGXSample(wo, n, max(pow(ms.roughness, 2), 1e-4), rng);
  } else {
    wi = cosineHemisphereSample(n, rng);
  }

  return wi;
}

float fresnelBlendPDF(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  let f0 = mix(pow(abs((1.0 - ms.ior) / (1.0 + ms.ior)), 2.0), ms.colour.rgb, ms.metallic);
  // let f0 = mix(float3(0.04), ms.colour.rgb, ms.metallic);
  let fr = schlick3(abs(dot(wo, n)), f0);
  let p_spec = max(fr.r, max(fr.g, fr.b));

  let kd = (1.0 - p_spec) * (1.0 - ms.metallic); // diffuse weight
  let ks = p_spec; // specular weight

  let sum = kd + ks;

  float c_specular = select(sum > 0, ks / sum, 1);
  float c_diffuse = select(sum > 0, kd / sum, 0);

  float pdf_ggx = GGXPDF(wi, wo, n, max(ms.roughness * ms.roughness, 1e-4));
  float pdf_diffuse = cosineHemispherePDF(wi, n);
  float pdf_mix = c_specular * pdf_ggx + c_diffuse * pdf_diffuse;

  return pdf_mix;
  
}

[shader("compute")]
[numthreads(64,1,1)]
void shadeMain(uint3 threadId : SV_DispatchThreadID) {
  let idx = queueRead(shade_qh, shade_qd);
  if (idx < 0) {
    return;
  }

  let s = &samples[idx];
  let h = &extension_hit_records[idx];
  let ray = &extension_rays[idx];
  let wo = -ray.dir;

  Instance instance = instances[h.instance_id];

  let mat = materials[instance.material];
  MaterialSample ms = MaterialSample(mat.colour, mat.emissive, mat.metallic, max(mat.roughness, 1e-4), mat.ior, mat.transmission);
  ms.ior = 1.5;

  s.rad += s.throughput * mat.emissive.rgb;
  
  float3 n = h.vert.normal.xyz;
  n *= h.front_face != 0 ? 1.0 : -1.0;

  float3 wi;
  float pdf;

  // float fr = fresnel(abs(dot(wo, n)), ms.ior);
  // float w_gg = ms.metallic * (1.0 - r2) + (1.0 - ms.metallic) * fr * (1.0 - r2);
  // float w_gg = ms.metallic * (1.0 - ms.roughness);
  // float w_gg = 0.9 * ms.metallic;
  float w_gg = 0.5;
  float w_df = 1.0 - w_gg;
  int tag = 0;
  if (random_gen(randoms, idx) <= w_gg) {
    tag = 1;
    wi = GGXSample(wo, n, ms.roughness, idx);
  } else {
    wi = cosineHemisphereSample(n, idx);
  }
  pdf = 0.0;
  pdf += cosineHemispherePDF(wi, n) * w_df;
  pdf += GGXPDF(wi, wo, n, ms.roughness) * w_gg;
 
  float weight = 1.0;

  ray.dir = wi;
  ray.pos = h.vert.position.xyz;
  
  float3 brdf = material(wi, wo, n, ms);

  if (pdf > 1e-5) {
    s.throughput *= brdf * abs(dot(n, wi)) * weight / pdf;
    s.bounces -= 1;
  } else {
    s.throughput *= 0.0;
    s.bounces = 0;
  }

  if (s.bounces == 0) {
    queuePush(terminate_qh, terminate_qd, idx);
  } else {
    queuePush(extension_qh, extension_qd, idx);
  }
}
