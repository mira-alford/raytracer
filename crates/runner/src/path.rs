#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable)]
pub struct Path {
    pub position: [f32; 3],
    pub _pad0: u32, // pad vec3 to 16 bytes
    pub direction: [f32; 3],
    pub _pad1: u32, // pad vec3 to 16 bytes
    pub terminated: u32,
    pub generated: u32,
    pub _pad2: u64, // pad to 16 byte boundary
    pub radiance: [f32; 3],
    pub _pad3: u32, // Pad the entire struct to 16 byte boundary
}

pub struct Paths {
    pub path_buffer: wgpu::Buffer,
    pub path_bind_group_layout: wgpu::BindGroupLayout,
    pub path_bind_group: wgpu::BindGroup,
}

impl Paths {
    pub fn new(device: &wgpu::Device, dims: (u32, u32)) -> Self {
        let path_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Path Buffer"),
            size: (dims.0 * dims.1 * std::mem::size_of::<Path>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let path_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Path Bind Group Layout"),
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

        let path_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Path Bind Group"),
            layout: &path_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: path_buffer.as_entire_binding(),
            }],
        });

        Self {
            path_buffer,
            path_bind_group_layout,
            path_bind_group,
        }
    }
}
