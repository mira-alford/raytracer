use itertools::Itertools;
use wgpu::{BindGroupLayoutEntry, util::DeviceExt};

pub struct Mesh {
    pub positions: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 4]>,
    pub faces: Vec<[u32; 4]>,
}

impl Mesh {
    pub fn from_model(model: &tobj::Mesh) -> Self {
        // let mut positions = Vec::new();
        let positions = model
            .positions
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], 0.0])
            .collect_vec();

        let mut normals = model
            .normals
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], 0.0])
            .collect_vec();

        let faces = model
            .indices
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], 0])
            .collect_vec();

        Self {
            positions,
            normals,
            faces,
        }
    }
}

pub struct Meshes {
    pub unified: Mesh,
    pub triangles_bindgroup: wgpu::BindGroup,
    pub triangles_bindgroup_layout: wgpu::BindGroupLayout,
}

impl Meshes {
    pub fn new(device: &wgpu::Device, meshes: Vec<Mesh>) -> Self {
        // Merge the meshes
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut faces = Vec::new();
        let mut offset = 0;

        for mut mesh in meshes {
            // TODO: Can be optimised using a hashmap/btreemap to re-id
            // all the vertices shared between meshes.
            positions.append(&mut mesh.positions);
            normals.append(&mut mesh.normals);
            faces.append(&mut mesh.faces.iter().map(|f| f.map(|i| i + offset)).collect());

            // Move offset to the end of this meshes vertices
            offset += mesh.positions.len() as u32;
        }

        let unified = Mesh {
            positions,
            normals,
            faces,
        };

        // Make the position buffer:
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Position buffer"),
            contents: bytemuck::cast_slice(&unified.positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Make the faces buffer:
        let face_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face buffer"),
            contents: bytemuck::cast_slice(&unified.faces),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Make the normals buffer:
        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Normal buffer"),
            contents: bytemuck::cast_slice(&unified.normals),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let triangles_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Triangles bindgroup layout descriptor"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
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

        let triangles_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Triangles bindgroup"),
            layout: &triangles_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: face_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: normal_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            unified,
            triangles_bindgroup,
            triangles_bindgroup_layout,
        }
    }
}
