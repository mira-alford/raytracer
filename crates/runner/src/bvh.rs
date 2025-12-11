use glam::{UVec3, Vec3};
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::mesh::{Mesh, Meshes};

#[derive(Default, Clone, Copy, Debug)]
struct AABB {
    lb: Vec3,
    ub: Vec3,
}

impl AABB {
    fn union(&self, other: &AABB) -> AABB {
        AABB {
            lb: self.lb.min(other.lb),
            ub: self.ub.max(other.ub),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BVHNode {
    Internal {
        // Bounds of this BVH:
        bounds: AABB,
        // Children BVHNodes:
        left: usize,
        right: usize,
    },
    Leaf {
        // Bounds of this BVH:
        bounds: AABB,
        // Contained primitives
        start: usize,
        end: usize, // non inclusive
    },
}

impl BVHNode {
    fn bounds(&self) -> AABB {
        match self {
            BVHNode::Internal { bounds, .. } => *bounds,
            BVHNode::Leaf { bounds, .. } => *bounds,
        }
    }
}

#[derive(Debug)]
pub struct BVH {
    pub nodes: Vec<BVHNode>,
    pub tri_positions: Vec<Vec3>,
    pub tri_faces: Vec<UVec3>,
    pub tri_normals: Vec<Vec3>,
}

impl BVH {
    fn face_bounds(&self, face: usize) -> AABB {
        let face = self.tri_faces[face];
        let positions = face.to_array().map(|i| &self.tri_positions[i as usize]);
        let lb = positions[0].min(*positions[1]).min(*positions[2]);
        let ub = positions[0].max(*positions[1]).max(*positions[2]);
        AABB { lb, ub }
    }

    fn face_centroid(&self, face: usize) -> Vec3 {
        let face = self.tri_faces[face];
        let positions = face.to_array().map(|i| self.tri_positions[i as usize]);
        positions.into_iter().reduce(|acc, v| acc + v).unwrap() / 3.0
    }

    fn compute_bounds(&mut self, node_idx: usize) {
        let mut node = self.nodes[node_idx];
        match &mut node {
            BVHNode::Internal {
                bounds,
                left,
                right,
            } => {
                let l = &self.nodes[*left];
                let r = &self.nodes[*right];
                *bounds = l.bounds().union(&r.bounds());
            }
            BVHNode::Leaf { bounds, start, end } => {
                let mut new_bounds = self.face_bounds(*start);
                for i in *start + 1..*end {
                    new_bounds = new_bounds.union(&self.face_bounds(i))
                }
                *bounds = new_bounds;
            }
        };
        self.nodes[node_idx] = node;
    }

    fn subdivide(&mut self, node_idx: usize, threshold: usize) {
        let node = self.nodes[node_idx];
        let node = match node {
            BVHNode::Internal {
                bounds,
                left,
                right,
            } => {
                self.subdivide(left, threshold);
                self.subdivide(right, threshold);
                return;
            }
            BVHNode::Leaf { bounds, start, end } => {
                // Don't subdivide if the number of circles within threshold:
                if end - start <= threshold {
                    return;
                }

                // Compute the longest axis, on which we will split
                let extent = bounds.ub - bounds.lb;
                let mut axis = 0;
                if extent.y > extent.x {
                    axis = 1
                };
                if extent.z > extent[axis] {
                    axis = 2
                };

                // Get the median circle
                let split = bounds.lb[axis] + extent[axis] / 2.0;
                let (mut i, mut j) = (start, end - 1);
                while i <= j {
                    if self.face_centroid(i)[axis] < split {
                        i += 1;
                    } else {
                        self.tri_faces.swap(i, j);
                        j -= 1;
                    }
                }

                if i == end - 1 || i == start {
                    // Either empty or one sided, so make no changes.
                    // This is probably unreachable given i use the median
                    // and a threshold, but here to be safe.
                    return;
                }

                let left = BVHNode::Leaf {
                    bounds: Default::default(),
                    start: start,
                    end: i,
                };
                let right = BVHNode::Leaf {
                    bounds: Default::default(),
                    start: i,
                    end: end,
                };

                let l = self.nodes.len();
                self.nodes.push(left);
                let r = self.nodes.len();
                self.nodes.push(right);

                self.compute_bounds(l);
                self.compute_bounds(r);
                self.subdivide(l, threshold);
                self.subdivide(r, threshold);

                BVHNode::Internal {
                    bounds: Default::default(),
                    left: l,
                    right: r,
                }
            }
        };

        self.nodes[node_idx] = node;
        self.compute_bounds(node_idx);
    }

    pub fn new(mesh: Mesh) -> BVH {
        let mut bvh = BVH {
            nodes: vec![BVHNode::Leaf {
                bounds: AABB::default(),
                start: 0,
                end: mesh.faces.len(),
            }],
            tri_positions: mesh
                .positions
                .iter()
                .map(|p| Vec3::from_slice(&p[..3]))
                .collect(),
            tri_faces: mesh
                .faces
                .iter()
                .map(|p| UVec3::from_slice(&p[..3]))
                .collect(),
            tri_normals: mesh
                .normals
                .iter()
                .map(|p| Vec3::from_slice(&p[..3]))
                .collect(),
        };

        bvh.compute_bounds(0);

        // Subdivide until all BVHs have at most 4 elements
        bvh.subdivide(0, 32);

        bvh
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct BVHNodeGPU {
    pub lower_bound: [f32; 4], // last one is padding
    pub upper_bound: [f32; 4], // last one is padding
    pub left: u32,             // Left child, (meaningless if 0 || is_leaf)
    pub right: u32,            // Right child, (meaningless if 0 || is_leaf)
    pub is_leaf: u32,          // Leaf node? start/end are meaningless if 0
    pub start: u32,            // Start face, inclusive
    pub end: u32,              // End face, not inclusive
    pub _pad: [u32; 3],        // pad struct to 16
}

impl From<BVHNode> for BVHNodeGPU {
    fn from(value: BVHNode) -> Self {
        match value {
            BVHNode::Internal {
                bounds,
                left,
                right,
            } => BVHNodeGPU {
                lower_bound: [bounds.lb.x, bounds.lb.y, bounds.lb.z, 0.0],
                upper_bound: [bounds.ub.x, bounds.ub.y, bounds.ub.z, 0.0],
                left: left as u32,   // FIXME these should be offset for more bvhs
                right: right as u32, // But 0 should not be offset, its just null
                is_leaf: 0,
                start: 0,
                end: 0,
                ..Default::default()
            },
            BVHNode::Leaf { bounds, start, end } => BVHNodeGPU {
                lower_bound: [bounds.lb.x, bounds.lb.y, bounds.lb.z, 0.0],
                upper_bound: [bounds.ub.x, bounds.ub.y, bounds.ub.z, 0.0],
                left: 0,
                right: 0,
                is_leaf: 1,
                start: start as u32,
                end: end as u32,
                ..Default::default()
            },
        }
    }
}

pub struct BLAS {
    pub nodes: Vec<BVHNodeGPU>,
    pub tri_positions: Vec<[f32; 4]>,
    pub tri_normals: Vec<[f32; 4]>,
    pub tri_faces: Vec<[u32; 4]>,
    pub bindgroup: wgpu::BindGroup,
    pub bindgroup_layout: wgpu::BindGroupLayout,
}

impl BLAS {
    pub fn new(device: &wgpu::Device, bvhs: Vec<BVH>) -> Self {
        // Merge the meshes
        let mut nodes = Vec::new();
        let mut tri_positions = Vec::new();
        let mut tri_normals = Vec::new();
        let mut tri_faces = Vec::new();
        let mut face_offset = 0;
        let mut vertex_offset = 0;

        for mut bvh in bvhs {
            // TODO: Can be optimised using a hashmap/btreemap to re-id
            // all the vertices shared between meshes.
            // FIXME: Assumes normals have exact same length as positions
            // and correspond 1-to-1 with position index.
            // Ideally have a seperate normal index which is nullable.
            // Move offset to the end of this meshes vertices
            if bvh.tri_positions.len() != bvh.tri_normals.len() {
                panic!("BAD! LOOK AT THE FIXME ABOVE");
            }
            let tpl = bvh.tri_positions.len();
            let tfl = bvh.tri_faces.len();

            nodes.append(
                &mut bvh
                    .nodes
                    .into_iter()
                    .map(|n| BVHNodeGPU::from(n))
                    .map(|mut n| {
                        if n.is_leaf == 1 {
                            n.start += face_offset;
                            n.end += face_offset;
                        }
                        n
                    })
                    .collect_vec(),
            );
            tri_positions.append(
                &mut bvh
                    .tri_positions
                    .into_iter()
                    .map(|p| [p.x, p.y, p.z, 0.0])
                    .collect(),
            );
            tri_normals.append(
                &mut bvh
                    .tri_normals
                    .into_iter()
                    .map(|p| [p.x, p.y, p.z, 0.0])
                    .collect(),
            );
            tri_faces.append(
                &mut bvh
                    .tri_faces
                    .iter()
                    .map(|f| f.map(|i| i + vertex_offset))
                    .map(|p| [p.x, p.y, p.z, 0])
                    .collect(),
            );

            vertex_offset += tpl as u32;
            face_offset += tfl as u32;
        }

        // Make the node buffer:
        let node_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVHNode buffer"),
            contents: bytemuck::cast_slice(&nodes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Make the position buffer:
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Position buffer"),
            contents: bytemuck::cast_slice(&tri_positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Make the faces buffer:
        let face_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face buffer"),
            contents: bytemuck::cast_slice(&tri_faces),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Make the normals buffer:
        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Normal buffer"),
            contents: bytemuck::cast_slice(&tri_normals),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Triangles bindgroup layout descriptor"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangles bindgroup"),
            layout: &bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: node_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: face_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: normal_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            nodes,
            tri_positions,
            tri_normals,
            tri_faces,
            bindgroup,
            bindgroup_layout,
        }
    }
}
