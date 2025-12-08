use wesl::include_wesl;
use wgpu::{include_spirv, util::DeviceExt};

use crate::{
    camera::{self, CameraData},
    path, queue,
};

pub struct NewRayPhase {
    pipeline: wgpu::ComputePipeline,
}

impl NewRayPhase {
    pub fn new(
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        extension_queue: &queue::Queue,
        camera: &camera::Camera,
    ) -> Self {
        let compute_shader =
            device.create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/new_ray.spv")));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NewRayPhase Pipeline Layout"),
            bind_group_layouts: &[
                &path_buffer.path_bind_group_layout,
                &new_ray_queue.bind_group_layout,
                &extension_queue.bind_group_layout,
                &camera.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("NewRayPhase Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self { pipeline }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        extension_queue: &queue::Queue,
        camera: &camera::Camera,
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogicPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &new_ray_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(3, &camera.bind_group, &[]);
        compute_pass.dispatch_workgroups(new_ray_queue.size.div_ceil(64), 1, 1);

        drop(compute_pass);

        encoder.finish()
    }
}
