module logic;

import path;
import queue;
import sample;
import camera;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_new_ray;

[[vk::binding(0,2)]] RWStructuredBuffer<uint> output;

[[vk::binding(0,3)]] ConstantBuffer<uint2> dims;

[[vk::binding(0,4)]] RWStructuredBuffer<uint> sample_index;
[[vk::binding(1,4)]] RWStructuredBuffer<SampleData> sample_data;
[[vk::binding(2,4)]] RWStructuredBuffer<float4> sample_mean;
[[vk::binding(3,4)]] RWStructuredBuffer<float4> sample_std;

[[vk::binding(0,5)]] ConstantBuffer<Camera> camera;

[[vk::binding(0,6)]] RWStructuredBuffer<QueueHeader> qh_lambertian;
[[vk::binding(1,6)]] RWStructuredBuffer<uint> qd_lambertian;

[[vk::binding(0,7)]] RWStructuredBuffer<QueueHeader> qh_metallic;
[[vk::binding(1,7)]] RWStructuredBuffer<uint> qd_metallic;

[shader("compute")]
[numthreads(1,1,1)]
void logicStart(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_new_ray);
    queueReset(qh_lambertian);
    queueReset(qh_metallic);
    return;
  }
}

[shader("compute")]
[numthreads(16,16,1)]
void maintainSamples(uint3 threadId : SV_DispatchThreadID) {
  // Cycles the samples buffer, swap-adding to the active
  // sample list where cycles need reactivation.
}

[shader("compute")]
[numthreads(16,16,1)]
void maintainActiveSamples(uint3 threadId : SV_DispatchThreadID) {
  // Pull of the sample completion queue and deactivates those samples
  // swap-removing from the active sample list.
}

[shader("compute")]
[numthreads(64,1,1)]
void logicMain(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x > paths.getCount()) {
    return;
  }

  let path = &paths[threadId.x];
  let hit = &hits[threadId.x];

  if (path.sampled == 0) {
    // Path has not been sampled
    // Pull a sample from the pool:
    uint sample_id;
    InterlockedAdd(sample_index[0], 1, sample_id);
    sample_id %= sample_data.getCount();
    let sample_data = sample_data[sample_id];

    // Generate the path starting conditions:
    path.bounces = 8;
    path.throughput = float3(1.0);
    path.rad = float3(0.0);
    path.sampled = 1;
    path.terminated = 0;
    path.sample_id = sample_id;
    queuePush(qh_new_ray, qd_new_ray, threadId.x);
  } else if (path.terminated == 1) {
    // Path is terminated, update the sample data:
    sample_data[path.sample_id].samples += 1;
    let samples = sample_data[path.sample_id].samples;
    if (camera.changed != 0) {
      sample_data[path.sample_id].samples = min(samples, 1);
    }

    sample_mean[path.sample_id].xyz = sample_mean[path.sample_id].xyz
      * (float(samples-1) / float(samples))
      + path.rad / float(samples);

    // Write the sampled radiance to the output buffer:
    let out_pos = sample_data[path.sample_id].out_pos;
    let out_idx = out_pos.x + out_pos.y * dims.x;
    output[out_idx] = pack_rgb(sample_mean[path.sample_id].xyz);

    // Re-mark the path to be sampled:
    path.sampled = 0;
  } else {
    // Path is not yet terminated this is where we would give it to the material it hit
    // but for now im just putting it straight back on the extension queue.
    switch (hit.mat) {
      case 0:
        // No material??? Shouldn't get here.
        output[threadId.x + threadId.y * dims.x] = pack_rgb(float3(0.0,0.0,255.0));
        break;
      case 1:
        // Lambertian
        queuePush(qh_lambertian, qd_lambertian, threadId.x + threadId.y * dims.x);
        break;
      case 2:
        // Lambertian
        queuePush(qh_metallic, qd_metallic, threadId.x + threadId.y * dims.x);
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

