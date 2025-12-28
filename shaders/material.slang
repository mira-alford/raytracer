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
    path.rad = emission.xyz * path.throughput;
    path.terminated = 1;
    return;
  }

  // Front face calculation + backface norm flipping:
  let front_face = dot(hit.hit.norm, path.ray.dir) < 0;
  let ffs = front_face ? 1 : -1;
  hit.hit.norm *= ffs;
  
  float3 scatter;
  float weight = 1.0;
  const float light_prob = 0.05;

  // Direct Light Sampling occurs with light_prob probability:
  if (!sample_light(random_states[index], light_prob)) {
    // No light sample, do scattering as usual:
    if (mat_data.scatter(
      scatter,
      -path.ray.dir,
      hit.hit.norm,
      random_states[index]
    )) {
      scatter = normalize(-scatter);

      // I omit weight = weight * brdf / brdf
      // as sampled = actual, so it would always equal 1.0
      weight /= max(1.0 - max(light_prob,0.0), 1e-8);
    } else {
      // We don't scatter, indicating the ray can just stop here.
      path.terminated = 1;
    }
  } else {
    // Light sample, start by sampling a random point on a light:
    float3 sample_pos;
    float sample_prob;
    get_light_sample(
      random_states[index],
      sample_pos,
      sample_prob,
      light_samples,
      blas_roots,
      blas_nodes,
      tlas_nodes,
      tri_positions,
      instances,
    );

    if (sample_prob <= 0.0 || path.bounces < 2) {
      // Impossible sample, if we dont reject it we add infinity to the colour
      // which is bad, obviously:
      path.terminated = 1;
    } else {
      // Then we "scatter" towards that sampled point:
      scatter = normalize(sample_pos - hit.hit.pos);

      // Otherwise, multiply weight by (actual_prob / sample_prob)
      // to do proper importance sampling:
      weight /= max(light_prob * sample_prob, 1e-8);
      weight *= mat_data.pdf(-scatter, -path.ray.dir, hit.hit.norm);
      path.bounces = 2;
    }
  }

  path.throughput *= mat_data.brdf(-scatter, -path.ray.dir, hit.hit.norm).xyz;
  path.throughput *= weight;

  // if (weight <= 1e-8) {
  //   path.rad = float3(0.0,0.0,0.0);
  //   path.terminated = 1;
  // }

  // Update path position to match hit with slight epsilon
  // to avoid self intersects:
  // if (dot(scatter, ffs * hit.hit.norm) < 0.0) {
  //   // Scattered *into* the surface, so shift against norm
  //   path.ray.pos = hit.hit.pos - hit.hit.norm * 1e-3;
  // } else {
    // Scattered *off* the surface, so shift with norm
    // path.ray.pos = hit.hit.pos + hit.hit.norm * 1e-3;
  // }
  path.ray.dir = scatter;
  path.ray.pos = hit.hit.pos;
  
  // If not terminated, push for extension:
  if (path.terminated == 0) {
    queuePush(qh_extension, qd_extension, index);
  }
}
