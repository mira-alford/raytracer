// sample.slang
//
// Performs sample generation and completion.
// Takes complete samples off the complete_sample queue
// and accumulates their results into the overall sample.
// 
// If more samples are waiting to be spawned on termination,
// will place a new sample on the ray extension queue.
module sample;

import common;
import pathtracer;
import queue;
import random;

[[vk::binding(0,3)]] RWStructuredBuffer<uint> output;

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

void accumulateSample(uint idx, uint id) {
  var s = &samples[idx];
  var sample_count = 0;
  InterlockedAdd(sample_sources[s.sample_id].sample_count, 1, sample_count);
  
  s.rad.x = select(isnan(s.rad.x) || isinf(s.rad.x), 0.0, s.rad.x);
  s.rad.y = select(isnan(s.rad.y) || isinf(s.rad.y), 0.0, s.rad.y);
  s.rad.z = select(isnan(s.rad.z) || isinf(s.rad.z), 0.0, s.rad.z);

  // Multiply it by 1000 before adding into the sample sum buffer as we have no atomic floats :(
  sample_sum.InterlockedAdd(s.sample_id * sizeof(uint4) + 0 * sizeof(uint), uint(s.rad.x * 1000.0));
  sample_sum.InterlockedAdd(s.sample_id * sizeof(uint4) + 1 * sizeof(uint), uint(s.rad.y * 1000.0));
  sample_sum.InterlockedAdd(s.sample_id * sizeof(uint4) + 2 * sizeof(uint), uint(s.rad.z * 1000.0));

  let out_pos = sample_sources[s.sample_id].out_pos;
  let out_idx = out_pos.x + out_pos.y * dims.x;

  float3 rad = float3(sample_sum.Load3(s.sample_id * sizeof(uint4))) / float(1000 * sample_count);
  
  // rad *= exp2(-6.1); // TODO: compute this dynamically :)
  // rad = acesToneMap(rad);
  output[out_idx] = packRgb(rad);
}

void spawnSample(uint idx) {
  var s = &samples[idx];
  var ray = &extension_rays[idx];

  // Pull a sample to spawn:
  uint sample_idx;
  InterlockedAdd(sample_index[0], 1, sample_idx);
  sample_idx %= sample_sources.getCount();
  let sample_source = sample_sources[sample_idx];

  // Initialize sample:
  s.bounces = 0;
  s.rad = float3(0);
  s.sample_id = sample_idx;
  s.throughput = float3(1.0);

  // Initialize the ray:
  ray.pos = camera.position;

  // Generate from camera:
  let right = cross(camera.forward, camera.up);
  let top_left = camera.forward * camera.focal_length
                  + camera.up * camera.dims.y
                  - right * camera.dims.x;

  let d = float2(random_gen(randoms, idx), random_gen(randoms, idx)) / float2(dims.x, dims.y);
  let screen_pos = sample_source.screen_pos;
  let offset = -2.0 * camera.up * camera.dims.y * (screen_pos.y + d.y)
              + 2.0 * right * camera.dims.x * (screen_pos.x + d.x);

  float3 dir = top_left + offset;
  ray.dir = normalize(dir);

  // Queue it up for extension
  queuePush(extension_qh, extension_qd, idx);
}

[shader("compute")]
[numthreads(64,1,1)]
void sampleMain(uint3 threadId : SV_DispatchThreadID) {
  let idx = queueRead(terminate_qh, terminate_qd);
  if (idx < 0) {
    return;
  }

  // An active sample at idx has terminated.
  // Write its radiance to the output buffer:
  accumulateSample(idx, threadId.x);

  // Spawn a new sample
  // A good sampling algorithm (probs a linked list) is a
  // later problem, for now it just doesnt ever finish samples
  // which works but is a little bit of a perf drain.
  spawnSample(idx);
}

[shader("compute")]
[numthreads(64,1,1)]
void sampleCleanup(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x > sample_sources.getCount()) {
    return;
  }

  let wgc = WorkgroupCount().x;
  let wgs = WorkgroupSize().x;
  let work = sample_sources.getCount() / (wgc * wgs);

  for (int i = threadId.x * work; i < threadId.x * work + work; i++) {
    if (camera.changed != 0) {
      sample_sources[i].sample_count = 0;
      sample_sources[i].flags = 0;
      sample_sum.InterlockedExchange(i * sizeof(uint4) + 0 * sizeof(uint), 0);
      sample_sum.InterlockedExchange(i * sizeof(uint4) + 1 * sizeof(uint), 0);
      sample_sum.InterlockedExchange(i * sizeof(uint4) + 2 * sizeof(uint), 0);
      sample_sum.InterlockedExchange(i * sizeof(uint4) + 3 * sizeof(uint), 0);
    }
  }

}
