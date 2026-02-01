module tlas;

import math;

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

public struct Instance {
  public Transform transform;
  public uint mesh;
  public uint material;
  public uint material_data;
}
