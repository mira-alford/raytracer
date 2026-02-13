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

  // public float4 brdf(float3 wi, float3 wo, float3 n);

float3 conductorFresnel(float3 wi, float3 wo, float3 f0, float3 bsdf) {
  float3 h = normalize(wo + wi); // half vector
  return bsdf * (f0 + (1.0 - f0) * pow((1.0 - abs(dot(wo, h))), 5.0));
}

float3 mix(float3 dielectric_brdf, float3 metallic_brdf, float metallic) {
  return (1.0 - metallic) * dielectric_brdf + metallic * metallic_brdf;
}

float3 fresnelMix(float3 wi, float3 wo, float ior, float3 base, float3 layer) {
  float3 h = normalize(wo + wi); // half vector
  float f0 = pow((1.0 - ior) / (1.0 + ior), 2.0);
  float fr = f0 + (1.0 - f0) * pow(1.0 - abs(dot(wo, h)), 5.0);
  return mix(base, layer, fr);
}

float heaviside(float x) {
  return select(x.x > 0.0, 1.0, 0.0);
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#specular-brdf
float specularBRDF(float3 wi, float3 wo, float3 n, float alpha) {
  float3 h = normalize(wo + wi); // half vector
  float a2 = alpha * alpha;

  float d = a2 * heaviside(dot(n, h));
  d /= float.getPi() * pow(pow(dot(n, h), 2.0) * (a2-1.0) + 1.0, 2.0);

  float nwi2 = pow(dot(n, wi), 2.0);
  float nwo2 = pow(dot(n, wo), 2.0);
  float v = heaviside(dot(h, wi)) / (abs(dot(n, wi)) + sqrt(a2 + (1.0 - a2) * nwi2));
  v *= heaviside(dot(h, wo)) / (abs(dot(n, wo)) + sqrt(a2 + (1.0 - a2) * nwo2));

  return v * d;
}

float3 diffuseBRDF(float3 colour) {
  return (1.0 / float.getPi()) * colour;
}

float3 specularBTDF(float3 wi, float3 wo, float3 n, float alpha) {
  float3 ht = normalize(wo - 2.0 * dot(n, wi) * n + wi); // half vector
  float a2 = alpha * alpha;

  float3 Dt = a2 * heaviside(dot(n, ht));
  Dt /= float.getPi() * pow((pow(dot(n, ht), 2.0) * (a2 - 1.0) + 1.0), 2.0);

  float3 Vt = heaviside(dot(ht, wi) / dot(n, wi));
  Vt *= heaviside(dot(ht, wo) / dot(n, wo));
  Vt /= abs(dot(n, wi)) + sqrt(a2 + (1.0 - a2)) * pow(dot(n, wi), 2.0);
  Vt /= abs(dot(n, wo)) + sqrt(a2 + (1.0 - a2)) * pow(dot(n, wo), 2.0);

  return Vt * Dt;
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#metal-brdf-and-dielectric-brdf
float3 dielectricBRDF(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  float3 h = normalize(wo + wi); // half vector
  return fresnelMix(
    wi, wo, ms.ior,
    mix(
      diffuseBRDF(ms.colour.rgb),
      specularBTDF(wi, wo, n, pow(ms.roughness, 2.0)) * ms.colour.rgb,
      ms.transmission
    ),
    specularBRDF(wi, wo, n, pow(ms.roughness, 2.0)),
  );
}

// Based on https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#metal-brdf-and-dielectric-brdf
float3 metallicBRDF(float3 wi, float3 wo, float3 n, MaterialSample ms) {
  float3 h = normalize(wo + wi); // half vector
  return conductorFresnel(
    wi, wo, ms.colour.rgb,
    specularBRDF(wi, wo, n, pow(ms.roughness, 2.0))
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


float3 metallicSample(float3 wo, float3 n, float roughness, int rng) {
  float3 ref = reflect(wo, n);
  float3 dir = unitSphereSample(rng) * roughness;
  float3 wi = ref + dir;
  return normalize(select(length(wi) < 1e-6, dir, wi));
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
  let wo = ray.dir;

  Instance instance = instances[h.instance_id];

  let mat = materials[instance.material];
  MaterialSample ms = MaterialSample(mat.colour, mat.emissive, mat.metallic, mat.roughness, mat.ior, mat.transmission);

  s.rad += s.throughput * mat.emissive.rgb;
  
  float3 n = h.vert.normal.xyz;
  n *= h.front_face != 0 ? 1.0 : -1.0;

  float3 diffuse_sample = cosineHemisphereSample(n, idx);
  float3 metallic_sample = metallicSample(wo, n, ms.roughness, idx);

  let refraction_ratio = h.front_face == 0 ? (1.0 / ms.ior) : (ms.ior);
  float3 transmission_sample = refract(wo, n, refraction_ratio);

  float diffuse_pdf = cosineHemispherePDF(diffuse_sample, n);
  float metallic_pdf = cosineHemispherePDF(metallic_sample, n);
  float transmission_pdf = cosineHemispherePDF(transmission_sample, n);

  float sum_of_p = (diffuse_pdf + metallic_pdf + transmission_pdf);
  float diffuse_weight = diffuse_pdf / sum_of_p;
  float metallic_weight = metallic_pdf / sum_of_p;
  float transmission_weight = diffuse_weight / sum_of_p;

  float3 wi = cosineHemisphereSample(n, idx);
  float pdf = cosineHemispherePDF(wi, n);
  float weight = 1.0;

  // float3 wi;
  // float pdf;
  // float weight;
  // if (random_gen(randoms, idx) < ms.transmission) {
  //   wi = transmission_sample;
  //   weight = transmission_weight;
  //   pdf = transmission_pdf * 0.06;
  // } else if (random_gen(randoms, idx) <= ms.metallic) {
  //   wi = metallic_sample;
  //   weight = metallic_weight;
  //   pdf = metallic_pdf;
  // } else {
  //   wi = diffuse_sample;
  //   weight = diffuse_weight;
  //   pdf = diffuse_pdf;
  // }

  ray.dir = wi;
  ray.pos = h.vert.position.xyz;
  
  s.throughput *= material(wi, wo, n, ms) * abs(dot(n, wi)) * weight / pdf;
  s.bounces -= 1;

  if (s.bounces == 0) {
    queuePush(terminate_qh, terminate_qd, idx);
  } else {
    queuePush(extension_qh, extension_qd, idx);
  }
}
