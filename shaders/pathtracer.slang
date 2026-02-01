// pathtracer.slang
//
// Contains bindings for an individual pathtracers state.
// This allows multiple pathtracers to be swapped in and out easily
// for example for multiple cameras.
//
// Pathtracer data goes into bind group 1.
// Camera data goes into bind group 2 as it gets updated lots.
// TODO Investigate if this seperate bg is necessary for perf given its only one call?
module pathtracer;
import common;

// Sample information:
[[vk::binding(0,1)]] public RWStructuredBuffer<Sample> samples;

// Ray extension and ray connection input buffers
[[vk::binding(1,1)]] public RWStructuredBuffer<Ray> extension_rays;
[[vk::binding(2,1)]] public RWStructuredBuffer<Ray> connect_rays;

// Hit buffers for extension and connect
[[vk::binding(3,1)]] public RWStructuredBuffer<HitRecord> extension_hit_records;
[[vk::binding(4,1)]] public RWStructuredBuffer<HitRecord> connect_hit_records;

// Random state:
[[vk::binding(5,1)]] public RWStructuredBuffer<uint4> randoms;

// Sampling buffers:
[[vk::binding(6,1)]] public RWStructuredBuffer<uint> sample_index;
[[vk::binding(7,1)]] public RWStructuredBuffer<SampleSource> sample_sources;
[[vk::binding(8,1)]] public RWByteAddressBuffer sample_sum;
[[vk::binding(9,1)]] public RWStructuredBuffer<float4> sample_std;

// Queues:
[[vk::binding(10,1)]] public RWStructuredBuffer<int> extension_qh;
[[vk::binding(11,1)]] public RWStructuredBuffer<uint> extension_qd;

[[vk::binding(12,1)]] public RWStructuredBuffer<int> connect_qh;
[[vk::binding(13,1)]] public RWStructuredBuffer<uint> connect_qd;

[[vk::binding(14,1)]] public RWStructuredBuffer<int> terminate_qh;
[[vk::binding(15,1)]] public RWStructuredBuffer<uint> terminate_qd;

[[vk::binding(16,1)]] public RWStructuredBuffer<int> shade_qh;
[[vk::binding(17,1)]] public RWStructuredBuffer<uint> shade_qd;

// Dimensions:
[[vk::binding(18,1)]] public ConstantBuffer<uint2> dims;

// Camera, all alone:
[[vk::binding(0,2)]] public ConstantBuffer<Camera> camera;
