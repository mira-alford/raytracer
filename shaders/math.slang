module math;

public float schlick(float cos_theta, float f0) {
  return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

public float3 schlick3(float cos_theta, float3 f0) {
  return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

public float fresnel(float cos_theta, float ior) {
  float f0 = pow((1.0 - ior) / (1.0 + ior), 2.0);
  return schlick(cos_theta, f0);
};

// Lerps from a to b by m effectively
public float3 mix(float3 a, float3 b, float m) {
  return (1.0 - m) * a + m * b;
}

public float3 fresnelMix(float3 wi, float3 wo, float ior, float3 base, float3 layer) {
  float3 h = normalize(wo + wi); // half vector
  float fr = fresnel(abs(dot(wo, h)), ior);
  return mix(base, layer, fr);
}

public float heaviside(float x) {
  return select(x.x > 0.0, 1.0, 0.0);
}

