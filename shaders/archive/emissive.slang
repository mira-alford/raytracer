module emissive;

import camera;
import path;
import queue;
import random;
import material;

public struct EmissiveData : IMaterial {
  float4 albedo;

  public float4 brdf(float3 wi, float3 wo, float3 n) {
    return 0.0;
  }

  public float pdf(float3 wi, float3 wo, float3 n) {
    return 0.0;
  }

  public bool scatter(out float3 wi, float3 wo, float3 n, inout uint4 random_state) {
    return false;
  }

  public bool emit(out float4 colour) {
    colour = albedo;
    return true;
  }
};

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_material;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_material;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] StructuredBuffer<EmissiveData> em_data;

[shader("compute")]
[numthreads(64,1,1)]
void computeMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_material, qd_material, index) == -1) {
    return;
  }

  let path = &paths[index];
  let hit = &hits[index];
  let mat_data = em_data[hit.mat_data];

  float4 colour;
  mat_data.emit(colour);
  path.rad += colour.rgb * path.throughput;

  path.terminated = 1;
}
