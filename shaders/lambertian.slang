module lambertian;

import camera;
import path;
import queue;
import random;

public struct LambertianData {
  float4 albedo;
};

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_material;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_material;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] StructuredBuffer<LambertianData> lb_data;

[shader("compute")]
[numthreads(64,1,1)]
void newRayMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_material, qd_material, index) == -1) {
    return;
  }

  let path = &paths[index];
  let hit = &hits[index];
  let mat_data = lb_data[hit.mat_data];

  path.ray.pos = hit.hit.pos + hit.hit.norm * 0.005;
  // path.ray.pos = hit.hit.pos;
  path.throughput *= mat_data.albedo.xyz;

  RandomResult result = Random(random_states[index]);
  RandomResult result2 = Random(result.state);
  RandomResult result3 = Random(result2.state);
  random_states[index] = result2.state;
  float3 scatter = hit.hit.norm + normalize(1.0 - 2.0 * float3(result.value, result2.value, result3.value));
  scatter = select(length(scatter) < 1e-2, hit.hit.norm, scatter);

  // Actual lambertian scattering:
  path.ray.dir = normalize(path.ray.dir + scatter);
  
  queuePush(qh_extension, qd_extension, index);
}
