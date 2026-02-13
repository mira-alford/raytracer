// common.slang
//
// All common structures and functions for pathtracer.
module common;

// Material, follows pbr roughness-metallic model
public struct Material {
    public uint colour_texture;             // 0 -> use base colour
    public uint emissive_texture;           // 0 -> use base emissive
    public uint metallic_roughness_texture; // 0 -> use base metallic/roughness
    public uint normal_texture;             // 0 -> use mesh vertex normals
    public float4 colour;                   // 0.0..=1.0 rgba
    public float4 emissive;                 // 0.0..=1.0 rgba
    public float metallic;                  // 0.0..=1.0
    public float roughness;                 // 0.0..=1.0
    public float ior;
    public float transmission;              // 0.0..=1.0
}

public struct MaterialSample {
  public float4 colour;
  public float4 emissive;
  public float metallic;
  public float roughness;
  public float ior;
  public float transmission;
}

public struct Vertex {
  public float4 position;
  public float4 normal;
  public float4 uv;
};

public struct Triangle {
  public Vertex v0;
  public Vertex v1;
  public Vertex v2;
}

public float4x4 scale_matrix(float3 factor) {
  return float4x4(
    float4(factor.x,0.0,0.0,0.0),
    float4(0.0,factor.y,0.0,0.0),
    float4(0.0,0.0,factor.z,0.0),
    float4(0.0,0.0,0.0,     1.0)
  );
}

public float4x4 rotate_matrix(float3 factor) {
  let x = factor.x;
  let y = factor.y;
  let z = factor.z;
  return mul(
    float4x4(
      float4(1.0,     0.0,     0.0,     0.0),
      float4(0.0,     cos(x),  -sin(x), 0.0),
      float4(0.0,     sin(x),  cos(x),  0.0),
      float4(0.0,     0.0,     0.0,     1.0)
    ),
    mul(
      float4x4(
        float4(cos(y),  0.0,     sin(y),  0.0),
        float4(0.0,     1.0,     0.0,     0.0),
        float4(-sin(y), 0.0,     cos(y),  0.0),
        float4(0.0,     0.0,     0.0,     1.0)
      ),
      float4x4(
        float4(cos(z),  -sin(z), 0.0,     0.0),
        float4(sin(z),  cos(z),  0.0,     0.0),
        float4(0.0   ,  0.0,     1.0,     0.0),
        float4(0.0   ,  0.0,     0.0,     1.0)
      )
    )
  );
}

public float4x4 translate_matrix(float3 factor) {
  return float4x4(
    float4(1.0,0.0,0.0,factor.x),
    float4(0.0,1.0,0.0,factor.y),
    float4(0.0,0.0,1.0,factor.z),
    float4(0.0,0.0,0.0,1.0)
  );
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

// Ongoing sample data, contains accumulated sample information
// and flags for sample control.
public struct Sample {
  public float3 rad;
  public float3 throughput;
  public uint bounces;
  public uint sample_id;
};

// A ray has a position and direction.
public struct Ray {
  public float3 pos;
  public float3 dir;
}

// Records the result of a hit.
// instance + triangle id are enough to uniquely
// identify the hit triangle and get material information.
public struct HitRecord {
  // A fake vertex which stores the interpolated position, uv and norm
  // of the actually hit triangle.
  public Vertex vert;
  public uint triangle_id;
  public uint instance_id;
  public uint front_face;
}


[Flags]
public enum SampleFlag : uint {
  Converged, // = 1
};

public struct SampleSource {
  public float2 screen_pos; // Screen position in 0.0..=1.0
  public uint2 out_pos; // Screen position in pixels
  public uint sample_count; // Number of samples taken
  public uint flags; 
};

public struct Camera {
  public float3 position;
  public float3 forward;
  public float3 up;
  public float2 dims;
  public float focal_length;
  public uint changed;
}

// Instance, represents an object in the scene.
// Three indexes into transform, geometry and material tables
// corresponding to this instance.
public struct Instance {
  public uint transform;
  public uint geometry;
  public uint material;
}

public struct GeometryOffsets {
  public uint vertex;
  public uint index;
  public uint blas_node;
}

public uint packRgb(float3 color) {
    color = saturate(color.bgr);
    let r = uint(color.r * 255.0) << 16;
    let g = uint(color.g * 255.0) << 8;
    let b = uint(color.b * 255.0) << 0;
    return r | g | b;
}

