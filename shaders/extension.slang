module extension;

import path;
import ray;
import queue;
import hit;

struct BVHNode {
  float3 lb;
  float3 ub;
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
    t = -1;
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
  if (alpha > -(10e-5) && alpha < 10e-5) {
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

  // h.norm = normalize(cross(e1, e2));
  h.norm = n0 * (1.0 - u - v) + n1 * u + n2 * v;
  h.pos = ray.pos + t * ray.dir;
  return;
}

void rayBVHIntersect(Ray ray, uint bvh_node, out float2 uv, out float t, out Hit h) {
  // It appears i need a ROPED bvh traversal
  // Definitely not getting this done tonight then.
  // Idea:
  // Each node points to the next node in a full dfs traversal
  // after all of its children are already done.
  // AKA each node has the index of its right sibling,
  // if that right sibling doesnt exist, keep going up until something has one
  // 0 is nullptr, NOT root node :)
}

[[vk::binding(0,0)]] RWStructuredBuffer<Path> paths;
[[vk::binding(3,0)]] RWStructuredBuffer<HitData> hits;

[[vk::binding(0,1)]] RWStructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,1)]] RWStructuredBuffer<uint> qd_extension;

[[vk::binding(0,2)]] StructuredBuffer<Sphere> primitives;

[[vk::binding(0,3)]] StructuredBuffer<BVHNode> bvh_nodes;
[[vk::binding(1,3)]] StructuredBuffer<float3> tri_positions;
[[vk::binding(2,3)]] StructuredBuffer<uint3> tri_faces;
[[vk::binding(3,3)]] StructuredBuffer<float3> tri_normals;

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
  for (uint pid = 0; pid < primitives.getCount(); pid++) {
    float t2;
    Hit h2;
    Sphere s = primitives[pid];
    s.raySphereIntersect(path.ray, t2, h2);

    if (t2 >= 0 && t2 <= t) {
      hits[index].mat = 1;
      t = t2;
      h = h2;
    }
  }
  for (uint pid = 0; pid < tri_faces.getCount(); pid++) {
    float t2;
    Hit h2;
    float2 uv;
    rayTriIntersect(tri_faces[pid], path.ray, uv, t2, h2);

    if (t2 >= 0 && t2 <= t) {
      hits[index].mat = 2;
      t = t2;
      h = h2;
    }
  }


  if (t < float.maxValue) {
    // paths[index].rad += float3(dot(h.norm,float3(0.0,0.0,-1.0))) / (5-path.bounces);
    hits[index].hit = h;
    paths[index].bounces -= 1;
    paths[index].terminated = paths[index].terminated | paths[index].bounces == 0;
  } else {
    paths[index].rad += paths[index].throughput * float3(1.0);
    paths[index].terminated = 1;
    // paths[index].rad = float3(0.0);
    // paths[index].terminated = 1;
  }

}
