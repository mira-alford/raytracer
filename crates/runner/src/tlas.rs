use glam::UVec3;
use glam::Vec3;
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::bvh::AABB;
use crate::bvh::BVH;
use crate::bvh::BVHNode;
use crate::bvh::BVHNodeGPU;
use crate::instance::Transform;
use crate::mesh::Mesh;

#[derive(Debug)]
pub struct TLAS {
    pub nodes: Vec<BVHNode>,
    pub blas: Vec<usize>,
    pub blas_aabb: Vec<AABB>,
    pub blas_transform: Vec<Transform>,
}
