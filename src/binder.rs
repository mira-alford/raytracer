use std::{collections::HashMap, num::NonZero};

use bevy_ecs::prelude::*;
use glam::Vec4;
use itertools::Itertools;
use wgpu::util::DeviceExt;

use crate::{
    app::BevyApp,
    bvh::{AABB, BVHNodeGPU},
    instance::Instance,
    material::{Material, MaterialId, MaterialServer},
    mesh::{MeshId, MeshServer},
    pathtracer::{Pathtracer, PathtracerOutput},
    render_resources::{RenderDevice, RenderQueue},
    schedule,
    tlas::TLAS,
    transform::Transform,
};

pub fn initialize(app: &mut BevyApp) {
    app.world.insert_resource(SceneBindings::default());
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::Update, binder_system);
}

#[derive(Resource, Default)]
pub struct SceneBindings {
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
}

#[derive(Resource)]
pub struct BinderLocal {
    tlas_cache: Option<wgpu::Buffer>,
    tlas_regenerate: bool,
}

impl Default for BinderLocal {
    fn default() -> Self {
        Self {
            tlas_cache: Default::default(),
            tlas_regenerate: true,
        }
    }
}

pub fn binder_system(
    objects: Query<(Ref<Transform>, Ref<MeshId>, &MaterialId)>,
    removed_transforms: RemovedComponents<Transform>,
    removed_meshids: RemovedComponents<MeshId>,
    mesh_server: Res<MeshServer>,
    material_server: Res<MaterialServer>,
    device: Res<RenderDevice>,
    mut binder_local: Local<BinderLocal>,
    mut path_tracer_bindings: ResMut<SceneBindings>,
) {
    let bind_group_layout = device
        .0
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pathtracer Bindgroup Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
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

    path_tracer_bindings.bind_group_layout = Some(bind_group_layout.clone());

    let Some(vertex_buffer) = mesh_server.vertex_buffer().as_ref() else {
        return;
    };
    let Some(index_buffer) = mesh_server.index_buffer().as_ref() else {
        return;
    };
    let Some(blas_node_buffer) = mesh_server.node_buffer().as_ref() else {
        return;
    };
    let Some(geometry_buffer) = mesh_server.offset_buffer().as_ref() else {
        return;
    };

    let mut materials = Vec::<Material>::new();
    let mut transforms = Vec::<Transform>::new();
    let mut instances = Vec::<Instance>::new();
    let mut materials_id_map = HashMap::<MaterialId, u32>::new();
    let mut light_sources = Vec::<u32>::new();

    if !removed_transforms.is_empty() && !removed_meshids.is_empty() {
        binder_local.tlas_regenerate = true;
    }

    // TODO:
    // let mut textures = vec![];
    // let mut samplers = vec![];

    for (transform, mesh_id, mat_id) in objects {
        if transform.is_changed() || mesh_id.is_changed() {
            binder_local.tlas_regenerate = true;
        }

        // Get the geometry index from the mesh server
        let Some(geometry_idx) = mesh_server.geom_id(*mesh_id) else {
            continue;
        };

        let mut emissive = false;
        let material_idx = if let Some(&idx) = materials_id_map.get(mat_id) {
            idx
        } else {
            let Some(material) = material_server.get(*mat_id) else {
                continue;
            };

            if material.emissive != Vec4::ZERO || material.emissive_texture > 0 {
                emissive = true;
            }
            materials.push(*material);

            let idx = (materials.len() - 1) as u32;
            materials_id_map.insert(*mat_id, idx);
            idx
        };

        transforms.push(*transform);
        let transform_idx = (transforms.len() - 1) as u32;

        let instance = Instance {
            transform_idx,
            geometry_idx,
            material_idx,
        };
        instances.push(instance);

        if emissive {
            light_sources.push((instances.len() - 1) as u32);
        }
    }

    if instances.is_empty() {
        // Gonna have a hard time binding this :)
        return;
    }

    if light_sources.is_empty() {
        // what to do here? Could insist that all indexes are >0 i guess
        // TODO properly support having no light sources lol
        light_sources.push(u32::MAX);
    }

    if binder_local.tlas_regenerate {
        // Regenerate the TLAS only when transforms or meshes have changed
        binder_local.tlas_regenerate = false;
        let tlas = TLAS::new(mesh_server.aabbs(), &transforms, &instances);
        binder_local.tlas_cache = Some(
            device
                .0
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TLAS BVHNode Buffer"),
                    contents: bytemuck::cast_slice(
                        tlas.nodes
                            .clone()
                            .into_iter()
                            .map(|node| BVHNodeGPU::from(node))
                            .collect_vec()
                            .as_slice(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE,
                }),
        );
    }

    let Some(tlas_node_buffer) = &binder_local.tlas_cache else {
        return;
    };

    // TODO: cache all of these!
    let material_buffer = device
        .0
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(materials.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let instance_buffer = device
        .0
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(instances.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let transform_buffer = device
        .0
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Buffer"),
            contents: bytemuck::cast_slice(transforms.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let light_sources_buffer = device
        .0
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Source Buffer"),
            contents: bytemuck::cast_slice(light_sources.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

    let bind_group = device.0.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Pathtracer Bindgroup Descriptor"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: geometry_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: blas_node_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: material_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: transform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: tlas_node_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: light_sources_buffer.as_entire_binding(),
            },
        ],
    });

    path_tracer_bindings.bind_group = Some(bind_group);
}
