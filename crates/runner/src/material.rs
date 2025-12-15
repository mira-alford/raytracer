use wesl::include_wesl;
use wgpu::{ShaderModule, include_spirv, util::DeviceExt};

use crate::{
    camera::{self},
    path, queue,
};

pub struct Material {
    pub label: Option<String>,
    pub data_buffer: wgpu::Buffer,
    pub data_bindgroup: wgpu::BindGroup,
    pub data_bindgroup_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::ComputePipeline,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        shader: ShaderModule,
        path_buffer: &path::Paths,
        material_queue: &queue::Queue,
        extension_queue: &queue::Queue,
        data_buffer: wgpu::Buffer,
        data_bindgroup: wgpu::BindGroup,
        data_bindgroup_layout: wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!(
                "{}Phase Pipeline Layout",
                label.unwrap_or_default()
            )),
            bind_group_layouts: &[
                &path_buffer.path_bind_group_layout,
                &material_queue.bind_group_layout,
                &extension_queue.bind_group_layout,
                &data_bindgroup_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("{}Phase Pipeline", label.unwrap_or_default())),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            label: label.map(|l| l.to_owned()),
            data_buffer,
            data_bindgroup,
            data_bindgroup_layout,
            pipeline,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        material_queue: &queue::Queue,
        extension_queue: &queue::Queue,
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!(
                "{} Encoder",
                self.label.clone().unwrap_or_default()
            )),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &material_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(3, &self.data_bindgroup, &[]);
        compute_pass.dispatch_workgroups(material_queue.size.div_ceil(64), 1, 1);

        drop(compute_pass);

        encoder.finish()
    }
}
