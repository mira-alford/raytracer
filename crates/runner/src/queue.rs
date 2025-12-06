use wgpu::util::DeviceExt;

pub struct Queue {
    pub counter_uniform: wgpu::Buffer,
    pub queue_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub size: u32,
}

impl Queue {
    pub fn new(device: &wgpu::Device, size: u32, label: Option<&str>) -> Self {
        // Create the atomic uniform, initially 0
        let counter_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::bytes_of(&[0u32, 0u32]),
        });

        // Create the queue buffer, fits size u32s
        let queue_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            usage: wgpu::BufferUsages::STORAGE,
            size: (size as u64 * std::mem::size_of::<u32>() as u64),
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                // Atomic counter for queue:
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
                // The queue itself:
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
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: counter_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queue_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            bind_group,
            bind_group_layout,
            counter_uniform,
            queue_buffer,
            size,
        }
    }
}
