module path;

import ray;

public struct Path {
  public Ray ray;
  public float3 rad;
  public float2 screen_pos;
  public uint32_t terminated;
  public uint32_t generated;
};
