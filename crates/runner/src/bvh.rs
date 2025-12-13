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

#[derive(Clone, Copy, Debug, Default)]
pub struct BVHNode {
    bounds: AABB,
    left: usize,
    right: usize,
    is_leaf: bool,
    start: usize,
    end: usize,
}

impl BVHNode {
    fn bounds(&self) -> AABB {
        self.bounds
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
        if !node.is_leaf {
            let l = &self.nodes[node.left];
            let r = &self.nodes[node.right];
            node.bounds = l.bounds().union(&r.bounds());
        } else {
            let mut new_bounds = self.face_bounds(node.start);
            for i in node.start + 1..node.end {
                new_bounds = new_bounds.union(&self.face_bounds(i))
            }
            node.bounds = new_bounds;
        }
        self.nodes[node_idx] = node;
    }

    fn subdivide(&mut self, node_idx: usize, threshold: usize) {
        let node = self.nodes[node_idx];
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
                if self.face_centroid(i)[axis] < split {
                    i += 1;
                } else {
                    self.tri_faces.swap(i, j);
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

            let l = self.nodes.len();
            self.nodes.push(left);
            let r = self.nodes.len();
            self.nodes.push(right);

            self.compute_bounds(l);
            self.compute_bounds(r);
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

        self.nodes[node_idx] = node;
        self.compute_bounds(node_idx);
    }

    /// Converts all right children into skip connections
    /// for a stackless traversal.
    /// Must be done after the BVH is fully constructed,
    /// and donly done once.
    pub fn rights_to_skips(&mut self, parent_idx: usize, node_idx: usize) {
        let BVHNode {
            bounds,
            left,
            right,
            is_leaf,
            start,
            end,
        } = self.nodes[node_idx];

        let BVHNode { right: p_right, .. } = self.nodes[parent_idx];

        // Left child
        if !is_leaf {
            self.rights_to_skips(node_idx, left);
        }

        // Self:
        self.nodes[node_idx] = BVHNode {
            bounds,
            left,
            right: if node_idx == 0 { 0 } else { p_right },
            start,
            end,
            is_leaf,
        };

        // Right child:
        if !is_leaf {
            self.rights_to_skips(node_idx, right);
        }
    }

    pub fn new(mesh: Mesh) -> BVH {
        let mut bvh = BVH {
            nodes: vec![BVHNode {
                is_leaf: true,
                bounds: AABB::default(),
                start: 0,
                end: mesh.faces.len(),
                ..Default::default()
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
        bvh.subdivide(0, 8);
        bvh.rights_to_skips(0, 0);

        bvh
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct BVHNodeGPU {
    pub lower_bound: [f32; 3], // last one is padding
    pub _pad0: u32,
    pub upper_bound: [f32; 3], // last one is padding
    pub _pad2: u32,
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
            lower_bound: [value.bounds.lb.x, value.bounds.lb.y, value.bounds.lb.z],
            upper_bound: [value.bounds.ub.x, value.bounds.ub.y, value.bounds.ub.z],
            left: value.left as u32,
            right: value.right as u32,
            is_leaf: value.is_leaf as u32,
            start: value.start as u32,
            end: value.end as u32,
            ..Default::default()
        }
    }
}

pub struct BLAS {
    pub nodes: Vec<BVHNodeGPU>,
    pub roots: Vec<u32>,
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
        let mut roots = Vec::new();
        let mut face_offset = 0;
        let mut vertex_offset = 0;
        let mut node_offset = 0;

        for bvh in bvhs {
            // TODO: Can be optimised using a hashmap/btreemap to re-id
            // all the vertices shared between meshes.
            // FIXME: Assumes normals have exact same length as positions
            // and correspond 1-to-1 with position index.
            // Ideally have a seperate normal index which is nullable.
            // Move offset to the end of this meshes vertices
            if bvh.tri_positions.len() != bvh.tri_normals.len() {
                // panic!("BAD! LOOK AT THE FIXME ABOVE");
            }
            let tpl = bvh.tri_positions.len();
            let tfl = bvh.tri_faces.len();
            let nl = bvh.nodes.len();

            roots.push(node_offset);

            nodes.append(
                &mut bvh
                    .nodes
                    .into_iter()
                    .map(|n| BVHNodeGPU::from(n))
                    .map(|mut n| {
                        n.start += face_offset;
                        n.end += face_offset;
                        n.left += node_offset;
                        n.right += node_offset;
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
            node_offset += nl as u32;
        }

        if tri_normals.len() == 0 {
            tri_normals.push(Default::default());
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

        // Maps bvh/mesh ids (the order they come in) to their root location
        let root_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Root buffer"),
            contents: bytemuck::cast_slice(&roots),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: root_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            nodes,
            roots,
            tri_positions,
            tri_normals,
            tri_faces,
            bindgroup,
            bindgroup_layout,
        }
    }
}
