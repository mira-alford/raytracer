module extension;

import path;
import ray;
import queue;
import hit;
import math;
import light;
import bvh;
import tlas;
import raycast;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,2)]] StructuredBuffer<Sphere> primitives;

[[vk::binding(0,3)]] StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(1,3)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,3)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,3)]] StructuredBuffer<float3> tri_normals;
[[vk::binding(4,3)]] StructuredBuffer<uint> blas_roots;

[[vk::binding(0,4)]] StructuredBuffer<Instance> instances;

[[vk::binding(0,5)]] StructuredBuffer<BVHNode> tlas_nodes;
[[vk::binding(1,5)]] StructuredBuffer<uint> tlas_instance_ids;
[[vk::binding(2,5)]] StructuredBuffer<AABB> tlas_aabbs;

const static RaycastBuffers rb = RaycastBuffers(
  blas_nodes,
  tri_positions,
  tri_faces,
  tri_normals,
  blas_roots,

  instances,

  tlas_nodes,
  tlas_instance_ids,
  tlas_aabbs,
);

[shader("compute")]
[numthreads(1,1,1)]
void extensionReset(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_extension);
    return;
  }
}

[shader("compute")]
[numthreads(256,1,1)]
void extensionMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_extension, qd_extension, index) == -1) {
    return;
  }

  Path path = paths[index];

  float t = float.maxValue;
  Hit h;

  // Sphere intersects, temporary TODO: replace with poly based spheres or something
  for (uint pid = 0; pid < primitives.getCount(); pid++) {
    float t2;
    Hit h2;
    Sphere s = primitives[pid];
    s.raySphereIntersect(path.ray, t2, h2);

    if (t2 >= 0 && t2 < t) {
      hits[index].mat = 1;
      hits[index].mat_data = 0;
      t = t2;
      h = h2;
    }
  }

  h.prim = uint3(-1);
  let prim = hits[index].hit.prim;

 
  // BLAS Intersects
  float2 uv;
  rayTLASIntersect(
    rb,
    path.ray,
    prim,
    uv,
    t,
    h,
    hits[index].mat,
    hits[index].mat_data,
  );

  let front_face = dot(h.norm, path.ray.dir) < 0;
  let ffs = front_face ? 1 : -1;
  h.front_face = front_face;
  h.norm *= ffs;

  if (t < float.maxValue) {
    hits[index].hit = h;
    paths[index].bounces -= 1;
    paths[index].terminated = paths[index].terminated | paths[index].bounces == 0;
    if (paths[index].terminated == 1) {
      paths[index].rad = float3(1.0,0.0,0.0);
    } else {
      // Queue up a shadow extension too yay
      
    }
  } else {
    // Skybox hit:
    paths[index].rad = paths[index].throughput * float3(0.0);
    paths[index].rad = float3(1.0,0.0,0.0);
    paths[index].terminated = 1;
  }

}
