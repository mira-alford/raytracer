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

struct RandomResult {
  uint4 state;
  float value;
};

uint TausStep(uint z, int S1, int S2, int S3, uint M) {
  uint b = (((z << S1) ^ z) >> S2);
  return (((z & M) << S3) ^ b);
}

uint LCGStep(uint z, uint A, uint C) {
  return (A * z + C);
}

RandomResult Random(uint4 state) {
  state.x = TausStep(state.x, 13, 19, 12, 4294967294);
  state.y = TausStep(state.y, 2, 25, 4, 4294967288);
  state.z = TausStep(state.z, 3, 11, 17, 4294967280);
  state.w = LCGStep(state.w, 1664525, 1013904223);

  RandomResult result;
  result.state = state;
  result.value = 2.3283064365387e-10 * (state.x ^ state.y ^ state.z ^ state.w);

  return result;
}

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

  // Add random value in [0.0, 1.0 / screen dimensions] to the x,y
  

  float3 dir = top_left + offset;
  path.ray.dir = normalize(dir);

  // If we move too much, we reset the multi sampling :(
  if (length(path.sampled_pos - (path.ray.pos + path.ray.dir)) > 0.02) {
    path.sampled_pos = path.ray.pos + path.ray.dir;
    path.samples = 0;
    path.sampled_rad = float3(0.0);
  }
 
  queuePush(qh_extension, qd_extension, index);
}
