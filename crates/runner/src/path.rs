use bytemuck::Zeroable;
use rand::Rng;
use wgpu::util::DeviceExt;

use crate::dims::Dims;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Hit {
    pub position: [f32; 3],
    pub _pad0: u32, // pad vec3 to 16 bytes
    pub normal: [f32; 3],
    pub _pad1: u32, // pad vec3 to 16 bytes
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Ray {
    pub position: [f32; 3],
    pub _pad0: u32, // pad vec3 to 16 bytes
    pub direction: [f32; 3],
    pub _pad1: u32, // pad vec3 to 16 bytes
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Path {
    pub ray: Ray,
    pub radiance: [f32; 3],
    pub _pad2: u32, // pad to 16 byte boundary
    pub throughput: [f32; 3],
    pub _pad3: u32, // pad to 16 byte boundary
    pub terminated: u32,
    pub sampled: u32,
    pub bounces: u32,
    pub sample_id: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SamplingState {
    pub sampled_radiance: [f32; 3], // multi sample radiance
    pub _pad1: u32,                 // pad to 16 byte boundary
    pub sampled_pos: [f32; 3],      // last sampled pos for reset check
    pub _pad2: u32,                 // pad to 16 byte boundary
    pub samples: u32,               // multi sample count
    pub _pad3: [u32; 3],            // pad to 16 byte boundary
}

pub struct HitData {
    pub hit: Hit,
    pub mat: u32,
    pub mat_data: u32,
    pub _pad3: [u32; 2], // pad to 16 byte boundary
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RandomState {
    pub random_state: [u32; 4],
}

pub struct Paths {
    pub path_buffer: wgpu::Buffer,
    pub random_state_buffer: wgpu::Buffer,
    // pub sample_state_buffer: wgpu::Buffer,
    pub hit_data_buffer: wgpu::Buffer,
    pub path_bind_group_layout: wgpu::BindGroupLayout,
    pub path_bind_group: wgpu::BindGroup,
}

impl Paths {
    pub fn new(device: &wgpu::Device, dims: &Dims) -> Self {
        let mut rng = rand::rng();
        let paths: Vec<_> = (0..=(dims.threads)).map(|_| Path::zeroed()).collect();

        let random_states: Vec<_> = (0..=(dims.threads))
            .map(|_| RandomState {
                random_state: [
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                ],
            })
            .collect();

        let path_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Path Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&paths),
        });

        let random_state_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("random_state Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&random_states),
        });

        let hit_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit_data Buffer"),
            size: ((dims.threads) as u64 * std::mem::size_of::<HitData>() as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let path_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Path Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let path_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Path Bind Group"),
            layout: &path_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: path_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: random_state_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hit_data_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            path_buffer,
            random_state_buffer,
            // sample_state_buffer,
            hit_data_buffer,
            path_bind_group_layout,
            path_bind_group,
        }
    }
}
