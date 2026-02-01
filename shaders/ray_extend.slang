// ray_extend.slang
//
// Performs ray extension for all samples waiting in the extension queue.
// Ray extension involves intersecting a ray with the *next*
// hit in the scene and constructing a hit record for use by material
// shaders.
module ray_extend;

import common;
import scene;
import pathtracer;
import random;
import queue;

[[vk::binding(0,3)]] RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(64,1,1)]
void extensionMain(uint3 threadId : SV_DispatchThreadID) {
  let idx = queueRead(extension_qh, extension_qd);
  if (idx < 0) {
    return;
  }

  var s = &samples[idx];
  var ray = &extension_rays[idx];
  s.rad = ray.dir;

  queuePush(terminate_qh, terminate_qd, idx);
}
