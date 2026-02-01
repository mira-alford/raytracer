module scene_bindings;

import bvh;
import math;

public struct Instance {
  public uint transform;
  public uint geometry;
  public uint material;
}

public struct Transform {
  public float4 scale;
  public float4 rotation;
  public float4 translation;

  public float4x4 matrix() {
    return mul(
      translate_matrix(translation.xyz),
      mul(
        rotate_matrix(rotation.xyz),
        scale_matrix(scale.xyz)
      )
    );
  }  

  public float4x4 matrix_inverse() {
    return mul(
      scale_matrix(1.0 / scale.xyz),
      mul(
        transpose(rotate_matrix(rotation.xyz)),
        translate_matrix(-translation.xyz)
      )
    );
  }  
}

public struct VertexData {
  public float4 position;
  public float4 normal;
  public float4 uv;
};

public struct GeometryOffsets {
  public uint vertex;
  public uint index;
  public uint blas_node;
}

public struct Material {
    public uint colour_texture;             // 0 -> use base colour
    public uint emissive_texture;           // 0 -> use base emissive
    public uint metallic_roughness_texture; // 0 -> use base metallic/roughness
    public uint normal_texture;             // 0 -> use mesh vertex normals
    public float4 colour;                   // 0.0..=1.0 rgba
    public float4 emissive;                 // 0.0..=1.0 rgba
    public float metallic;                  // 0.0..=1.0
    public float roughness;                 // 0.0..=1.0
    public uint _pad0;
    public uint _pad1;
}

// Geometry (indexed by geometry id):
[[vk::binding(0,0)]] public StructuredBuffer<VertexData> vertices;
[[vk::binding(1,0)]] public StructuredBuffer<uint4> indices;
[[vk::binding(2,0)]] public StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(3,0)]] public StructuredBuffer<GeometryOffsets> geometry_offsets;

// Tlas (indexed by instance id):
[[vk::binding(4,0)]] public StructuredBuffer<BVHNode> tlas_nodes;

// Materials (indexed by material id):
[[vk::binding(5,0)]] public StructuredBuffer<Material> materials;

// Transforms (indexed by transform id):
[[vk::binding(6,0)]] public StructuredBuffer<Transform> transforms;

// Instances (indexed by instance id):
[[vk::binding(7,0)]] public StructuredBuffer<Instance> instances;

// Light Sources (indexed by intsance id):
[[vk::binding(8,0)]] public StructuredBuffer<BVHNode> light_sources;


