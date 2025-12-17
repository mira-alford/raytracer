module math;

public float4x4 scale(float3 factor) {
  return float4x4(
    float4(factor.x,0.0,0.0,0.0),
    float4(0.0,factor.y,0.0,0.0),
    float4(0.0,0.0,factor.z,0.0),
    float4(0.0,0.0,0.0,     1.0)
  );
}

public float4x4 rotate(float3 factor) {
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

public float4x4 translate(float3 factor) {
  return float4x4(
    float4(1.0,0.0,0.0,factor.x),
    float4(0.0,1.0,0.0,factor.y),
    float4(0.0,0.0,1.0,factor.z),
    float4(0.0,0.0,0.0,1.0)
  );
}

