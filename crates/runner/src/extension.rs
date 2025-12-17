use wesl::include_wesl;
use wgpu::{include_spirv, util::DeviceExt};

use crate::{
    blas::{self, BLASData},
    instance::{Instance, Instances},
    mesh::Meshes,
    path, queue, tlas,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
}

pub struct ExtensionPhase {
    pipeline: wgpu::ComputePipeline,
    reset_pipeline: wgpu::ComputePipeline,
    primitives_buffer: wgpu::Buffer,
    primitives_bindgroup_layout: wgpu::BindGroupLayout,
    primitives_bindgroup: wgpu::BindGroup,
}

impl ExtensionPhase {
    pub fn new(
        device: &wgpu::Device,
        paths: &path::Paths,
        extension_queue: &queue::Queue,
        blas_data: &blas::BLASData,
        tlas_data: &tlas::TLASData,
        primitives: &[Sphere],
        instances: &Instances,
    ) -> Self {
        let compute_shader =
            device.create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/extension.spv")));

        // Primitives Initialization:
        let primitives_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ExtensionPhase Primitives Buffer"),
            contents: bytemuck::cast_slice(primitives),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let primitives_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ExtensionPhase Primitives Bindgroup Layout"),
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

        let primitives_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ExtensionPhase Primitives Bindgroup"),
            layout: &primitives_bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: primitives_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ExtensionPhase Pipleline Layout"),
            bind_group_layouts: &[
                &paths.path_bind_group_layout,
                &extension_queue.bind_group_layout,
                &primitives_bindgroup_layout,
                &blas_data.bindgroup_layout,
                &instances.bindgroup_layout,
                &tlas_data.bindgroup_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ExtensionPhase Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("extensionMain"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let reset_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ExtensionPhase reset Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("extensionReset"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            pipeline,
            reset_pipeline,
            primitives_buffer,
            primitives_bindgroup_layout,
            primitives_bindgroup,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        extension_queue: &queue::Queue,
        blas_data: &blas::BLASData,
        tlas_data: &tlas::TLASData,
        instances: &Instances,
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ExtensionPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.primitives_bindgroup, &[]);
        compute_pass.set_bind_group(3, &blas_data.bindgroup, &[]);
        compute_pass.set_bind_group(4, &instances.bindgroup, &[]);
        compute_pass.set_bind_group(5, &tlas_data.bindgroup, &[]);
        compute_pass.dispatch_workgroups(extension_queue.size.div_ceil(64), 1, 1);

        // Reset extension queue after done:
        compute_pass.set_pipeline(&self.reset_pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.primitives_bindgroup, &[]);
        compute_pass.set_bind_group(3, &blas_data.bindgroup, &[]);
        compute_pass.set_bind_group(4, &instances.bindgroup, &[]);
        compute_pass.set_bind_group(5, &tlas_data.bindgroup, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1);

        drop(compute_pass);

        encoder.finish()
    }
}
