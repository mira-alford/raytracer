use glam::{UVec3, Vec3};
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::mesh::{Mesh, Meshes};

#[derive(Default, Clone, Copy, Debug)]
pub struct AABB {
    pub lb: Vec3,
    pub ub: Vec3,
}

impl AABB {
    pub fn union(&self, other: &AABB) -> AABB {
        AABB {
            lb: self.lb.min(other.lb),
            ub: self.ub.max(other.ub),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BVHNode {
    pub bounds: AABB,
    pub left: usize,
    pub right: usize,
    pub skip: usize,
    pub is_leaf: bool,
    pub start: usize,
    pub end: usize,
}

impl BVHNode {
    fn bounds(&self) -> AABB {
        self.bounds
    }
}

pub trait BVH {
    fn elem_bounds(&self, elem: usize) -> AABB;

    fn elem_centroid(&self, elem: usize) -> Vec3;

    fn elem_swap(&mut self, elem: usize, elem2: usize);

    fn node(&self, idx: usize) -> &BVHNode;

    fn node_bounds(&self, idx: usize) -> AABB;

    fn push_node(&mut self, node: BVHNode) -> usize;

    fn node_mut(&mut self, idx: usize) -> &mut BVHNode;

    fn compute_node_bounds(&mut self, idx: usize) {
        let mut node = *self.node(idx);
        if !node.is_leaf {
            let l = *self.node(node.left);
            let r = *self.node(node.right);
            node.bounds = l.bounds().union(&r.bounds());
        } else {
            let mut new_bounds = self.elem_bounds(node.start);
            for i in node.start + 1..node.end {
                new_bounds = new_bounds.union(&self.elem_bounds(i))
            }
            node.bounds = new_bounds;
        }
        *self.node_mut(idx) = node;
    }

    fn subdivide(&mut self, idx: usize, threshold: usize) {
        let node = *self.node(idx);
        let node = if !node.is_leaf {
            self.subdivide(node.left, threshold);
            self.subdivide(node.right, threshold);
            return;
        } else {
            // Don't subdivide if the number of circles within threshold:
            if node.end - node.start <= threshold {
                return;
            }

            // Compute the longest axis, on which we will split
            let extent = node.bounds.ub - node.bounds.lb;
            let mut axis = 0;
            if extent.y > extent.x {
                axis = 1
            };
            if extent.z > extent[axis] {
                axis = 2
            };

            // Get the median circle
            let split = node.bounds.lb[axis] + extent[axis] / 2.0;
            let (mut i, mut j) = (node.start, node.end - 1);
            while i <= j {
                if self.elem_centroid(i)[axis] < split {
                    i += 1;
                } else {
                    self.elem_swap(i, j);
                    j -= 1;
                }
            }

            if i == node.end || i == node.start {
                // Either empty or one sided, so make no changes.
                // This is probably unreachable given i use the median
                // and a threshold, but here to be safe.
                return;
            }

            let left = BVHNode {
                is_leaf: true,
                bounds: Default::default(),
                start: node.start,
                end: i,
                ..Default::default()
            };
            let right = BVHNode {
                is_leaf: true,
                bounds: Default::default(),
                start: i,
                end: node.end,
                ..Default::default()
            };

            let l = self.push_node(left);
            let r = self.push_node(right);

            self.compute_node_bounds(l);
            self.compute_node_bounds(r);
            self.subdivide(l, threshold);
            self.subdivide(r, threshold);

            BVHNode {
                is_leaf: false,
                bounds: Default::default(),
                left: l,
                right: r,
                ..Default::default()
            }
        };

        *self.node_mut(idx) = node;
        self.compute_node_bounds(idx);
    }

    fn generate_skips(&mut self, idx: usize, next: usize) {
        let node = *self.node(idx);

        {
            let mut n = node;
            n.skip = next;
            *self.node_mut(idx) = n;
        }

        if !node.is_leaf {
            self.generate_skips(node.left, node.right);
            self.generate_skips(node.right, next);
        }
    }

    fn initialize(&mut self, threshold: usize) {
        self.compute_node_bounds(0);
        self.subdivide(0, threshold);
        self.generate_skips(0, 0);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct AABBGPU {
    pub lower_bound: [f32; 3], // last one is padding
    pub _pad0: u32,
    pub upper_bound: [f32; 3], // last one is padding
    pub _pad2: u32,
}

impl From<AABB> for AABBGPU {
    fn from(aabb: AABB) -> Self {
        AABBGPU {
            lower_bound: [aabb.lb.x, aabb.lb.y, aabb.lb.z],
            upper_bound: [aabb.ub.x, aabb.ub.y, aabb.ub.z],
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct BVHNodeGPU {
    pub aabb: AABBGPU,
    pub left: u32,        // Left child, (meaningless if 0 || is_leaf)
    pub right: u32,       // Right child, (meaningless if 0 || is_leaf)
    pub is_leaf: u32,     // Leaf node? start/end are meaningless if 0
    pub start: u32,       // Start face, inclusive
    pub end: u32,         // End face, not inclusive
    pub _pad_2: [u32; 3], // pad struct to 16
}

impl From<BVHNode> for BVHNodeGPU {
    fn from(value: BVHNode) -> Self {
        BVHNodeGPU {
            aabb: AABBGPU::from(value.bounds),
            left: value.left as u32,
            right: value.skip as u32,
            is_leaf: value.is_leaf as u32,
            start: value.start as u32,
            end: value.end as u32,
            ..Default::default()
        }
    }
}
