use bevy_ecs::component::Component;
use bytemuck::Zeroable;
use glam::{UVec4, Vec4};
use itertools::Itertools;
use rand::{Rng, seq::SliceRandom};
use wgpu::util::DeviceExt;

use crate::queue;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: Vec4,
    pub uv: Vec4,
    pub norm: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable)]
pub struct HitRecord {
    pub vert: Vertex,
    pub triangle_id: UVec4,
    pub instance_id: u32,
    pub front_face: u32,
    pub _pad: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Ray {
    pub position: Vec4,
    pub direction: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Sample {
    pub radiance: [f32; 3],
    pub _pad0: u32, // pad to 16 byte boundary
    pub throughput: [f32; 3],
    pub _pad1: u32, // pad to 16 byte boundary
    pub bounces: u32,
    pub sample_id: u32,
    pub _pad2: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RandomState {
    pub random_state: [u32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShadowData {
    pub dir: [f32; 3],
    pub _pad1: u32,
    pub rad: [f32; 3],
    pub _pad2: u32,
    pub prob: f32,
    pub _pad3: [u32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SampleSource {
    pub screen_pos: [f32; 2],
    pub out_pos: [u32; 2],
    pub samples: u32,
    pub flags: u32,
}

#[derive(Component)]
pub struct PathtracerState {
    // Path tracer intermediate state:
    pub path_buffer: wgpu::Buffer,
    pub random_state_buffer: wgpu::Buffer,
    pub shadow_data_buffer: wgpu::Buffer,
    pub hit_data_buffer: wgpu::Buffer,
    // Sampling intermediate buffers:
    pub sampling_counter_buffer: wgpu::Buffer,
    pub sampling_data_buffer: wgpu::Buffer,
    pub sampling_mean_buffer: wgpu::Buffer,
    pub sampling_std_buffer: wgpu::Buffer,

    // Queues:
    pub new_ray_queue: queue::Queue,
    pub extension_queue: queue::Queue,
    pub shadow_queue: queue::Queue,
    pub material_queue: queue::Queue,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl PathtracerState {
    pub fn new(device: &wgpu::Device, dims: (u32, u32), threads: u32) -> Self {
        let mut rng = rand::rng();
        let samples: Vec<_> = (0..=threads).map(|_| Sample::zeroed()).collect();

        let random_states: Vec<_> = (0..=threads)
            .map(|_| RandomState {
                random_state: [
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                    rng.random_range(1000..=u32::MAX),
                ],
            })
            .collect();

        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Path Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&samples),
        });

        let random_state_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Random State Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&random_states),
        });

        let extension_rays_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hit Data Buffer"),
            size: (threads as u64 * std::mem::size_of::<Ray>() as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let connect_rays_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hit Data Buffer"),
            size: (threads as u64 * std::mem::size_of::<Ray>() as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let extension_hit_records_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hit Data Buffer"),
            size: (threads as u64 * std::mem::size_of::<HitRecord>() as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let connect_hit_records_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hit Data Buffer"),
            size: (threads as u64 * std::mem::size_of::<HitRecord>() as u64),
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let shadow_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Data Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: std::mem::size_of::<ShadowData>() as u64 * threads as u64,
            mapped_at_creation: false,
        });

        dbg!(dims);
        dbg!(threads);
        let dims_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dims Buffer"),
            contents: bytemuck::cast_slice(&[dims.0, dims.1]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Sampling buffers:
        let sampling_counter_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sample Counter Buffer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: bytemuck::bytes_of(&[0u32, 0u32]),
            });

        let tile_size = 128;
        let mut data = (0..dims.0 / tile_size)
            .cartesian_product(0..dims.1 / tile_size)
            .collect_vec();

        data.shuffle(&mut rand::rng());

        let mut data = data
            .into_iter()
            .flat_map(|(x, y)| {
                ((x * tile_size)..(x * tile_size + tile_size))
                    .cartesian_product((y * tile_size)..(y * tile_size + tile_size))
                    .map(|(x, y)| SampleSource {
                        screen_pos: [x as f32 / dims.0 as f32, y as f32 / dims.1 as f32],
                        out_pos: [x, y],
                        samples: 0,
                        flags: 0,
                    })
            })
            .collect_vec();
        data.shuffle(&mut rand::rng());
        // data.sort_by_key(|d| (d.out_pos[0] / 256, d.out_pos[1] / 256));

        let sampling_source_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sample Data Buffer"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let sampling_sum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sample Mean Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: ((dims.0 * dims.1) as u64 * std::mem::size_of::<[f32; 4]>() as u64),
            mapped_at_creation: false,
        });

        let sampling_std_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sample Std Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: ((dims.0 * dims.1) as u64 * std::mem::size_of::<[f32; 4]>() as u64),
            mapped_at_creation: false,
        });

        let terminate_queue = queue::Queue::new(&device, threads, Some("Terminate Queue"), true);
        let extension_queue = queue::Queue::new(&device, threads, Some("Extension Queue"), false);
        let shade_queue = queue::Queue::new(&device, threads, Some("Shade Queue"), false);
        let connect_queue = queue::Queue::new(&device, threads, Some("Connect Queue"), false);

        let mut bgles = (0..18)
            .map(|i| wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect_vec();
        bgles.push(wgpu::BindGroupLayoutEntry {
            binding: 18,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pathtracer State Bind Group Layout"),
            entries: &bgles,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pathtracer State Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                // Sample instance state + rays + hits:
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sample_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: extension_rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: connect_rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: extension_hit_records_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: connect_hit_records_buffer.as_entire_binding(),
                },
                // Random:
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: random_state_buffer.as_entire_binding(),
                },
                // Sampling:
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: sampling_counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: sampling_source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: sampling_sum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: sampling_std_buffer.as_entire_binding(),
                },
                // All the queues ever:
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: extension_queue.counter_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: extension_queue.queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: connect_queue.counter_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: connect_queue.queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: terminate_queue.counter_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 15,
                    resource: terminate_queue.queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 16,
                    resource: shade_queue.counter_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 17,
                    resource: shade_queue.queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 18,
                    resource: dims_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            path_buffer: sample_buffer,
            random_state_buffer,
            hit_data_buffer: extension_hit_records_buffer,
            shadow_data_buffer,
            sampling_counter_buffer,
            sampling_data_buffer: sampling_source_buffer,
            sampling_mean_buffer: sampling_sum_buffer,
            sampling_std_buffer,
            new_ray_queue: terminate_queue,
            extension_queue,
            shadow_queue: connect_queue,
            material_queue: shade_queue,
            bind_group_layout,
            bind_group,
        }
    }
}
