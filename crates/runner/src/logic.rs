use tracing::info;
use wesl::include_wesl;
use wgpu::util::DeviceExt;

use crate::{path, queue};

pub struct LogicPhase {
    pipeline: wgpu::ComputePipeline,
    output_buffer: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
}

impl LogicPhase {
    pub fn new(
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        material_queues: &[queue::Queue],
        dims: (u32, u32),
    ) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LogicPhase"),
            source: wgpu::ShaderSource::Wgsl(include_wesl!("logic").into()),
        });

        // Pixel output buffer (atomically written to in shader?):
        let output_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LogicPhase Output"),
            contents: bytemuck::cast_slice(&(0..(dims.0 * dims.1)).collect::<Vec<_>>()),
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

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LogicPhase Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            pipeline,
            output_buffer,
            output_bind_group,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        material_queues: &[queue::Queue],
        dims: (u32, u32),
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogicPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &new_ray_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.output_bind_group, &[]);
        for (i, m) in material_queues.iter().enumerate() {
            compute_pass.set_bind_group(i as u32 + 3, &m.bind_group, &[]);
        }
        compute_pass.dispatch_workgroups((dims.0).div_ceil(16), (dims.1).div_ceil(16), 1);
        drop(compute_pass);

        encoder.finish()
    }

    pub fn output<'a>(&'a self) -> &'a wgpu::Buffer {
        &self.output_buffer
    }
}
