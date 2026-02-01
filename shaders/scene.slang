// scene.slang
//
// Contains the bindings for scene state.
// Uses bind group 0.
module scene;
import common;
import bvh;

// Instances (indexed by instance id):
[[vk::binding(0,0)]] public StructuredBuffer<Instance> instances;

// Geometry (indexed by geometry id):
[[vk::binding(1,0)]] public StructuredBuffer<GeometryOffsets> geometry_offsets;
[[vk::binding(2,0)]] public StructuredBuffer<Vertex> vertices;
[[vk::binding(3,0)]] public StructuredBuffer<uint4> indices;
[[vk::binding(4,0)]] public StructuredBuffer<BVHNode> blas_nodes;

// Materials (indexed by material id):
[[vk::binding(5,0)]] public StructuredBuffer<Material> materials;

// Transforms (indexed by transform id):
[[vk::binding(6,0)]] public StructuredBuffer<Transform> transforms;

// Tlas (only index the root):
[[vk::binding(7,0)]] public StructuredBuffer<BVHNode> tlas_nodes;

// Light Sources (indexed by intsance id):
[[vk::binding(8,0)]] public StructuredBuffer<BVHNode> light_sources;
