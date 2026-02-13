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
import colour;

[[vk::binding(0,3)]] RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(64,1,1)]
void extensionMain(uint3 threadId : SV_DispatchThreadID) {
  let idx = queueRead(extension_qh, extension_qd);
  if (idx < 0) {
    return;
  }

  let s = &samples[idx];
  let ray = &extension_rays[idx];
  let hit = &extension_hit_records[idx];

  float t = float.maxValue;
  HitRecord h;
 
  if (!tlasFirstHit(*ray, hit.instance_id, hit.triangle_id, t, h)) {
    // No hit, queue for skybox?
    s.rad += s.throughput * float3(10.0);
    queuePush(terminate_qh, terminate_qd, idx);
    return;
  }

  // Useful TLAS debug view:
  // else {
  //   float lerp = (float)(1+h.instance_id) / (float)tlas_nodes.getCount();
  //   s.rad = HSVtoRGB(float3(lerp, 0.5, 0.5));
  //   queuePush(terminate_qh, terminate_qd, idx);
  //   return;
  // }

  *hit = h;
  queuePush(shade_qh, shade_qd, idx);
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
  const uint last_inst,
  const uint last_prim,
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
      if (!(p == last_prim && instance_id == last_inst)) {
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
    }
  } while (current != root);
  return success;
}

bool tlasFirstHit(
  const Ray ray,
  const uint last_inst,
  const uint last_prim,
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
      Instance instance = instances[tlas_to_instances[i]];

      Transform transform = transforms[instance.transform];
      float4x4 m = transform.matrix();
      float4x4 mi = transform.matrix_inverse();

      Ray r;
      r.pos = mul(mi, float4(ray.pos, 1.0)).xyz;
      r.dir = mul(mi, float4(ray.dir, 0.0)).xyz;

      float t2 = t;
      HitRecord h2;
      if (blasFirstHit(r, tlas_to_instances[i], last_inst, last_prim, t2, h2)) {
        h2.vert.position = mul(m, h2.vert.position);
        h2.vert.normal = normalize(mul(m, h2.vert.normal));
        h2.front_face = dot(h2.vert.normal.xyz, ray.dir) < 0;
        h2.instance_id = tlas_to_instances[i];
        t = t2;
        h = h2;
        success = true;
      }
    }
  } while (current != root);
  return success;
}
