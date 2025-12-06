module new_ray;

import camera;
import path;
import queue;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_new_ray;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,3)]] ConstantBuffer<Camera> camera;

[shader("compute")]
[numthreads(256,1,1)]
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

  let offset = -2.0 * camera.up * camera.dims.y * path.screen_pos.y
                + 2.0 * right * camera.dims.x * path.screen_pos.x;

  float3 dir = top_left + offset;
  path.ray.dir = normalize(dir);
  
  queuePush(qh_extension, qd_extension, index);
}
