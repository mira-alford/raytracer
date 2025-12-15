module path;

import ray;
import hit;

public struct Path {
  public Ray ray; // 0..=32
  public float3 rad; // 33..=48
  public float3 throughput; // 49..=64
  public float2 screen_pos; // 
  public uint terminated;
  public uint generated;
  public uint bounces;
};

public struct HitData {
  // For mat:
  // - 0 is meaningless
  // - 1 is a lambertian
  public Hit hit;
  public uint mat;
  public uint mat_data;
}

public struct SampleState {
  public float3 sampled_rad;
  public float3 sampled_pos;
  public uint samples;
}
