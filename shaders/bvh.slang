// bvh.slang
//
// Contains all relevant structures and functionality for
// bvh traversal, including BLAS and TLAS.
module bvh;
import common;

public struct BVHNode {
  public float4 lb;
  public float4 ub;
  public uint left;
  public uint right;
  public uint is_leaf;
  public uint start;
  public uint end;
}

// public void rayTriIntersect(Triangle tri, Ray ray, out float2 uv, out float t, out Hit h) {
//   let p0 = tri.v0.position.xyz;
//   let p1 = tri.v1.position.xyz;
//   let p2 = tri.v2.position.xyz;

//   let n0 = tri.v0.normal.xyz;
//   let n1 = tri.v1.normal.xyz;
//   let n2 = tri.v2.normal.xyz;

//   uv = float2(0.0);
//   t = -1;
//   h = Hit();
  
//   let e1 = p1 - p0;
//   let e2 = p2 - p0;
//   let q = cross(ray.dir, e2);
//   let alpha = dot(e1, q);
//   if (alpha > -(10e-8) && alpha < 10e-8) {
//     return;
//   }
//   let f = 1.0 / alpha;
//   let s = ray.pos - p0;
//   let u = f * dot(s, q);
//   if (u < 0.0) {
//     return;
//   }
//   let r = cross(s, e1);
//   let v = f * dot(ray.dir, r);
//   if (v < 0.0 || u + v > 1.0) {
//     return;
//   }
//   t = f * dot(e2, r);

//   uv = float2(u,v);
//   h.norm = n0 * (1.0 - u - v) + n1 * u + n2 * v;
//   h.pos = p0 + e1 * u + e2 * v;

//   return;
// }
