// ray_extend.slang
//
// Performs ray extension for all samples waiting in the extension queue.
// Ray extension involves intersecting a ray with the *next*
// hit in the scene and constructing a hit record for use by material
// shaders.
module ray_extend;

import common;
import scene;
import pathtracer;
import random;
import queue;
import bvh;

[[vk::binding(0,3)]] RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(64,1,1)]
void extensionMain(uint3 threadId : SV_DispatchThreadID) {
  let idx = queueRead(extension_qh, extension_qd);
  if (idx < 0) {
    return;
  }

  var s = &samples[idx];
  var ray = &extension_rays[idx];

  // trace the ray
  // Start a tlas intersect on the tlas root:

  // If hit, rad = 1 else 0 for now
  float t = float.maxValue;
  HitRecord h;
 
  s.rad = float3(0.0);
  bool magic = tlasFirstHit(*ray, t, h);
  if (magic) {
    Instance instance = instances[h.instance_id];
    let mat = materials[instance.material];
    s.rad = dot(-h.vert.normal.xyz, ray.dir) * mat.colour.xyz;
  }

  queuePush(terminate_qh, terminate_qd, idx);
}

bool rayTriIntersect(Ray ray, Triangle tri, inout float t, inout HitRecord h) {
  let p0 = tri.v0.position.xyz;
  let p1 = tri.v1.position.xyz;
  let p2 = tri.v2.position.xyz;

  let n0 = tri.v0.normal.xyz;
  let n1 = tri.v1.normal.xyz;
  let n2 = tri.v2.normal.xyz;
  
  let e1 = p1 - p0;
  let e2 = p2 - p0;
  let q = cross(ray.dir, e2);
  let alpha = dot(e1, q);
  if (alpha > -(10e-8) && alpha < 10e-8) {
    return false;
  }
  let f = 1.0 / alpha;
  let s = ray.pos - p0;
  let u = f * dot(s, q);
  if (u < 0.0) {
    return false;
  }
  let r = cross(s, e1);
  let v = f * dot(ray.dir, r);
  if (v < 0.0 || u + v > 1.0) {
    return false;
  }

  let t2 = f * dot(e2, r);
  if (t2 > t || t2 < 0.0) {
    return false;
  }

  t = t2;
  h.vert.uv = float4(u,v,0.0,0.0);
  h.vert.normal = float4(n0 * (1.0 - u - v) + n1 * u + n2 * v, 0.0);
  h.vert.position = float4(p0 + e1 * u + e2 * v, 1.0);

  return true;
}

bool rayBoxIntersect(Ray ray, float3 lb, float3 ub, out float tmin, inout float tmax) {
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

bool blasFirstHit(
  const Ray ray,
  const uint instance_id,
  inout float t,
  inout HitRecord h
) {
  let instance = instances[instance_id];
  let geometry_offset = geometry_offsets[instance.geometry];
  let root = 0;
  var current = 0;
  var success = false;

  do {
    let node = blas_nodes[current + geometry_offset.blas_node];

    float tmax_aabb = t;
    float tmin_aabb;
    let hit_aabb = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, tmin_aabb, tmax_aabb);

    // If we hit, progress left
    current = select(hit_aabb, node.left, node.right);
    // If it's a leaf, always go right
    current = select(node.is_leaf == 1, node.right, current);

    if (!hit_aabb || node.is_leaf == 0) {
      continue;
    }

    // Iterate the primitives
    for (int p = node.start; p < node.end; p++) {
      uint3 face = indices[p + geometry_offset.index].xyz + geometry_offset.vertex;
      Triangle tri = Triangle(vertices[face.x], vertices[face.y], vertices[face.z]);
      float t2 = t;
      HitRecord h2;
      if (rayTriIntersect(ray, tri, t2, h2)) {
        h2.triangle_id = p;
        t = t2;
        h = h2;
        success = true;
      }
    }
  } while (current != root);
  return success;
}

bool tlasFirstHit(
  const Ray ray,
  inout float t,
  inout HitRecord h
) {
  let root = 0;
  var current = 0;
  var success = false;

  do {
    let node = tlas_nodes[current];

    float tmax_aabb = t;
    float tmin_aabb;
    let hit_aabb = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, tmin_aabb, tmax_aabb)
      && (tmax_aabb >= 0 || tmin_aabb >= 0);

    // If we hit, progress left
    current = select(hit_aabb, node.left, node.right);
    // If it's a leaf, always go right
    current = select(node.is_leaf == 1, node.right, current);

    if (!hit_aabb || node.is_leaf == 0) {
      continue;
    }

    for (int i = node.start; i < node.end; i++) {
      Instance instance = instances[i];

      Transform transform = transforms[instance.transform];
      float4x4 m = transform.matrix();
      float4x4 mi = transform.matrix_inverse();

      Ray r;
      r.pos = mul(mi, float4(ray.pos, 1.0)).xyz;
      r.dir = mul(mi, float4(ray.dir, 0.0)).xyz;

      float t2 = t;
      HitRecord h2;
      if (blasFirstHit(r, i, t2, h2)) {
        h2.vert.position = mul(m, h2.vert.position);
        h2.vert.normal = normalize(mul(m, h2.vert.normal));
        h2.instance_id = i;
        h2.front_face = dot(h2.vert.normal.xyz, ray.dir) < 0;
        t = t2;
        h = h2;
        success = true;
      }
    }
  } while (current != root);
  return success;
}
