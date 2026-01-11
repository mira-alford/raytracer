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
[[vk::binding(2,4)]] RWByteAddressBuffer sample_mean;
[[vk::binding(3,4)]] RWStructuredBuffer<float4> sample_std;

[[vk::binding(0,5)]] ConstantBuffer<Camera> camera;

[[vk::binding(0,6)]] RWStructuredBuffer<QueueHeader> qh_lambertian;
[[vk::binding(1,6)]] RWStructuredBuffer<uint> qd_lambertian;
[[vk::binding(2,6)]] RWStructuredBuffer<QueueHeader> qh_metallic;
[[vk::binding(3,6)]] RWStructuredBuffer<uint> qd_metallic;
[[vk::binding(4,6)]] RWStructuredBuffer<QueueHeader> qh_dielectric;
[[vk::binding(5,6)]] RWStructuredBuffer<uint> qd_dielectric;
[[vk::binding(6,6)]] RWStructuredBuffer<QueueHeader> qh_emissive;
[[vk::binding(7,6)]] RWStructuredBuffer<uint> qd_emissive;

[shader("compute")]
[numthreads(1,1,1)]
void logicStart(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_new_ray);
    queueReset(qh_lambertian);
    queueReset(qh_metallic);
    queueReset(qh_dielectric);
    queueReset(qh_emissive);
    return;
  }
}

[shader("compute")]
[numthreads(64,1,1)]
void maintainSamples(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x > sample_data.getCount()) {
    return;
  }

  let wgc = WorkgroupCount().x;
  let wgs = WorkgroupSize().x;
  let work = sample_data.getCount() / (wgc * wgs);

  for (int i = threadId.x * work; i < threadId.x * work + work; i++) {
    if (camera.changed != 0) {
      sample_data[i].samples = 1;
      sample_data[i].flags = 0;
      sample_mean.InterlockedExchange(i * sizeof(uint4) + 0 * sizeof(uint), 0);
      sample_mean.InterlockedExchange(i * sizeof(uint4) + 1 * sizeof(uint), 0);
      sample_mean.InterlockedExchange(i * sizeof(uint4) + 2 * sizeof(uint), 0);
      sample_mean.InterlockedExchange(i * sizeof(uint4) + 3 * sizeof(uint), 0);
    }
  }

}

[shader("compute")]
[numthreads(64,1,1)]
void maintainActiveSamples(uint3 threadId : SV_DispatchThreadID) {
  // Pull of the sample completion queue and deactivates those samples
  // swap-removing from the active sample list.
}

float3 acesToneMap(float3 hdr) {
  float3x3 m1 = float3x3(
      0.59719, 0.35458, 0.04823,
      0.07600, 0.90834, 0.01566,
      0.02840, 0.13383, 0.83770
  );
  float3x3 m2 = float3x3(
       1.60475, -0.53108, -0.07367,
      -0.10208,  1.10813, -0.00605,
      -0.00327, -0.07276,  1.07602
  );
  let v = mul(m1, hdr);
  let a = v*((v + 0.0245786)) - 0.000090537;
  let b = v*((0.983729 * v + 0.4329510)) + 0.238081;
  return saturate(mul(m2, a / b));
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
    
    if (sample_data.flags == 1) {
      // Skip this sample, resample it next frame i guess lol
      return;
    }

    // Generate the path starting conditions:
    path.bounces = 256;
    path.throughput = float3(1.0);
    path.rad = float3(0.0);
    path.sampled = 1;
    path.terminated = 0;
    path.sample_id = sample_id;

    hit.hit.prim = uint(-1);
    hit.hit.front_face = 0;
    hit.hit.norm = float3(0.0);
    hit.hit.pos = float3(0.0);
    hit.mat = 0;
    hit.mat_data = 0;

    queuePush(qh_new_ray, qd_new_ray, threadId.x);
  } else if (path.terminated == 1) {
    // Path is terminated, update the sample data:
    var samples = 0;
    InterlockedAdd(sample_data[path.sample_id].samples, 1, samples);

    sample_mean.InterlockedAdd(path.sample_id * sizeof(uint4) + 0 * sizeof(uint),
                                uint(path.rad.x * 1024.0));
    sample_mean.InterlockedAdd(path.sample_id * sizeof(uint4) + 1 * sizeof(uint),
                                uint(path.rad.y * 1024.0));
    sample_mean.InterlockedAdd(path.sample_id * sizeof(uint4) + 2 * sizeof(uint),
                                uint(path.rad.z * 1024.0));

    // Write the sampled radiance to the output buffer:
    let out_pos = sample_data[path.sample_id].out_pos;
    let out_idx = out_pos.x + out_pos.y * dims.x;

    float3 rad = float3(sample_mean.Load3(path.sample_id * sizeof(uint4))) / float(1024 * samples);
    
    rad *= exp2(-9.0); // TODO: compute this dynamically :)
    float3 rgb = acesToneMap(rad);
    output[out_idx] = pack_rgb(rgb);

    // Re-mark the path to be sampled:
    path.sampled = 0;
  } else {
    // Path is not yet terminated this is where we would give it to the material it hit
    // but for now im just putting it straight back on the extension queue.
    switch (hit.mat) {
      case 0:
        // No material??? Shouldn't get here.
        // output[threadId.x + threadId.y * dims.x] = pack_rgb(float3(0.0,0.0,255.0));
        break;
      case 1:
        queuePush(qh_lambertian, qd_lambertian, threadId.x);
        break;
      case 2:
        queuePush(qh_metallic, qd_metallic, threadId.x);
        break;
      case 3:
        queuePush(qh_dielectric, qd_dielectric, threadId.x);
        break;
      case 4:
        queuePush(qh_emissive, qd_emissive, threadId.x);
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

