module logic;

import path;
import queue;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(2,0)]] RWStructuredBuffer<SampleState> sample_states;
[[vk::binding(3,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_new_ray;

[[vk::binding(0,2)]] RWStructuredBuffer<uint> output;

[[vk::binding(0,3)]] RWStructuredBuffer<QueueHeader> qh_lambertian;
[[vk::binding(1,3)]] RWStructuredBuffer<uint> qd_lambertian;

[shader("compute")]
[numthreads(1,1,1)]
void logicStart(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_new_ray);
    queueReset(qh_lambertian);
    return;
  }
}

[shader("compute")]
[numthreads(16,16,1)]
void logicMain(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x > 1024 || threadId.y > 1024) {
    return;
  }
  uint32_t idx = threadId.x + threadId.y * 1024;

  let path = &paths[idx];
  let sample_state = &sample_states[idx];
  let hit = &hits[idx];

  if (path.generated == 0) {
    // Path has never been generated
    // Set the pos:
    path.screen_pos = float2(threadId.xy) / 1024.0;
    path.bounces = 32;
    path.terminated = 0;
    path.throughput = float3(1.0);
    path.rad = float3(0.0);

    // Push to the queue:
    queuePush(qh_new_ray, qd_new_ray, idx);
    path.generated = 1;
  } else if (path.terminated == 1) {
    // Path is terminated, add radiance to the buffer
    // output[idx] = pack_rgb(path.rad.xyz);
    sample_state.samples += 1;
    // path.samples = min(path.samples,20);
    sample_state.sampled_rad = sample_state.sampled_rad * (float(sample_state.samples-1) / float(sample_state.samples)) + path.rad  / float(sample_state.samples);
    output[idx] = pack_rgb(sample_state.sampled_rad.xyz);
    path.generated = 0;
  } else {
    // Path is not yet terminated this is where we would give it to the material it hit
    // but for now im just putting it straight back on the extension queue.
    switch (hit.mat) {
      case 0:
        // No material??? Shouldn't get here.
        output[threadId.x + threadId.y * 1024] = pack_rgb(float3(0.0,0.0,255.0));
        break;
      case 1:
        // Lambertian
        queuePush(qh_lambertian, qd_lambertian, threadId.x + threadId.y * 1024);
        break;
      case 2:
        // Lambertian
        queuePush(qh_lambertian, qd_lambertian, threadId.x + threadId.y * 1024);
        // path.rad = float3(1.0,0.0,0.0);
        // path.terminated = 1;
        break;
      default:
        break;
    }
  }
}

uint pack_rgb(float3 color) {
    color = saturate(color);
    let r = uint(color.r * 255.0) << 16;
    let g = uint(color.g * 255.0) << 8;
    let b = uint(color.b * 255.0) << 0;
    return r | g | b;
}

