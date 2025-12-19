use tracing::info;
use wesl::include_wesl;
use wgpu::{include_spirv, util::DeviceExt};

use crate::{camera, dims::Dims, path, queue, sample};

pub struct LogicPhase {
    start_pipeline: wgpu::ComputePipeline,
    pipeline: wgpu::ComputePipeline,
    output_buffer: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
}

impl LogicPhase {
    pub fn new(
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        samples: &sample::Samples,
        camera: &camera::Camera,
        new_ray_queue: &queue::Queue,
        material_queues: &[&queue::Queue],
        dims: &Dims,
    ) -> Self {
        let compute_shader =
            device.create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/logic.spv")));

        let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LogicPhase Output"),
            contents: bytemuck::cast_slice(&(0..(dims.size())).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        });

        let output_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("LogicPhase Output Bindgroup Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let output_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LogicPhase Output Bindgroup"),
            layout: &output_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            }],
        });

        // TODO this is actually a mess lol.
        // please clean this, it feels bad.
        let bind_group_layouts = std::iter::chain(
            [
                path_buffer.path_bind_group_layout.clone(),
                new_ray_queue.bind_group_layout.clone(),
                output_bind_group_layout,
                dims.bindgroup_layout.clone(),
                samples.bind_group_layout.clone(),
                camera.bind_group_layout.clone(),
            ]
            .into_iter(),
            material_queues
                .iter()
                .map(|mq| mq.bind_group_layout.clone()),
        )
        .collect::<Vec<_>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LogicPhase Pipleline Layout"),
            bind_group_layouts: bind_group_layouts.iter().collect::<Vec<_>>().as_slice(),
            push_constant_ranges: &[],
        });

        let start_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LogicPhase Start Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("logicStart"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LogicPhase Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("logicMain"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            start_pipeline,
            pipeline,
            output_buffer,
            output_bind_group,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        samples: &sample::Samples,
        camera: &camera::Camera,
        new_ray_queue: &queue::Queue,
        material_queues: &[&queue::Queue],
        dims: &Dims,
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogicPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());

        // Start Pipeline to reset the queues:
        compute_pass.set_pipeline(&self.start_pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &new_ray_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.output_bind_group, &[]);
        compute_pass.set_bind_group(3, &dims.bindgroup, &[]);
        compute_pass.set_bind_group(4, &samples.bind_group, &[]);
        compute_pass.set_bind_group(5, &camera.bind_group, &[]);
        for (i, m) in material_queues.iter().enumerate() {
            compute_pass.set_bind_group(i as u32 + 6, &m.bind_group, &[]);
        }
        compute_pass.dispatch_workgroups(1, 1, 1);

        // Main Pipeilne:
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &new_ray_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.output_bind_group, &[]);
        compute_pass.set_bind_group(3, &dims.bindgroup, &[]);
        compute_pass.set_bind_group(4, &samples.bind_group, &[]);
        compute_pass.set_bind_group(5, &camera.bind_group, &[]);
        for (i, m) in material_queues.iter().enumerate() {
            compute_pass.set_bind_group(i as u32 + 6, &m.bind_group, &[]);
        }
        compute_pass.dispatch_workgroups(dims.threads.div_ceil(64), 1, 1);
        drop(compute_pass);

        encoder.finish()
    }

    pub fn output<'a>(&'a self) -> &'a wgpu::Buffer {
        &self.output_buffer
    }
}
