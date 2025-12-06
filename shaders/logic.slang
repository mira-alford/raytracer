module logic;

import path;
import queue;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_new_ray;

[[vk::binding(0,2)]] RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1,1,1)]
void logicStart(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_new_ray);
    return;
  }
}

[shader("compute")]
[numthreads(16,16,1)]
void logicMain(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x > 512 || threadId.y > 512) {
    return;
  }

  let path = &paths[threadId.x + threadId.y * 512];

  if (path.generated == 0) {
    // Path has never been generated
    // Set the pos:
    path.screen_pos = float2(threadId.xy) / 512.0;

    // Push to the queue:
    queuePush(qh_new_ray, qd_new_ray, threadId.x + threadId.y * 512);
    path.generated = 1;
  } else if (path.terminated == 1) {
    // Path is terminated, add radiance to the buffer
    // output[threadId.x + threadId.y * 512] = pack_rgb(oldWrite);
  } else {
    output[threadId.x + threadId.y * 512] = pack_rgb(path.rad.xyz);
  }
}

uint pack_rgb(float3 color) {
    color = saturate(color);
    let r = uint(color.r * 255.0) << 16;
    let g = uint(color.g * 255.0) << 8;
    let b = uint(color.b * 255.0) << 0;
    return r | g | b;
}

