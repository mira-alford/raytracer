use wesl::include_wesl;
use wgpu::{include_spirv, util::DeviceExt};

use crate::{path, queue};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Camera {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub forward: [f32; 3],
    pub _pad1: u32,
    pub up: [f32; 3],
    pub _pad2: u32,
    pub dims: [f32; 2],
    pub focal_length: f32,
    pub _pad3: [u32; 2],
}

impl Camera {
    fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            dims: [1.0, 1.0],
            focal_length: 1.0,
            ..Default::default()
        }
    }
}

pub struct NewRayPhase {
    pipeline: wgpu::ComputePipeline,
    camera_uniform: wgpu::Buffer,
    camera_bindgroup_layout: wgpu::BindGroupLayout,
    camera_bindgroup: wgpu::BindGroup,
}

impl NewRayPhase {
    pub fn new(
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        extension_queue: &queue::Queue,
    ) -> Self {
        let compute_shader =
            device.create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/new_ray.spv")));
        // let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        //     label: Some("NewRayPhase"),
        //     source: wgpu::ShaderSource::Wgsl(include_wesl!("new_ray").into()),
        // });

        // Camera initialization:
        let camera_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ExtensionPhase Camera Uniform"),
            contents: bytemuck::bytes_of(&Camera::new()),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let camera_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ExtensionPhase Camera Bindgroup Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ExtensionPhase Camera Bindgroup"),
            layout: &camera_bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NewRayPhase Pipeline Layout"),
            bind_group_layouts: &[
                &path_buffer.path_bind_group_layout,
                &new_ray_queue.bind_group_layout,
                &extension_queue.bind_group_layout,
                &camera_bindgroup_layout,
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

        Self {
            pipeline,
            camera_uniform,
            camera_bindgroup_layout,
            camera_bindgroup,
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        path_buffer: &path::Paths,
        new_ray_queue: &queue::Queue,
        extension_queue: &queue::Queue,
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogicPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &new_ray_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(3, &self.camera_bindgroup, &[]);
        compute_pass.dispatch_workgroups(new_ray_queue.size.div_ceil(64), 1, 1);

        drop(compute_pass);

        encoder.finish()
    }
}
