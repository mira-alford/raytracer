module extension;

import path;
import ray;
import queue;
import hit;
import math;
import random;
import light;
import bvh;
import tlas;
import raycast;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,2)]] RWStructuredBuffer<QueueHeader> qh_shadow;
[[vk::binding(1,2)]] RWStructuredBuffer<uint> qd_shadow;

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

  // path.terminated = 1;
  // path.rad = float3(t*100.0);
  // return;

  if (t < float.maxValue) {
    hits[index].hit = h;
    paths[index].bounces -= 1;
    paths[index].terminated = paths[index].terminated | paths[index].bounces == 0;
    if (paths[index].terminated == 0) {
      queuePush(qh_shadow, qd_shadow, index);
    }
  } else {
    // Skybox hit:
    float strength = (dot(path.ray.dir, float3(0,1,0)) + 1.0)/2.0;
    paths[index].rad += path.throughput * float3(30.0,0.0,0.0) * strength;
    paths[index].terminated = 1;
  }

}
