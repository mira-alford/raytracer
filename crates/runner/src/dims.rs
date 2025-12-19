use wgpu::util::DeviceExt;

pub struct Dims {
    pub dims: (u32, u32),
    pub threads: u32,
    pub buffer: wgpu::Buffer,
    pub bindgroup: wgpu::BindGroup,
    pub bindgroup_layout: wgpu::BindGroupLayout,
}

impl Dims {
    pub fn size(&self) -> u32 {
        self.dims.0 * self.dims.1
    }

    pub fn new(device: &wgpu::Device, dims: (u32, u32), threads: u32) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dims Buffer"),
            contents: bytemuck::cast_slice(&[dims.0, dims.1]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Dims Bindgroup Layout"),
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

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dims Bindgroup"),
            layout: &bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            dims,
            threads,
            buffer,
            bindgroup,
            bindgroup_layout,
        }
    }
}
