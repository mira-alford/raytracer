module shadow;

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
import emissive;
import material;

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(1,0)]] RWStructuredBuffer<uint4> random_states;
[[vk::binding(2,0)]] RWStructuredBuffer<HitData> hits;
[[vk::binding(3,0)]] RWStructuredBuffer<ShadowData> shadow_data;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_shadow;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_shadow;

[[vk::binding(0,2)]] StructuredBuffer<uint> light_samples;

[[vk::binding(0,3)]] StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(1,3)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,3)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,3)]] StructuredBuffer<float3> tri_normals;
[[vk::binding(4,3)]] StructuredBuffer<uint> blas_roots;

[[vk::binding(0,4)]] StructuredBuffer<Instance> instances;

[[vk::binding(0,5)]] StructuredBuffer<BVHNode> tlas_nodes;
[[vk::binding(1,5)]] StructuredBuffer<uint> tlas_instance_ids;
[[vk::binding(2,5)]] StructuredBuffer<AABB> tlas_aabbs;

[[vk::binding(0,6)]] StructuredBuffer<EmissiveData> em_data;

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
void shadowReset(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_shadow);
    return;
  }
}

[shader("compute")]
[numthreads(256,1,1)]
void shadowMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_shadow, qd_shadow, index) == -1) {
    return;
  }

  HitData hit = hits[index];
  float3 orig = hit.hit.pos + hit.hit.norm * 1e-3;

  // Generate the shadow ray
  float3 sample_pos;
  float sample_prob;
  uint instance_id = -1;
  float3 sample_norm;

  get_light_sample(
    random_states[index],
    sample_pos,
    sample_prob,
    instance_id,
    sample_norm,
    light_samples,
    blas_roots,
    blas_nodes,
    tlas_nodes,
    tri_positions,
    instances,
  );

  float3 scatter = normalize(sample_pos - orig);
  float shadow_t = length(sample_pos - orig);
  Ray shadow_ray = Ray(orig, scatter);

  ShadowData sd = ShadowData(float3(0),float3(0),0);

  if (dot(shadow_ray.dir, hit.hit.norm) > 0) {
    float t = shadow_t;
    Hit shadow_hit;

    shadow_hit.prim = uint3(-1);
    let prim = hit.hit.prim;
 
    // BLAS Intersects
    float2 uv;
    uint a,b; // lmao this was breaking everything...
    rayTLASIntersect(
      rb,
      shadow_ray,
      prim,
      uv,
      t,
      shadow_hit,
      a,
      b
    );

    if (length(shadow_hit.pos - sample_pos) < 1e-0) {
      Instance instance = instances[instance_id];
      EmissiveData mat_data = em_data[instance.material_data];
      float4 colour;
      mat_data.emit(colour);
      sd.rad = colour.xyz;
      sd.prob = sample_prob;

      // must convert to solid angle
      float3 omega = float3(shadow_ray.dir);
      float3 r = shadow_hit.pos - orig;
      sd.prob = sd.prob * dot(r,r) / abs(dot(shadow_hit.norm, -omega));

      sd.dir = scatter;
    }
  }

  shadow_data[index] = sd;
}
