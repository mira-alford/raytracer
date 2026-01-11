module metallic;

import camera;
import path;
import queue;
import random;
import bvh;
import material;

public struct MetallicData : IMaterial {
  float4 albedo;
  float fuzz;

  public float4 brdf(float3 wi, float3 wo, float3 n) {
    let oob = (dot(n, wi) <= 0.0 || dot(n, wo) <= 0.0);
    return select(oob, 0.0, albedo / 3.1415);
  }

  public float pdf(float3 wi, float3 wo, float3 n) {
    // let cosTheta = dot(n, wi);
    // return select(cosTheta > 0.0, 1.0 / 6.2831, 0.0);
    return max(0.0, dot(wi, n)) / 3.1415;
  }

  public bool scatter(out float3 wi, float3 wo, float3 n, inout uint4 random_state) {
    let reflected = -wo - 2.0 * dot(-wo, n) * n;

    RandomResult result = Random(random_state);
    RandomResult result2 = Random(result.state);
    RandomResult result3 = Random(result2.state);
    let random_unit_sphere = normalize(1.0 - 2.0 * float3(result.value, result2.value, result3.value));

    wi = reflected + random_unit_sphere * fuzz;
    wi = normalize(select(length(wi) < 1e-6, reflected, wi));

    return true;
  }

  public bool emit(out float4 colour) {
    colour = float4(0.0);
    return false;
  }
};

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_material;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_material;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] StructuredBuffer<MetallicData> lb_data;

[[vk::binding(0,4)]] StructuredBuffer<uint> light_samples;

[[vk::binding(0,5)]] StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(1,5)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,5)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,5)]] StructuredBuffer<float3> tri_normals;
[[vk::binding(4,5)]] StructuredBuffer<uint> blas_roots;

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
  if (!front_face) {
    hit.hit.norm *= -1;
  }

  path.ray.pos = hit.hit.pos + hit.hit.norm * 0.005;
  path.throughput *= mat_data.albedo.xyz;

  let reflected = path.ray.dir - 2.0 * dot(path.ray.dir, hit.hit.norm) * hit.hit.norm;
  RandomResult result = Random(random_states[index]);
  RandomResult result2 = Random(result.state);
  RandomResult result3 = Random(result2.state);
  random_states[index] = result3.state;
  let random_unit_sphere = normalize(1.0 - 2.0 * float3(result.value, result2.value, result3.value));
  float3 scatter = reflected + random_unit_sphere * mat_data.fuzz;

  path.ray.dir = normalize(select(length(scatter) < 1e-2, reflected, scatter));
  
  queuePush(qh_extension, qd_extension, index);
}
