module state_bindings;

import bvh;
import math;
import queue;

public struct Hit {
  public float3 pos;
  public float3 norm;
  public uint3 prim;
  public uint front_face;
}

public struct Ray {
  public float3 pos;
  public float3 dir;
}

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
  public Hit hit;
  public uint mat;
}

public struct ShadowData {
  public float3 dir;
  public float3 rad;
  public float prob;
}

[Flags]
public enum SampleFlag : uint {
  Converged, // = 1
};

public struct SampleData {
  public float2 screen_pos; // Screen position in 0.0..=1.0
  public uint2 out_pos; // Screen position in pixels
  public uint samples; // Number of samples taken
  public uint flags; 
};

public struct Camera {
  public float3 position;
  public float3 forward;
  public float3 up;
  public float2 dims;
  public float focal_length;
  public uint changed;
}

// Path state:
[[vk::binding(1,0)]] public StructuredBuffer<Path> paths;

// Random state:
[[vk::binding(1,1)]] public StructuredBuffer<uint4> randoms;

// Hit and shadow result data:
[[vk::binding(1,2)]] public StructuredBuffer<ShadowData> shadow_data;
[[vk::binding(1,3)]] public StructuredBuffer<HitData> _;

// Sampling buffers:
[[vk::binding(1,4)]] public StructuredBuffer<uint> sample_index;
[[vk::binding(1,5)]] public StructuredBuffer<SampleData> sample_data;
[[vk::binding(1,6)]] public RWByteAddressBuffer sample_mean;
[[vk::binding(1,7)]] public RWStructuredBuffer<float4> sample_std;

// Dimensions/Camera constants::
[[vk::binding(1,8)]] ConstantBuffer<uint2> dims;

// Queues:
[[vk::binding(1,9)]] public StructuredBuffer<QueueHeader> qh_extension;
[[vk::binding(1,10)]] public StructuredBuffer<uint> qd_extension;
[[vk::binding(1,11)]] public StructuredBuffer<QueueHeader> qh_new_ray;
[[vk::binding(1,12)]] public StructuredBuffer<uint> qd_new_ray;
[[vk::binding(1,13)]] public StructuredBuffer<QueueHeader> qh_shadow;
[[vk::binding(1,14)]] public StructuredBuffer<uint> qd_shadow;
[[vk::binding(1,15)]] public StructuredBuffer<QueueHeader> qh_material;
[[vk::binding(1,16)]] public StructuredBuffer<uint> qd_material;
