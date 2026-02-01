module path;

import ray;
import hit;

public struct Path {
  public Ray ray; // 0..=32
  public float3 rad; // 33..=48
  public float3 throughput; // 49..=64
  public uint terminated;
  public uint sampled;
  public uint bounces;
  public uint sample_id;
};

public struct HitData {
  // For mat:
  // - 0 is meaningless
  // - 1 is a lambertian
  // - 2 is a metallic
  public Hit hit;
  public uint mat;
  public uint mat_data;
}

public struct ShadowData {
  public float3 dir;
  public float3 rad;
  public float prob;
}
