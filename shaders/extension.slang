module extension;

import path;
import ray;
import queue;

struct Sphere {
    float3 position;
    float radius;
}

void raySphereIntersect(Sphere sphere, Ray ray, out float t, out float3 p) {
  t = -1;
  p = float3(-1);
  let l = sphere.position - ray.pos;
  let s = dot(l, ray.dir);
  let l2 = dot(l, l);
  let r2 = sphere.radius * sphere.radius;
  if (s < 0 && l2 > r2) {
    return;
  }
  let m2 = l2 - s*s;
  if (m2 > r2) {
    return;
  }
  let q = sqrt(r2 - m2);
  if (l2 > r2) {
    t = s - q;
  } else {
    t = s + q;
  }
  p = ray.pos + t * ray.dir;
}

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,2)]] StructuredBuffer<Sphere> primitives;

[shader("compute")]
[numthreads(256,1,1)]
void newRayMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_extension, qd_extension, index) == -1) {
    return;
  }

  Path path = paths[index];

  // For now, just do a ray sphere intersect
  float t = float.maxValue;
  float3 p;
  for (uint pid = 0; pid < primitives.getCount(); pid++) {
    float t2;
    float3 p2;
    Sphere s = primitives[pid];
    raySphereIntersect(s, path.ray, t2, p2);

    if (t2 >= 0 && t2 <= t) {
      t = t2;
      p = p2;
    }
  }

  if (t < float.maxValue) {
    paths[index].rad = pow(float3(dot(normalize(p-float3(0.0,0.0,3.0)),float3(0.0,0.0,1.0))),2.0);
  }
}
