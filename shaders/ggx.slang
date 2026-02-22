module ggx;

import math;
import pathtracer;
import random;

// Based on code from https://jcgt.org/published/0007/04/01/paper.pdf
float3 GGXVNDFSampleNormal(float3 Ve, float alpha, int rng) {
  // transform Ve direction to hemisphere configuration
  float3 Vh = normalize(float3(alpha * Ve.x, alpha * Ve.y, Ve.z));

  // Orthonormal basis:
  float lensq = Vh.x * Vh.x + Vh.y * Vh.y;
  float3 T1 = lensq > 0 ? float3(-Vh.y, Vh.x, 0) * rsqrt(lensq) : float3(1,0,0);
  float3 T2 = cross(Vh, T1);

  // Parameterize projected area:
  float r = sqrt(random_gen(randoms, rng));
  float phi = 2.0 * float.getPi() * random_gen(randoms, rng);
  float t1 = r * cos(phi);
  float t2 = r * sin(phi);
  float s = 0.5 * (1.0 + Vh.z);
  t2 = (1.0 - s) * sqrt(1.0 - t1 * t1) + s * t2;

  // Reproject onto hemisphere:
  float3 Nh = t1 * T1 + t2 * T2 + sqrt(max(0.0, 1.0 - t1 * t1 - t2 * t2)) * Vh;

  // Transform normal back to ellipsoid configuration
  float3 Ne = normalize(float3(alpha * Nh.x, alpha * Nh.y, max(0.0, Nh.z)));
  return Ne;
}

float GGXVNDFDist(float3 N, float alpha) {
  N = normalize(N);
  float a2 = alpha * alpha;
  float n = N.x * N.x / a2 + N.y * N.y / a2 + N.z * N.z;
  float n2 = n * n;
  return 1.0 / (
    float.getPi() * a2 * n2
  );
}

float GGXVNDFSmithGeom(float3 V, float alpha) {
  float delta = -1.0;
  float a2 = alpha * alpha;
  delta += sqrt(1.0 + (a2 * V.x * V.x + a2 * V.y * V.y) / (V.z * V.z));
  delta /= 2.0;
  return 1.0 / (1.0 + delta);
}

public float GGXSmithGeom(float3 wi, float3 wo, float3 n, float alpha) {
  float3 Z = normalize(n);
  float lensq = Z.x * Z.x + Z.y * Z.y;
  float3 X = lensq > 0 ? float3(-Z.y, Z.x, 0) * rsqrt(lensq) : float3(1,0,0);
  float3 Y = cross(Z, X);

  float3 Ve = float3(dot(wo,X), dot(wo,Y), dot(wo,Z));

  return GGXVNDFSmithGeom(Ve, alpha);
}

float GGXVNDFPDF(float3 Ve, float3 Ne, float alpha) {
  float pdf = GGXVNDFSmithGeom(Ve, alpha) * max(0.0, dot(Ve, Ne)) * GGXVNDFDist(Ne, alpha);
  pdf /= Ve.z;
  return pdf;
}

public float GGXDist(float3 wi, float3 wo, float3 n, float alpha) {
  // Orthonormal basis;
  float3 Z = normalize(n);
  float lensq = Z.x * Z.x + Z.y * Z.y;
  float3 X = lensq > 0 ? float3(-Z.y, Z.x, 0) * rsqrt(lensq) : float3(1,0,0);
  float3 Y = cross(Z, X);

  float3 Ve = float3(dot(wo,X), dot(wo,Y), dot(wo,Z));
  float3 Le = float3(dot(wi,X), dot(wi,Y), dot(wi,Z));

  float3 Ne = normalize(Ve + Le);

  return GGXVNDFDist(Ne, alpha);
}

public float3 GGXSample(float3 wo, float3 n, float alpha, int rng) {
  // Orthonormal basis;
  float3 Z = normalize(n);
  float lensq = Z.x * Z.x + Z.y * Z.y;
  float3 X = lensq > 0 ? float3(-Z.y, Z.x, 0) * rsqrt(lensq) : float3(1,0,0);
  float3 Y = cross(Z, X);

  float3 Ve = float3(dot(wo,X), dot(wo,Y), dot(wo,Z));

  // Sample a new normal:
  float3 Ne = GGXVNDFSampleNormal(Ve, alpha, rng);

  // Reflect Ve about Ne:
  float3 Le = reflect(-Ve, Ne);

  // Transform Ve back to world coords:
  // return Ne.x * X + Ne.y * Y + Ne.z * Z;
  return normalize(Le.x * X + Le.y * Y + Le.z * Z);
}

public float GGXPDF(float3 wi, float3 wo, float3 n, float alpha) {
  if (dot(wo, n) <= 0 || dot(wi, n) <= 0) return 0.0;

  // Orthonormal basis:
  float3 Z = normalize(n);
  float lensq = Z.x * Z.x + Z.y * Z.y;
  float3 X = lensq > 0 ? float3(-Z.y, Z.x, 0) * rsqrt(lensq) : float3(1,0,0);
  float3 Y = cross(Z, X);

  // Transform wo, wi into Ve, Le again
  float3 Ve = float3(dot(wo,X), dot(wo,Y), dot(wo,Z));
  float3 Le = float3(dot(wi,X), dot(wi,Y), dot(wi,Z));

  // v = reflect(i,n) => n = normalize(i - v) * (dot(i,n) <= 0 ? 1 : -1)
  // We do Le = reflect(-Ve, Ne) in GGXSample
  // In reality this is probs equal to the half vector
  float3 Ne = normalize(-Ve - Le);
  if (dot(-Ve, Ne) > 0) Ne = -Ne;
   
  return GGXVNDFPDF(Ve, Ne, alpha) / (4 * dot(Ve, Ne));
}
