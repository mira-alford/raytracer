use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraData {
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

impl CameraData {
    pub fn new() -> Self {
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

pub struct Camera {
    pub uniform: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Camera {
    pub fn new(device: &wgpu::Device, label: Option<&str>) -> Self {
        let label = label.unwrap_or_default();

        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Camera Uniform", label)),
            contents: bytemuck::bytes_of(&CameraData::new()),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{} Camera Bindgroup Layout", label)),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{} Camera Bindgroup", label)),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            bind_group,
            bind_group_layout,
        }
    }
}
