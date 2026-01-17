module light;

import random;
import bvh;
import tlas;
 
public bool sample_light(inout uint4 random_state, float prob) {
  RandomResult r = Random(random_state);
  random_state = r.state;
  return r.value < prob;
}

// Samples a random point on the given triangle
public float3 tri_sample(float3 p0, float3 p1, float3 p2, inout uint4 random_state) {
  float3 e1 = p1-p0;
  float3 e2 = p2-p0;

  RandomResult r1 = Random(random_state);
  RandomResult r2 = Random(r1.state);
  random_state = r2.state;

  return p0 + e1 * r1.value + e2 * r2.value;
}

// Probability of sampling point x on a triangle
public float tri_pdf(float3 p0, float3 p1, float3 p2, float3 x) {
  float3 e1 = p1-p0;
  float3 e2 = p2-p0;
  float area = 0.5 * length(cross(e1,e2));
  return (area > 0.0) ? (1.0 / area) : 1.0;
}

public bool get_light_sample(
  inout uint4 random_state,
  out float3 sample_pos,
  out float sample_prob,
  out uint instance,
  out float3 sample_norm,
  StructuredBuffer<uint> light_samples,
  StructuredBuffer<uint> blas_roots,
  StructuredBuffer<BVHNode> blas_nodes,
  StructuredBuffer<BVHNode> tlas_nodes,
  StructuredBuffer<float3> tri_positions,
  StructuredBuffer<Instance> instances,
) {
  sample_pos = float3(0.0);
  sample_prob = 0.0;

  RandomResult r1 = Random(random_state);
  RandomResult r2 = Random(r1.state);
  random_state = r2.state;

  uint light_sample = uint(floor(r1.value * light_samples.getCount()));
  light_sample = light_samples[light_sample];

  // if (light_sample == 0) {
  //   return false;
  // }
  
  Instance sample_instance = instances[light_sample];
  uint sample_blas = blas_roots[sample_instance.mesh];
  BVHNode node = blas_nodes[sample_blas];

  uint3 sample_face = uint(floor(r2.value * float(node.end-node.start))) + node.end;
  float3 p0 = tri_positions[sample_face.x];
  float3 p1 = tri_positions[sample_face.y];
  float3 p2 = tri_positions[sample_face.z];

  sample_pos = tri_sample(p0, p1, p2, random_state);

  // prob of this point on triangle * prob of this triangle:
  sample_prob = tri_pdf(p0, p1, p2, sample_pos)                        
    * (1.0 / float(light_samples.getCount() * (node.end - node.start)));

  // Transform sample pos based on instance transform:
  let m = sample_instance.transform.matrix();
  sample_pos = mul(m, float4(sample_pos,1.0)).xyz;
  sample_norm = cross(p1-p0, p2-p0);

  return true;
}
