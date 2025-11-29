use wesl::include_wesl;
use wgpu::util::DeviceExt;

use crate::{path, queue};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
}

pub struct ExtensionPhase {
    pipeline: wgpu::ComputePipeline,
    primitives_buffer: wgpu::Buffer,
    primitives_bindgroup_layout: wgpu::BindGroupLayout,
    primitives_bindgroup: wgpu::BindGroup,
}

impl ExtensionPhase {
    pub fn new(
        device: &wgpu::Device,
        paths: &path::Paths,
        extension_queue: &queue::Queue,
        primitives: &[Sphere],
    ) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ExtensionPhase"),
            source: wgpu::ShaderSource::Wgsl(include_wesl!("extension").into()),
        });

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
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ExtensionPhase Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            pipeline,
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
    ) -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ExtensionPhase Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &path_buffer.path_bind_group, &[]);
        compute_pass.set_bind_group(1, &extension_queue.bind_group, &[]);
        compute_pass.set_bind_group(2, &self.primitives_bindgroup, &[]);
        compute_pass.dispatch_workgroups(extension_queue.size.div_ceil(64), 1, 1);
        drop(compute_pass);

        encoder.finish()
    }
}

// pub struct GeneratePhase {
//     camera_uniform: wgpu::Buffer,
//     primitive_buffer: wgpu::Buffer,
//     bind_group: wgpu::BindGroup,
//     pipeline: wgpu::ComputePipeline,
//     dims: (u32, u32),
//     spheres: Vec<Sphere>,
// }

// impl GeneratePhase {
//     pub fn new(device: &wgpu::Device) -> Self {
//         let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
//             label: Some("GenerationPhase"),
//             source: wgpu::ShaderSource::Wgsl(include_str!("extend.wgsl").into()),
//         });

//         let primitive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Ray Buffer"),
//             usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//             size: 0,
//             mapped_at_creation: false,
//         });

//         let primitive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Ray Buffer"),
//             usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//             size: 0,
//             mapped_at_creation: false,
//         });

//         let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             label: Some("GenerationPhase Bindgroup Layout"),
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::COMPUTE,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Storage { read_only: false },
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::COMPUTE,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Uniform,
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 },
//             ],
//         });

//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label: None,
//             layout: &bind_group_layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: primitive_buffer.as_entire_binding(),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: camera_uniform.as_entire_binding(),
//                 },
//             ],
//         });

//         let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: Some("GeneratePhase Pipleline Layout"),
//             bind_group_layouts: &[&bind_group_layout],
//             push_constant_ranges: &[],
//         });

//         let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
//             label: Some("GeneratePhase Pipeline"),
//             layout: Some(&pipeline_layout),
//             module: &compute_shader,
//             entry_point: Some("cs_main"),
//             compilation_options: Default::default(),
//             cache: Default::default(),
//         });

//         Self {
//             camera_uniform,
//             primitive_buffer,
//             bind_group,
//             pipeline,
//             dims,
//             camera: Camera::zeroed(),
//         }
//     }

//     pub fn render(&self, device: &wgpu::Device) -> wgpu::CommandBuffer {
//         let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some("GeneratePhase Encoder"),
//         });

//         let mut compute_pass = encoder.begin_compute_pass(&Default::default());
//         compute_pass.set_pipeline(&self.pipeline);
//         compute_pass.set_bind_group(0, &self.bind_group, &[]);
//         compute_pass.dispatch_workgroups(self.dims.0.div_ceil(64), self.dims.1, 1);
//         drop(compute_pass);

//         encoder.finish()
//     }
// }
