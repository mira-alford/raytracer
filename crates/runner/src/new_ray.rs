use wesl::include_wesl;
use wgpu::util::DeviceExt;

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
    pub right: [f32; 3],
    pub _pad3: u32,
    pub focal_length: f32,
    pub logical_half_width: u32,
    pub logical_half_height: u32,
    pub world_half_width: f32,
    pub world_half_height: f32,
    pub _pad4: [u32; 3],
}

impl Camera {
    fn new(dims: (u32, u32)) -> Self {
        Self {
            position: Default::default(),
            forward: Default::default(),
            up: Default::default(),
            right: Default::default(),
            focal_length: 1.0,
            logical_half_width: dims.0 / 2,
            logical_half_height: dims.1 / 2,
            world_half_width: 1.0,
            world_half_height: 1.0,
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
        dims: (u32, u32),
    ) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("NewRayPhase"),
            source: wgpu::ShaderSource::Wgsl(include_wesl!("new_ray").into()),
        });

        // Camera initialization:
        let camera_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ExtensionPhase Camera Uniform"),
            contents: bytemuck::bytes_of(&Camera::new(dims)),
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
            entry_point: Some("cs_main"),
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
