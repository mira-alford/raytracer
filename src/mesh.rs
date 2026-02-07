use std::{collections::HashMap, sync::Arc};

use bevy_ecs::prelude::*;
use crossbeam::channel::bounded;
use glam::{UVec3, UVec4, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::{
    app::BevyApp,
    blas::BLAS,
    bvh::{AABB, BVH, BVHNodeGPU},
    render_resources::RenderDevice,
    schedule::{self},
};

pub fn initialize(app: &mut BevyApp) {
    app.world.insert_resource(MeshServer::default());
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::Update, mesh_loading_system);
}

#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub positions: Vec<Vec4>,
    pub normals: Vec<Vec4>,
    pub faces: Vec<UVec4>,
    // pub uv: Vec<UVec2>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct GeometryOffsets {
    pub vertex: u32,
    pub index: u32,
    pub nodes: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct GPUVertexData {
    position: Vec4,
    normal: Vec4,
    uv: Vec4,
}

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq, Hash)]
pub struct MeshId(usize);

#[derive(Hash, Clone, PartialEq, Eq)]
pub enum MeshDescriptor {
    TOBJ(String),
    Rect,
    Cube,
}

pub struct MeshData {
    pub nodes: Vec<BVHNodeGPU>,
    pub mesh: Mesh,
    pub aabb: AABB,
}

pub struct MeshLoading {
    descriptor: MeshDescriptor,
    id: MeshId,
    rx: Option<crossbeam::channel::Receiver<MeshData>>,
}

#[derive(Resource, Default)]
pub struct MeshServer {
    loading: Vec<MeshLoading>,
    data: Vec<Option<MeshData>>,
    counter: usize,
    by_desc: HashMap<MeshDescriptor, MeshId>,
    node_buffer: Option<wgpu::Buffer>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    offset_buffer: Option<wgpu::Buffer>,
    aabbs: Vec<AABB>,
    mesh_id_to_geom_id: HashMap<usize, u32>,
}

fn mesh_loading_system(mut mesh_server: ResMut<MeshServer>, device: Res<RenderDevice>) {
    let MeshServer { loading, data, .. } = mesh_server.as_mut();

    let mut changed = false;
    loading.retain_mut(|l| {
        if let Some(rx) = &l.rx {
            if let Ok(d) = rx.try_recv() {
                data[l.id.0] = Some(d);
                changed = true;
                false
            } else {
                true
            }
        } else {
            l.start();
            true
        }
    });

    if changed {
        mesh_server.regenerate_buffer(device.0.clone());
    }
}

impl MeshLoading {
    fn start(&mut self) {
        if self.rx.is_some() {
            return;
        }

        let (tx, rx) = bounded::<MeshData>(1);
        self.rx = Some(rx);

        rayon::spawn({
            // let device = device.clone();
            let descriptor = self.descriptor.clone();
            move || {
                let mut load_options = tobj::GPU_LOAD_OPTIONS;
                load_options.single_index = false;
                let mesh = match &descriptor {
                    MeshDescriptor::TOBJ(s) => {
                        Mesh::from_model(&tobj::load_obj(s, &load_options).unwrap().0[0].mesh)
                    }
                    MeshDescriptor::Rect => Mesh::rect(),
                    MeshDescriptor::Cube => Mesh::cube(),
                };

                let blas = BLAS::new(mesh);
                let aabb = blas.node_bounds(0);
                let mesh = blas.mesh;
                let nodes = blas
                    .nodes
                    .into_iter()
                    .map(|node| BVHNodeGPU::from(node))
                    .collect_vec();

                tx.send(MeshData { nodes, mesh, aabb })
                    .expect("Expected to send mesh data");
            }
        });
    }
}

impl MeshServer {
    pub fn load_mesh(&mut self, descriptor: MeshDescriptor) -> MeshId {
        if let Some(id) = self.by_desc.get(&descriptor) {
            return *id;
        }
        let id = MeshId(self.counter);
        self.data.push(None);
        self.counter += 1;

        self.loading.push(MeshLoading {
            descriptor: descriptor.clone(),
            id,
            rx: None,
        });

        self.by_desc.insert(descriptor, id);
        id
    }

    pub fn mesh_data(&self, id: MeshId) -> Option<&MeshData> {
        if id.0 >= self.data.len() {
            return None;
        }
        self.data[id.0].as_ref()
    }

    pub fn vertex_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.index_buffer
    }

    pub fn node_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.node_buffer
    }

    pub fn offset_buffer(&self) -> &Option<wgpu::Buffer> {
        &self.offset_buffer
    }

    pub fn aabbs(&self) -> &Vec<AABB> {
        &self.aabbs
    }

    pub fn geom_id(&self, id: MeshId) -> Option<u32> {
        self.mesh_id_to_geom_id.get(&id.0).copied()
    }

    pub fn regenerate_buffer(&mut self, device: Arc<wgpu::Device>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut nodes = Vec::new();
        let mut aabbs = Vec::new();

        let mut mesh_id_to_geom_id = HashMap::new();
        let mut geom_id: u32 = 0;
        let mut offsets = Vec::new();

        for (mesh_id, mesh_data) in self
            .data
            .iter()
            .enumerate()
            .filter_map(|(id, m)| m.as_ref().map(|m| (id, m)))
        {
            dbg!(mesh_id);
            let Mesh {
                positions,
                normals,
                faces,
            } = mesh_data.mesh.clone();

            // Map the mesh id to geometry id for packing:
            mesh_id_to_geom_id.insert(mesh_id, geom_id);
            geom_id += 1;

            // Produce offset for start of this geometry in each buffer:
            offsets.push(GeometryOffsets {
                vertex: vertices.len() as u32,
                index: indices.len() as u32,
                nodes: nodes.len() as u32,
            });

            // Push the new data onto the buffers:
            aabbs.push(mesh_data.aabb);
            nodes.extend(mesh_data.nodes.clone());
            vertices.extend_from_slice(
                positions
                    .into_iter()
                    .zip(normals)
                    .map(|(position, normal)| GPUVertexData {
                        position,
                        normal,
                        uv: Vec4::ZERO,
                    })
                    .collect_vec()
                    .as_slice(),
            );
            indices.extend(faces);
        }

        self.mesh_id_to_geom_id = mesh_id_to_geom_id;

        self.aabbs = aabbs;

        self.node_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh BVHNode Buffer"),
                contents: bytemuck::cast_slice(&nodes),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        self.index_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );

        self.offset_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Geometry Offset Buffer"),
                contents: bytemuck::cast_slice(&offsets),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        )
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
