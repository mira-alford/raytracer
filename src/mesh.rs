use std::collections::HashMap;

use bevy_ecs::prelude::*;
use glam::{UVec2, UVec3, UVec4, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use wgpu::{BindGroupLayoutEntry, util::DeviceExt};

use crate::app::BevyApp;

pub fn initialize(app: &mut BevyApp) {
    app.world.insert_resource(MeshServer::default());
}

#[derive(Default)]
pub struct Mesh {
    pub positions: Vec<Vec4>,
    pub normals: Vec<Vec4>,
    pub faces: Vec<UVec4>,
    // pub uv: Vec<UVec2>,
}

#[derive(Clone, Copy, Component, Debug)]
pub struct MeshId(usize);

#[derive(Hash, Clone, PartialEq, Eq)]
pub enum MeshDescriptor {
    TOBJ(String),
    Rect,
    Cube,
}

#[derive(Resource, Default)]
pub struct MeshServer {
    meshes: Vec<Mesh>,
    by_desc: HashMap<MeshDescriptor, MeshId>,
}

impl MeshServer {
    pub fn load_mesh(&mut self, descriptor: MeshDescriptor) -> MeshId {
        if let Some(id) = self.by_desc.get(&descriptor) {
            return *id;
        }
        let mut load_options = tobj::GPU_LOAD_OPTIONS;
        load_options.single_index = false;
        self.meshes.push(match &descriptor {
            MeshDescriptor::TOBJ(s) => {
                Mesh::from_model(&tobj::load_obj(s, &load_options).unwrap().0[0].mesh)
            }
            MeshDescriptor::Rect => Mesh::rect(),
            MeshDescriptor::Cube => Mesh::cube(),
        });
        let id = MeshId(self.meshes.len() - 1);
        self.by_desc.insert(descriptor, id);
        id
    }
}

impl Mesh {
    pub fn new(positions: Vec<Vec4>, indices: Vec<u32>, normals: Vec<Vec4>) -> Self {
        let faces = indices
            .chunks_exact(3)
            .into_iter()
            .map(|p| UVec3::from_slice(p).extend(0))
            .collect_vec();

        let normals = if normals.len() >= positions.len() && !normals.is_empty() {
            normals
        } else {
            Self::compute_vertex_normals_ccw(&positions, &indices)
        };

        Self {
            positions,
            normals,
            faces,
        }
    }

    pub fn from_model(model: &tobj::Mesh) -> Self {
        let positions = model
            .positions
            .chunks_exact(3)
            .map(|chunk| Vec3::from_slice(chunk).extend(0.0))
            .collect_vec();

        let len = positions.len();
        let center: Vec4 = positions.iter().sum::<Vec4>() / (len as f32);

        let positions = positions.into_iter().map(|p| p - center).collect_vec();

        // Calculate the greatest distance from center
        // so we can scale down such that furthest point is on the unit cube
        let extent: Vec4 = positions
            .iter()
            .copied()
            .reduce(|acc, p| p.max(acc))
            .unwrap_or_default();

        let positions = positions
            .into_iter()
            .map(|p| p.xyz() / extent.xyz())
            .map(|p| p.extend(1.0))
            .collect_vec();

        let faces = model
            .indices
            .chunks_exact(3)
            .map(|chunk| UVec3::from_slice(chunk).extend(0))
            .collect_vec();

        let normals = if model.normals.len() >= model.positions.len() && !model.normals.is_empty() {
            model
                .normals
                .chunks_exact(3)
                .map(|c| Vec3::from_slice(c).extend(0.0))
                .collect_vec()
        } else {
            Self::compute_vertex_normals_ccw(&positions, &model.indices)
        };

        Self {
            positions,
            normals,
            faces,
        }
    }

    fn compute_vertex_normals_ccw(positions: &Vec<Vec4>, indices: &[u32]) -> Vec<Vec4> {
        let mut acc = vec![Vec4::ZERO; positions.len()];

        for tri in indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let p0 = positions[i0];
            let p1 = positions[i1];
            let p2 = positions[i2];

            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let mut n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            let l2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
            if l2 > 0.0 {
                let inv_len = 1.0 / l2.sqrt();
                n[0] *= inv_len;
                n[1] *= inv_len;
                n[2] *= inv_len;

                acc[i0][0] += n[0];
                acc[i0][1] += n[1];
                acc[i0][2] += n[2];

                acc[i1][0] += n[0];
                acc[i1][1] += n[1];
                acc[i1][2] += n[2];

                acc[i2][0] += n[0];
                acc[i2][1] += n[1];
                acc[i2][2] += n[2];
            }
        }

        for a in &mut acc {
            let l2 = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
            if l2 > 0.0 {
                let inv_len = 1.0 / l2.sqrt();
                a[0] *= inv_len;
                a[1] *= inv_len;
                a[2] *= inv_len;
            } else {
                a[2] = 1.0;
            }
            a[3] = 0.0;
        }

        acc
    }

    pub fn rect() -> Self {
        let positions = vec![
            Vec4::new(-0.5, -0.5, 0.0, 1.0),
            Vec4::new(0.5, -0.5, 0.0, 1.0),
            Vec4::new(0.5, 0.5, 0.0, 1.0),
            Vec4::new(-0.5, 0.5, 0.0, 1.0),
        ];

        let normals = vec![
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
        ];

        let faces = vec![UVec4::new(0, 1, 2, 0), UVec4::new(0, 2, 3, 0)];

        Self {
            positions,
            normals,
            faces,
        }
    }

    pub fn cube() -> Self {
        let positions = vec![
            // +X face
            [0.5, -0.5, -0.5, 1.0],
            [0.5, 0.5, -0.5, 1.0],
            [0.5, 0.5, 0.5, 1.0],
            [0.5, -0.5, 0.5, 1.0],
            // -X face
            [-0.5, -0.5, 0.5, 1.0],
            [-0.5, 0.5, 0.5, 1.0],
            [-0.5, 0.5, -0.5, 1.0],
            [-0.5, -0.5, -0.5, 1.0],
            // +Y face
            [-0.5, 0.5, -0.5, 1.0],
            [-0.5, 0.5, 0.5, 1.0],
            [0.5, 0.5, 0.5, 1.0],
            [0.5, 0.5, -0.5, 1.0],
            // -Y face
            [-0.5, -0.5, 0.5, 1.0],
            [-0.5, -0.5, -0.5, 1.0],
            [0.5, -0.5, -0.5, 1.0],
            [0.5, -0.5, 0.5, 1.0],
            // +Z face
            [-0.5, -0.5, 0.5, 1.0],
            [0.5, -0.5, 0.5, 1.0],
            [0.5, 0.5, 0.5, 1.0],
            [-0.5, 0.5, 0.5, 1.0],
            // -Z face
            [0.5, -0.5, -0.5, 1.0],
            [-0.5, -0.5, -0.5, 1.0],
            [-0.5, 0.5, -0.5, 1.0],
            [0.5, 0.5, -0.5, 1.0],
        ]
        .into_iter()
        .map(Vec4::from_array)
        .collect_vec();

        let normals = vec![
            // +X
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            // -X
            [-1.0, 0.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            // +Y
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            // -Y
            [0.0, -1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            // +Z
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            // -Z
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
        ]
        .into_iter()
        .map(Vec4::from_array)
        .collect_vec();

        let faces = vec![
            // each face: two CCW triangles
            [0, 1, 2, 0],
            [0, 2, 3, 0], // +X
            [4, 5, 6, 0],
            [4, 6, 7, 0], // -X
            [8, 9, 10, 0],
            [8, 10, 11, 0], // +Y
            [12, 13, 14, 0],
            [12, 14, 15, 0], // -Y
            [16, 17, 18, 0],
            [16, 18, 19, 0], // +Z
            [20, 21, 22, 0],
            [20, 22, 23, 0], // -Z
        ]
        .into_iter()
        .map(UVec4::from_array)
        .collect_vec();

        Self {
            positions,
            normals,
            faces,
        }
    }
}

// pub struct Meshes {
//     pub unified: Mesh,
//     pub triangles_bindgroup: wgpu::BindGroup,
//     pub triangles_bindgroup_layout: wgpu::BindGroupLayout,
// }

// impl Meshes {
//     pub fn new(device: &wgpu::Device, meshes: Vec<Mesh>) -> Self {
//         // Merge the meshes
//         let mut positions = Vec::new();
//         let mut normals = Vec::new();
//         let mut faces = Vec::new();
//         let mut offset = 0;

//         for mut mesh in meshes {
//             // TODO: Can be optimised using a hashmap/btreemap to re-id
//             // all the vertices shared between meshes.
//             positions.append(&mut mesh.positions);
//             normals.append(&mut mesh.normals);
//             faces.append(&mut mesh.faces.iter().map(|f| f.map(|i| i + offset)).collect());

//             // Move offset to the end of this meshes vertices
//             offset += mesh.positions.len() as u32;
//         }

//         let unified = Mesh {
//             positions,
//             normals,
//             faces,
//         };

//         // Make the position buffer:
//         let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Position buffer"),
//             contents: bytemuck::cast_slice(&unified.positions),
//             usage: wgpu::BufferUsages::STORAGE,
//         });

//         // Make the faces buffer:
//         let face_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Face buffer"),
//             contents: bytemuck::cast_slice(&unified.faces),
//             usage: wgpu::BufferUsages::STORAGE,
//         });

//         // Make the normals buffer:
//         let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Normal buffer"),
//             contents: bytemuck::cast_slice(&unified.normals),
//             usage: wgpu::BufferUsages::STORAGE,
//         });

//         let triangles_bindgroup_layout =
//             device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//                 label: Some("Triangles bindgroup layout descriptor"),
//                 entries: &[
//                     BindGroupLayoutEntry {
//                         binding: 0,
//                         visibility: wgpu::ShaderStages::COMPUTE,
//                         ty: wgpu::BindingType::Buffer {
//                             ty: wgpu::BufferBindingType::Storage { read_only: true },
//                             has_dynamic_offset: false,
//                             min_binding_size: None,
//                         },
//                         count: None,
//                     },
//                     BindGroupLayoutEntry {
//                         binding: 1,
//                         visibility: wgpu::ShaderStages::COMPUTE,
//                         ty: wgpu::BindingType::Buffer {
//                             ty: wgpu::BufferBindingType::Storage { read_only: true },
//                             has_dynamic_offset: false,
//                             min_binding_size: None,
//                         },
//                         count: None,
//                     },
//                     BindGroupLayoutEntry {
//                         binding: 2,
//                         visibility: wgpu::ShaderStages::COMPUTE,
//                         ty: wgpu::BindingType::Buffer {
//                             ty: wgpu::BufferBindingType::Storage { read_only: true },
//                             has_dynamic_offset: false,
//                             min_binding_size: None,
//                         },
//                         count: None,
//                     },
//                 ],
//             });

//         let triangles_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label: Some("Triangles bindgroup"),
//             layout: &triangles_bindgroup_layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: position_buffer.as_entire_binding(),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: face_buffer.as_entire_binding(),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 2,
//                     resource: normal_buffer.as_entire_binding(),
//                 },
//             ],
//         });

//         Self {
//             unified,
//             triangles_bindgroup,
//             triangles_bindgroup_layout,
//         }
//     }
// }
