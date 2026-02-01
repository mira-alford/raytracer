module new_ray;

import camera;
import path;
import queue;
import random;
import sample;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_new_ray;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] ConstantBuffer<Camera> camera;

[[vk::binding(0,4)]] ConstantBuffer<uint2> dims;

[[vk::binding(0,5)]] RWStructuredBuffer<uint> sample_index;
[[vk::binding(1,5)]] RWStructuredBuffer<SampleData> sample_data;
[[vk::binding(2,5)]] RWStructuredBuffer<float4> sample_mean;
[[vk::binding(3,5)]] RWStructuredBuffer<float4> sample_std;

[shader("compute")]
[numthreads(64,1,1)]
void newRayMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_new_ray, qd_new_ray, index) == -1) {
    return;
  }

  let right = cross(camera.forward, camera.up);

  let path = &paths[index];
  path.ray.pos = camera.position;

  let top_left = camera.forward * camera.focal_length
                  + camera.up * camera.dims.y
                  - right * camera.dims.x;

  // Add random value in [0.0, 1.0 / screen dimensions] to the x,y
  RandomResult result = Random(random_states[index]);
  RandomResult result2 = Random(result.state);
  random_states[index]= result2.state;
  let d = float2(result.value, result2.value) / float2(dims.x, dims.y);

  let screen_pos = sample_data[path.sample_id].screen_pos;
  let offset = -2.0 * camera.up * camera.dims.y * (screen_pos.y + d.y)
                + 2.0 * right * camera.dims.x * (screen_pos.x + d.x);

  float3 dir = top_left + offset;
  path.ray.dir = normalize(dir);

  // If we move too much, we reset the multi sampling :(
  // let sample_state = &sample_states[index];
  // if (length(sample_state.sampled_pos - (path.ray.pos + path.ray.dir)) > 0.01) {
  //   sample_state.sampled_pos = path.ray.pos + path.ray.dir;
  //   sample_state.samples = 0;
  //   sample_state.sampled_rad = float3(0.0);
  // }
 
  queuePush(qh_extension, qd_extension, index);
}
