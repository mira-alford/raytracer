use std::collections::HashMap;

use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::app::BevyApp;

pub fn initialize(app: &mut BevyApp) {
    app.world.insert_resource(MaterialServer::default());
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Component)]
pub struct Material {
    pub colour_texture: u32,             // 0 -> use base colour
    pub emissive_texture: u32,           // 0 -> use base emissive
    pub metallic_roughness_texture: u32, // 0 -> use base metallic/roughness
    pub normal_texture: u32,             // 0 -> use mesh vertex normals
    pub colour: Vec4,                    // 0.0..=1.0 rgba
    pub emissive: Vec4,                  // 0.0..=1.0 rgba
    pub metallic: f32,                   // 0.0..=1.0
    pub roughness: f32,                  // 0.0..=1.0
    pub ior: f32,
    pub transmission: f32, // 0.0..=1.0
}

impl Default for Material {
    fn default() -> Self {
        Self {
            colour_texture: Default::default(),
            emissive_texture: Default::default(),
            metallic_roughness_texture: Default::default(),
            normal_texture: Default::default(),
            colour: Default::default(),
            emissive: Default::default(),
            metallic: Default::default(),
            roughness: Default::default(),
            ior: 1.5,
            transmission: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Component, Debug, Hash, Eq, PartialEq)]
pub struct MaterialId(usize);

#[derive(Resource, Default)]
pub struct MaterialServer {
    materials: Vec<Material>,
    by_label: HashMap<String, MaterialId>,
}

impl MaterialServer {
    pub fn add_material(&mut self, material: Material) -> MaterialId {
        self.materials.push(material);
        MaterialId(self.materials.len() - 1)
    }

    pub fn add_material_labelled(&mut self, material: Material, label: String) -> MaterialId {
        if let Some(id) = self.by_label.get(&label) {
            return *id;
        }
        let id = self.add_material(material);
        self.by_label.insert(label, id);
        id
    }

    pub fn get(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(id.0)
    }
}

// use wesl::include_wesl;
// use wgpu::{ShaderModule, include_spirv, util::DeviceExt};

// use crate::{
//     blas,
//     camera::{self},
//     instance, path, queue, tlas,
// };

// #[derive(Clone)]
// pub struct Material {
//     pub label: Option<String>,
//     pub data_buffer: wgpu::Buffer,
//     pub data_bindgroup: wgpu::BindGroup,
//     pub data_bindgroup_layout: wgpu::BindGroupLayout,
//     pub pipeline: wgpu::ComputePipeline,
// }

// impl Material {
//     pub fn new<T>(
//         device: &wgpu::Device,
//         shader: ShaderModule,
//         path_buffer: &path::Paths,
//         material_queue: &queue::Queue,
//         extension_queue: &queue::Queue,
//         instances: &instance::Instances,
//         data: &Vec<T>,
//         blas_data: &blas::BLASData,
//         tlas_data: &tlas::TLASData,
//         light_sample_bindgroup_layout: &wgpu::BindGroupLayout,
//         label: Option<&str>,
//     ) -> Self
//     where
//         T: bytemuck::Pod + bytemuck::Zeroable,
//     {
//         let data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label,
//             contents: bytemuck::cast_slice(&data),
//             usage: wgpu::BufferUsages::STORAGE,
//         });

//         let data_bindgroup_layout =
//             device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//                 label,
//                 entries: &[wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::COMPUTE,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Storage { read_only: true },
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 }],
//             });

//         let data_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label,
//             layout: &data_bindgroup_layout,
//             entries: &[wgpu::BindGroupEntry {
//                 binding: 0,
//                 resource: data_buffer.as_entire_binding(),
//             }],
//         });

//         let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: Some(&format!(
//                 "{}Phase Pipeline Layout",
//                 label.unwrap_or_default()
//             )),
//             bind_group_layouts: &[
//                 &path_buffer.path_bind_group_layout,
//                 &material_queue.bind_group_layout,
//                 &extension_queue.bind_group_layout,
//                 &data_bindgroup_layout,
//                 &light_sample_bindgroup_layout,
//                 &blas_data.bindgroup_layout,
//                 &tlas_data.bindgroup_layout,
//                 &instances.bindgroup_layout,
//             ],
//             push_constant_ranges: &[],
//         });

//         let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
//             label: Some(&format!("{}Phase Pipeline", label.unwrap_or_default())),
//             layout: Some(&pipeline_layout),
//             module: &shader,
//             entry_point: Some("main"),
//             compilation_options: Default::default(),
//             cache: Default::default(),
//         });

//         Self {
//             label: label.map(|l| l.to_owned()),
//             data_buffer,
//             data_bindgroup,
//             data_bindgroup_layout,
//             pipeline,
//         }
//     }

//     pub fn render(
//         &self,
//         device: &wgpu::Device,
//         path_buffer: &path::Paths,
//         material_queue: &queue::Queue,
//         extension_queue: &queue::Queue,
//         blas_data: &blas::BLASData,
//         tlas_data: &tlas::TLASData,
//         instances: &instance::Instances,
//         light_sample_bindgroup: &wgpu::BindGroup,
//     ) -> wgpu::CommandBuffer {
//         let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some(&format!(
//                 "{} Encoder",
//                 self.label.clone().unwrap_or_default()
//             )),
//         });

//         let mut compute_pass = encoder.begin_compute_pass(&Default::default());
//         compute_pass.set_pipeline(&self.pipeline);
//         compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
//         compute_pass.set_bind_group(1, &material_queue.bind_group, &[]);
//         compute_pass.set_bind_group(2, &extension_queue.bind_group, &[]);
//         compute_pass.set_bind_group(3, &self.data_bindgroup, &[]);
//         compute_pass.set_bind_group(4, light_sample_bindgroup, &[]);
//         compute_pass.set_bind_group(5, &blas_data.bindgroup, &[]);
//         compute_pass.set_bind_group(6, &tlas_data.bindgroup, &[]);
//         compute_pass.set_bind_group(7, &instances.bindgroup, &[]);
//         compute_pass.dispatch_workgroups(material_queue.size.div_ceil(64), 1, 1);

//         drop(compute_pass);

//         encoder.finish()
//     }
// }
