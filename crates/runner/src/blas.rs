use glam::UVec3;
use glam::Vec3;
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::bvh::AABB;
use crate::bvh::BVH;
use crate::bvh::BVHNode;
use crate::bvh::BVHNodeGPU;
use crate::mesh::Mesh;

#[derive(Debug)]
pub struct BLAS {
    pub nodes: Vec<BVHNode>,
    pub tri_positions: Vec<Vec3>,
    pub tri_faces: Vec<UVec3>,
    pub tri_normals: Vec<Vec3>,
}

impl BVH for BLAS {
    fn elem_bounds(&self, face: usize) -> AABB {
        let face = self.tri_faces[face];
        let positions = face.to_array().map(|i| &self.tri_positions[i as usize]);
        let lb = positions[0].min(*positions[1]).min(*positions[2]);
        let ub = positions[0].max(*positions[1]).max(*positions[2]);
        AABB { lb, ub }
    }

    fn elem_centroid(&self, face: usize) -> Vec3 {
        let face = self.tri_faces[face];
        let positions = face.to_array().map(|i| self.tri_positions[i as usize]);
        positions.into_iter().reduce(|acc, v| acc + v).unwrap() / 3.0
    }

    fn elem_swap(&mut self, elem: usize, elem2: usize) {
        self.tri_faces.swap(elem, elem2);
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

impl BLAS {
    pub fn new(mesh: Mesh) -> BLAS {
        let mut bvh = BLAS {
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

        bvh.initialize(32);

        bvh
    }
}

pub struct BLASData {
    pub nodes: Vec<BVHNodeGPU>,
    pub roots: Vec<u32>,
    pub tri_positions: Vec<[f32; 4]>,
    pub tri_normals: Vec<[f32; 4]>,
    pub tri_faces: Vec<[u32; 4]>,
    pub bindgroup: wgpu::BindGroup,
    pub bindgroup_layout: wgpu::BindGroupLayout,
}

impl BLASData {
    pub fn new(device: &wgpu::Device, bvhs: Vec<BLAS>) -> Self {
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
