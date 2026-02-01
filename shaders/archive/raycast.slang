module raycast;

import path;
import ray;
import queue;
import hit;
import math;
import light;
import bvh;
import tlas;

public struct RaycastBuffers {
  // All buffers for BLAS traversal:
  public StructuredBuffer<BVHNode> blas_nodes;
  public StructuredBuffer<float3> tri_positions;
  public StructuredBuffer<uint3> tri_faces;
  public StructuredBuffer<float3> tri_normals;
  public StructuredBuffer<uint> blas_roots;

  // Instance buffer:
  public StructuredBuffer<Instance> instances;
  
  // All buffers for TLAS traversal:
  public StructuredBuffer<BVHNode> tlas_nodes;
  public StructuredBuffer<uint> tlas_instance_ids;
  public StructuredBuffer<AABB> tlas_aabbs;
}

public struct Sphere {
  public float3 position;
  public float radius;

  public void raySphereIntersect(Ray ray, out float t, out Hit h) {
    t = float.maxValue;
    h = Hit();
    h.pos = float3(-1);
    let l = position - ray.pos;
    let s = dot(l, ray.dir);
    let l2 = dot(l, l);
    let r2 = radius * radius;
    if (s < 0 && l2 > r2) {
      return;
    }
    let m2 = l2 - s*s;
    if (m2 > r2) {
      return;
    }
    let q = sqrt(r2 - m2);
    if (l2 > r2) {
      t = s - q;
    } else {
      t = s + q;
    }
    h.pos = ray.pos + t * ray.dir;
    h.norm = (h.pos - position) / radius;
  }
}

public void rayTriIntersect(const RaycastBuffers rb, uint3 face, Ray ray, out float2 uv, out float t, out Hit h) {
  let p0 = rb.tri_positions[face.x];
  let p1 = rb.tri_positions[face.y];
  let p2 = rb.tri_positions[face.z];

  let n0 = rb.tri_normals[face.x];
  let n1 = rb.tri_normals[face.y];
  let n2 = rb.tri_normals[face.z];

  uv = float2(0.0);
  t = -1;
  h = Hit();
  
  let e1 = p1 - p0;
  let e2 = p2 - p0;
  let q = cross(ray.dir, e2);
  let alpha = dot(e1, q);
  if (alpha > -(10e-8) && alpha < 10e-8) {
    return;
  }
  let f = 1.0 / alpha;
  let s = ray.pos - p0;
  let u = f * dot(s, q);
  if (u < 0.0) {
    return;
  }
  let r = cross(s, e1);
  let v = f * dot(ray.dir, r);
  if (v < 0.0 || u + v > 1.0) {
    return;
  }
  t = f * dot(e2, r);

  h.prim = face;
  uv = float2(u,v);
  h.norm = n0 * (1.0 - u - v) + n1 * u + n2 * v;
  h.pos = p0 + e1 * u + e2 * v;

  return;
}

public bool rayBoxIntersect(Ray ray, float3 lb, float3 ub, out float tmin, inout float tmax) {
  tmin = float.minValue;
  let dir_inv = 1.0 / ray.dir;

  for (int d = 0; d < 3; d++) {
    let sign = dir_inv[d] >= 0;
    float bmin = select(sign, lb[d], ub[d]);
    float bmax = select(!sign, lb[d], ub[d]);

    float dmin = (bmin - ray.pos[d]) * dir_inv[d];
    float dmax = (bmax - ray.pos[d]) * dir_inv[d];

    tmin = max(dmin, tmin);
    tmax = min(dmax, tmax);
  }

  return tmin <= tmax;
}

public void rayBLASIntersect(const RaycastBuffers rb, const Ray ray, uint3 prim, const uint bvh_node, in float tmax, out float2 uv, out float t, out Hit h) {
  let root = bvh_node;
  var current = bvh_node;
  t = tmax;
  h = Hit(float3(0.0), float3(0.0));
  uv = float2(0.0);
  float t2;
  Hit h2;
  float2 uv2;
  do {
    let node = rb.blas_nodes[current];

    float tmax = t;
    float tmin;
    let hit = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, tmin, tmax);

    // If we hit, progress left
    current = select(hit, node.left, node.right);
    // If it's a leaf, always go right
    current = select(node.is_leaf == 1, node.right, current);

    if (!hit || node.is_leaf == 0) {
      continue;
    }

    // TODO: Maybe we can batch the triangles somehow?
    for (int p = node.start; p < node.end; p++) {
      let face = rb.tri_faces[p];

      rayTriIntersect(rb, face, ray, uv2, t2, h2);

      if (t2 >= 0.0 && t2 < t && (!h2.prim.equals(prim) || true)) {
        t = t2;
        h = h2;
        uv = uv2;
      }
    }

  } while (root != current);
}

public void rayTLASIntersect(const RaycastBuffers rb, const Ray ray, uint3 prim, inout float2 uv, inout float t, inout Hit h, inout uint mat, inout uint mat_data) {
  let root = 0;
  var current = 0;

  do {
    let node = rb.tlas_nodes[current];

    float tmax = t;
    float tmin;
    let hit = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, tmin, tmax) && (tmax >= 0 || tmin >= 0);

    // If we hit, progress left
    current = select(hit, node.left, node.right);
    // If it's a leaf, always go right
    current = select(node.is_leaf == 1, node.right, current);

    if (!hit || node.is_leaf == 0) {
      continue;
    }

    for (int p = node.start; p < node.end; p++) {

      float t2 = float.maxValue;
      Hit h2;
      float2 uv2;

      Instance instance = rb.instances[rb.tlas_instance_ids[p]];
      uint bid = rb.blas_roots[instance.mesh];

      float4x4 m = instance.transform.matrix();
      float4x4 mi = instance.transform.matrix_inverse();

      Ray r = ray;
      r.pos = mul(mi, float4(r.pos, 1.0)).xyz;
      r.dir = mul(mi, float4(r.dir, 0.0)).xyz;

      rayBLASIntersect(rb, r, prim, bid, t, uv2, t2, h2);

      h2.pos  =           mul(m, float4(h2.pos,  1.0)).xyz;
      h2.norm = normalize(mul(m, float4(h2.norm, 0.0)).xyz);

      if (t2 >= 0 && t2 < t) {
        mat = instance.material;
        mat_data = instance.material_data;
        uv = uv2;
        t = t2;
        h = h2;
      }
    }
  } while (current != root);
}
