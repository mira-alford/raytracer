use glam::UVec3;
use glam::Vec3;
use glam::Vec4Swizzles;
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::blas::BLAS;
use crate::blas::BLASData;
use crate::bvh::AABB;
use crate::bvh::BVH;
use crate::bvh::BVHNode;
use crate::bvh::BVHNodeGPU;
use crate::instance::Instance;
use crate::instance::Transform;
use crate::mesh::Mesh;

#[derive(Debug)]
pub struct TLAS {
    pub nodes: Vec<BVHNode>,
    pub blas: Vec<usize>, // Which blas this points at
    pub aabbs: Vec<AABB>, // Its AABB (transformed)
}

impl BVH for TLAS {
    fn elem_bounds(&self, elem: usize) -> AABB {
        self.aabbs[elem]
    }

    fn elem_centroid(&self, elem: usize) -> Vec3 {
        (self.aabbs[elem].lb + self.aabbs[elem].ub) / 2.0
    }

    fn elem_swap(&mut self, elem: usize, elem2: usize) {
        self.aabbs.swap(elem, elem2);
        self.blas.swap(elem, elem2);
    }

    fn node(&self, idx: usize) -> &BVHNode {
        &self.nodes[idx]
    }

    fn push_node(&mut self, node: BVHNode) -> usize {
        let i = self.nodes.len();
        self.nodes.push(node);
        i
    }

    fn node_mut(&mut self, idx: usize) -> &mut BVHNode {
        &mut self.nodes[idx]
    }

    fn node_bounds(&self, idx: usize) -> AABB {
        self.nodes[idx].bounds
    }
}

impl TLAS {
    pub fn new(blases: &Vec<BLAS>, instances: &Vec<Instance>) -> Self {
        let aabbs = instances
            .iter()
            .map(|i| {
                let aabb = blases[i.mesh as usize].node_bounds(0);
                let translate = glam::Mat4::from_translation(i.transform.translation);
                let rotate = glam::Mat4::from_rotation_x(i.transform.rotation.x).mul_mat4(
                    &glam::Mat4::from_rotation_y(i.transform.rotation.y)
                        .mul_mat4(&glam::Mat4::from_rotation_z(i.transform.rotation.z)),
                );
                let scale = glam::Mat4::from_scale(i.transform.scale);
                let m = translate.mul_mat4(&rotate.mul_mat4(&scale));
                let aabb = AABB {
                    lb: m.mul_vec4(aabb.lb.extend(0.0)).xyz(),
                    ub: m.mul_vec4(aabb.ub.extend(0.0)).xyz(),
                };
                let aabb = AABB {
                    lb: aabb.lb.min(aabb.ub),
                    ub: aabb.lb.max(aabb.ub),
                };
                aabb
            })
            .collect_vec();

        let mut bvh = TLAS {
            nodes: vec![BVHNode {
                is_leaf: true,
                bounds: AABB::default(),
                start: 0,
                end: instances.len(),
                ..Default::default()
            }],
            blas: instances.iter().map(|i| i.mesh as usize).collect(),
            aabbs,
        };

        bvh.initialize();

        bvh
    }
}
