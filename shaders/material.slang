module material;

import path;
import queue;
import bvh;
import light;
import random;
import tlas;

public interface IMaterial {
  public float4 brdf(float3 wi, float3 wo, float3 n);
  public float pdf(float3 wi, float3 wo, float3 n);
  public bool scatter(out float3 wi, float3 wo, float3 n, inout uint4 random_state);
  public bool emit(out float4 colour);
}

public void process_material<T>(
  uint index,

  RWStructuredBuffer<Path> paths,
  RWStructuredBuffer<uint4> random_states,
  RWStructuredBuffer<HitData> hits,
  RWStructuredBuffer<ShadowData> shadow_data,

  RWStructuredBuffer<QueueHeader> qh_material,
  RWStructuredBuffer<uint> qd_material,

  RWStructuredBuffer<QueueHeader> qh_extension,
  RWStructuredBuffer<uint> qd_extension,

  StructuredBuffer<T> mat_datas,

  StructuredBuffer<uint> light_samples,

  StructuredBuffer<BVHNode> blas_nodes,
  StructuredBuffer<float3> tri_positions,
  StructuredBuffer<uint3> tri_faces,
  StructuredBuffer<float3> tri_normals,
  StructuredBuffer<uint> blas_roots,

  StructuredBuffer<BVHNode> tlas_nodes,
  StructuredBuffer<uint> tlas_instance_ids,
  StructuredBuffer<AABB> tlas_aabbs,

  StructuredBuffer<Instance> instances,
)
  where T: IMaterial
{
  let path = &paths[index];
  let hit = &hits[index];
  let mat_data = mat_datas[hit.mat_data];

  // Emission + Albedo:
  float4 emission;
  if (mat_data.emit(emission)) {
    path.rad += emission.xyz * path.throughput;
    path.terminated = 1;
    return;
  }
  
  float3 scatter;
  if (mat_data.scatter(
    scatter,
    -path.ray.dir,
    hit.hit.norm,
    random_states[index]
  )) {
  } else {
    // We don't scatter, indicating the ray can just stop here.
    path.terminated = 1;
  }

  if (path.terminated == 0) {
    ShadowData sd = shadow_data[index];
    float scatter_prob = mat_data.pdf(scatter, -path.ray.dir, hit.hit.norm);
  
    if (sd.prob > 0.0) {
      float shadow_weight = sd.prob;
      shadow_weight /= (sd.prob + mat_data.pdf(sd.dir, -path.ray.dir, hit.hit.norm));

      float3 contrib = path.throughput;
      contrib *= mat_data.brdf(sd.dir, -path.ray.dir, hit.hit.norm).xyz;
      contrib *= dot(sd.dir, hit.hit.norm);
      contrib *= shadow_weight;
      contrib /= sd.prob;
      path.rad += sd.rad * contrib;
    }
  
    if (scatter_prob > 0.0) {
      float scatter_weight = mat_data.pdf(scatter, -path.ray.dir, hit.hit.norm);
      scatter_weight /= (scatter_prob + 0.0);
      // FIXME: 0.0 should be prob of scatter in the light distribution...
      // Calculating this seems interesting...
      // Cast a ray
      // Get all lights hit on the ray
      // Sum the inverse of all the hit triangle areas * 1 / num tris per object
      // Then afterwards we can just multiply by num hit/num objects
      // This is going to be pretty close to and often just 0.0.... but its
      // probably still important and hella annoying to compute.
      // This doesnt seem to be the reason things are gaining energy though,
      // so maybe its not so urgent to fix?

      path.throughput *= mat_data.brdf(scatter, -path.ray.dir, hit.hit.norm).xyz;
      path.throughput *= dot(scatter, hit.hit.norm);
      path.throughput *= scatter_weight;
      path.throughput /= scatter_prob;
    } else {
      path.throughput *= 0.0;
      path.terminated = 1;
    }

    path.ray.pos = hit.hit.pos + hit.hit.norm * 1e-3;
    path.ray.dir = scatter;
  }

  // If not terminated, push for extension:
  if (path.terminated == 0) {
    queuePush(qh_extension, qd_extension, index);
  }
}
