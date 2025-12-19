use bytemuck::Zeroable;
use itertools::Itertools;
use rand::{Rng, seq::SliceRandom};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SampleData {
    pub screen_pos: [f32; 2],
    pub out_pos: [u32; 2],
    pub samples: u32,
    pub flags: u32,
}

pub struct Samples {
    pub counter_buffer: wgpu::Buffer,
    pub data_buffer: wgpu::Buffer,
    pub mean_buffer: wgpu::Buffer,
    pub std_buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Samples {
    pub fn new(device: &wgpu::Device, dims: (u32, u32)) -> Self {
        let counter_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sample Counter Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::bytes_of(&[0u32, 0u32]),
        });

        // let data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("Sample Data Buffer"),
        //     size: ((dims.0 * dims.1) as u64 * std::mem::size_of::<SampleData>() as u64),
        //     usage: wgpu::BufferUsages::STORAGE,
        //     mapped_at_creation: false,
        // });

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
                    .map(|(x, y)| SampleData {
                        screen_pos: [x as f32 / dims.0 as f32, y as f32 / dims.1 as f32],
                        out_pos: [x, y],
                        samples: 0,
                        flags: 0,
                    })
            })
            .collect_vec();
        data.shuffle(&mut rand::rng());
        // data.sort_by_key(|d| (d.out_pos[0] / 256, d.out_pos[1] / 256));

        let data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sample Data Buffer"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let mean_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sample Mean Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: ((dims.0 * dims.1) as u64 * std::mem::size_of::<[f32; 4]>() as u64),
            mapped_at_creation: false,
        });

        let std_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sample Std Buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: ((dims.0 * dims.1) as u64 * std::mem::size_of::<[f32; 4]>() as u64),
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sample Bind Group Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sample Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mean_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: std_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            bind_group_layout,
            bind_group,
            counter_buffer,
            data_buffer,
            mean_buffer,
            std_buffer,
        }
    }
}
