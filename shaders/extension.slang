module extension;

import path;
import ray;
import queue;
import hit;
import math;

struct AABB {
  float4 lb;
  float4 ub;
}

struct BVHNode {
  float4 lb;
  float4 ub;
  uint left;
  uint right;
  uint is_leaf;
  uint start;
  uint end;
}

struct Sphere {
  float3 position;
  float radius;

  void raySphereIntersect(Ray ray, out float t, out Hit h) {
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
    // Norms from the inside are very broken... must investigate
    // if (l2 < r2) {
    //   h.norm *= -1;
    // }
  }
}

struct Transform {
  float4 scale;
  float4 rotation;
  float4 translation;
}

struct Instance {
  Transform transform;
  uint mesh;
  uint material;
  uint material_data;
}

void rayTriIntersect(uint3 face, Ray ray, out float2 uv, out float t, out Hit h) {
  let p0 = tri_positions[face.x];
  let p1 = tri_positions[face.y];
  let p2 = tri_positions[face.z];

  let n0 = tri_normals[face.x];
  let n1 = tri_normals[face.y];
  let n2 = tri_normals[face.z];

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

  h.norm = normalize(cross(e1, e2));
  // h.norm = n0 * (1.0 - u - v) + n1 * u + n2 * v;
  h.pos = ray.pos + t * ray.dir;
  return;
}

bool rayBoxIntersect(Ray ray, float3 lb, float3 ub, float tmax) {
  var tmin = float.minValue;
  let dir_inv = 1.0 / ray.dir;

  for (int d = 0; d < 3; d++) {
    let sign = dir_inv[d] > 0;
    float bmin = select(sign, lb[d], ub[d]);
    float bmax = select(!sign, lb[d], ub[d]);

    float dmin = (bmin - ray.pos[d]) * dir_inv[d];
    float dmax = (bmax - ray.pos[d]) * dir_inv[d];

    tmin = max(dmin, tmin);
    tmax = min(dmax, tmax);
  }

  return tmin < tmax;
}

void rayBLASIntersect(Ray ray, uint bvh_node, float tmax, out float2 uv, out float t, out Hit h) {
  let root = bvh_node;
  var current = bvh_node;
  t = tmax;
  h = Hit(float3(0.0), float3(0.0));
  uv = float2(0.0);
  float t2;
  Hit h2;
  float2 uv2;
  do {
    let node = blas_nodes[current];

    let hit = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, t);
    current = select(!hit || node.is_leaf == 1, node.right, node.left);
    if (!hit || node.is_leaf == 0) {
      continue;
    }

    // TODO: Maybe we can batch the triangles somehow?
    for (int p = node.start; p < node.end; p++) {
      let face = tri_faces[p];
      rayTriIntersect(face, ray, uv2, t2, h2);

      if (t2 >= 0 && t2 < t) {
        t = t2;
        h = h2;
        uv = uv2;
      }
    }

  } while (root != current);
}

void rayTLASIntersect(Ray ray, inout float2 uv, inout float t, inout Hit h, inout uint mat, inout uint mat_data) {
  let root = 0;
  var current = 0;

  do {
    let node = tlas_nodes[current];

    let hit = rayBoxIntersect(ray, node.lb.xyz, node.ub.xyz, t);
    current = select(!hit || node.is_leaf == 1, node.right, node.left);
    if (!hit || node.is_leaf == 0) {
      continue;
    }

    for (int p = node.start; p < node.end; p++) {
      float t2;
      Hit h2;
      float2 uv2;

      Instance instance = instances[tlas_instance_ids[p]];
      uint bid = blas_roots[instance.mesh];

      float4x4 m = mul(translate(instance.transform.translation.xyz),
                   mul(rotate(instance.transform.rotation.xyz),
                       scale(instance.transform.scale.xyz)));
      float4x4 mi = mul(scale(1.0 / instance.transform.scale.xyz),
                    mul(transpose(rotate(instance.transform.rotation.xyz)),
                        translate(-instance.transform.translation.xyz)));

      Ray r = ray;
      r.dir = normalize(r.dir);
      r.pos = (mul(mi, float4(r.pos, 1.0))).xyz;
      r.dir = (mul(mi, float4(r.dir, 0.0))).xyz;

      rayBLASIntersect(r, bid, t, uv2, t2, h2);

      h2.pos  =           (mul(m, float4(h2.pos,  1.0))).xyz;
      h2.norm = normalize((mul(m, float4(h2.norm, 0.0))).xyz);

      // t2 = length(h2.pos - ray.pos) / length(ray.dir);

      if (t2 >= 0 && t2 < t) {
        mat = instance.material;
        mat_data = instance.material_data;
        uv = uv2;
        t = t2;
        h = h2;
      }
    }
  } while (root != current);
}

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(3,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,2)]] StructuredBuffer<Sphere> primitives;

[[vk::binding(0,3)]] StructuredBuffer<BVHNode> blas_nodes;
[[vk::binding(1,3)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,3)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,3)]] StructuredBuffer<float3> tri_normals;
[[vk::binding(4,3)]] StructuredBuffer<uint> blas_roots;

[[vk::binding(0,4)]] StructuredBuffer<Instance> instances;

[[vk::binding(0,5)]] StructuredBuffer<BVHNode> tlas_nodes;
[[vk::binding(1,5)]] StructuredBuffer<uint> tlas_instance_ids;
[[vk::binding(2,5)]] StructuredBuffer<AABB> tlas_aabbs;

[shader("compute")]
[numthreads(1,1,1)]
void extensionReset(uint3 threadId : SV_DispatchThreadID) {
  if (threadId.x == threadId.y == threadId.z == 0) {
    queueReset(qh_extension);
    return;
  }
}

[shader("compute")]
[numthreads(256,1,1)]
void extensionMain(uint3 threadId : SV_DispatchThreadID) {
  uint index;
  if (queueRead(qh_extension, qd_extension, index) == -1) {
    return;
  }

  Path path = paths[index];

  float t = float.maxValue;
  Hit h;

  // Sphere intersects, temporary TODO: replace with poly based spheres or something
  for (uint pid = 0; pid < primitives.getCount(); pid++) {
    float t2;
    Hit h2;
    Sphere s = primitives[pid];
    s.raySphereIntersect(path.ray, t2, h2);

    if (t2 >= 0 && t2 < t) {
      hits[index].mat = 1;
      hits[index].mat_data = 0;
      t = t2;
      h = h2;
    }
  }
 
  // BLAS Intersects
  float2 uv;
  rayTLASIntersect(
    path.ray,
    uv,
    t,
    h,
    hits[index].mat,
    hits[index].mat_data,
  );

  if (t < float.maxValue) {
    hits[index].hit = h;
    paths[index].bounces -= 1;
    paths[index].terminated = paths[index].terminated | paths[index].bounces == 0;
  } else {
    paths[index].rad += paths[index].throughput * float3(1.0);
    paths[index].terminated = 1;
  }

}
