module dielectric;

import camera;
import path;
import queue;
import random;
import bvh;

public struct DielectricData {
  float4 albedo;
  float ir;
};

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_material;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_material;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] StructuredBuffer<DielectricData> lb_data;

[[vk::binding(0,4)]] StructuredBuffer<uint> light_samples;

[[vk::binding(0,5)]] StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(1,5)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,5)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,5)]] StructuredBuffer<float3> tri_normals;
[[vk::binding(4,5)]] StructuredBuffer<uint> blas_roots;

float reflectance(float cosine, float ref_idx) {
  var r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
  r0 *= r0;
  return r0 + (1.0 - r0) * pow((1.0 - cosine), 5);
}

float3 reflect_vec(float3 v, float3 n) {
  return v - 2.0 * dot(v, n) * n;
}

float3 refract_vec(float3 uv, float3 n, float eta) {
  float cos_theta = min(-dot(uv, n), 1.0);
  float3 r_out_perp = eta * (uv + cos_theta * n);
  float k = 1.0 - dot(r_out_perp, r_out_perp);
  float3 r_out_parallel = -sqrt(max(k, 0.0)) * n;
  return r_out_perp + r_out_parallel;
}

[shader("compute")]
[numthreads(64,1,1)]
void computeMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_material, qd_material, index) == -1) {
    return;
  }

  let path = &paths[index];
  let hit = &hits[index];
  let mat_data = lb_data[hit.mat_data];

  let front_face = dot(hit.hit.norm, path.ray.dir) < 0;
  let ffs = front_face ? 1 : -1;
  hit.hit.norm *= ffs;

  let ir = mat_data.ir;
  let refraction_ratio = front_face ? (1.0 / ir) : (ir);

  let unit_direction = normalize(path.ray.dir);
  let cos_theta = min(-dot(unit_direction, hit.hit.norm), 1.0);
  let sin_theta = sqrt(max(0.0, 1.0 - pow(cos_theta,2)));
  let cannot_refract = refraction_ratio * sin_theta > 1.0;
  let reflectance_val = reflectance(cos_theta, refraction_ratio);

  RandomResult result = Random(random_states[index]);
  random_states[index] = result.state;

  float3 scatter;
  if (cannot_refract || reflectance_val > result.value) {
    // Reflect the ray:
    scatter = reflect_vec(unit_direction, hit.hit.norm);
    path.ray.pos = hit.hit.pos + hit.hit.norm * 1e-4;
  } else {
    // Refract the ray:
    scatter = refract_vec(unit_direction, hit.hit.norm, refraction_ratio);
    path.ray.pos = hit.hit.pos - hit.hit.norm * 1e-4;
  }
  
  path.throughput = mat_data.albedo.xyz;
  path.ray.dir = normalize(scatter);
  
  queuePush(qh_extension, qd_extension, index);
}
