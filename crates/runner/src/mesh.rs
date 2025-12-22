use itertools::Itertools;
use wgpu::{BindGroupLayoutEntry, util::DeviceExt};

pub struct Mesh {
    pub positions: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 4]>,
    pub faces: Vec<[u32; 4]>,
}

impl Mesh {
    pub fn from_model(model: &tobj::Mesh) -> Self {
        let positions = model
            .positions
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], 0.0])
            .collect_vec();

        let len = positions.len();
        let center = positions
            .iter()
            .copied()
            .reduce(|acc, pos| {
                [
                    acc[0] + pos[0],
                    acc[1] + pos[1],
                    acc[2] + pos[2],
                    acc[3] + pos[3],
                ]
            })
            .unwrap()
            .map(|i| i / len as f32);

        let positions = positions
            .into_iter()
            .map(|p| {
                [
                    (p[0] - center[0]),
                    (p[1] - center[1]),
                    (p[2] - center[2]),
                    (p[3] - center[3]),
                ]
            })
            .collect_vec();

        // Calculate the greatest distance from center
        // so we can scale down such that furthest point is on the unit cube
        let mut extent: f32 = 0.0;
        for pos in &positions {
            extent = extent.max((pos[0].powi(2) + pos[1].powi(2) + pos[2].powi(2)).sqrt());
        }

        let positions = positions
            .into_iter()
            .map(|p| [p[0] / extent, p[1] / extent, p[2] / extent, 1.0])
            .collect_vec();

        let normals = model
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

    pub fn rect() -> Self {
        // A unit quad centered at origin on the XY plane
        let positions = vec![
            [-0.5, -0.5, 0.0, 1.0], // bottom-left
            [0.5, -0.5, 0.0, 1.0],  // bottom-right
            [0.5, 0.5, 0.0, 1.0],   // top-right
            [-0.5, 0.5, 0.0, 1.0],  // top-left
        ];

        // All normals pointing +Z
        let normals = vec![
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];

        let faces = vec![[0, 1, 2, 0], [0, 2, 3, 0]];

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
