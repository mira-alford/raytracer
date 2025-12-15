use wgpu::{include_spirv, util::DeviceExt};

use crate::{material::Material, path, queue};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct LambertianData {
    pub albedo: [f32; 4],
}

pub struct LambertianPhase(Material);

impl LambertianPhase {
    pub fn new(
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        material_queue: &queue::Queue,
        extension_queue: &queue::Queue,
        lambertian_data: Vec<LambertianData>,
        label: Option<&str>,
    ) -> Self {
        let compute_shader = device
            .create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/lambertian.spv")));

        let data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&lambertian_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let data_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let data_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &data_bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: data_buffer.as_entire_binding(),
            }],
        });

        let mat = Material::new(
            device,
            compute_shader,
            path_buffer,
            material_queue,
            extension_queue,
            data_buffer,
            data_bindgroup,
            data_bindgroup_layout,
            label,
        );

        Self(mat)
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        material_queue: &queue::Queue,
        extension_queue: &queue::Queue,
    ) -> wgpu::CommandBuffer {
        self.0
            .render(device, path_buffer, material_queue, extension_queue)
    }
}
