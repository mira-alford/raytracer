use glam::Vec3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Transform {
    pub scale: Vec3,
    pub _pad0: [u32; 1],
    pub rotation: Vec3,
    pub _pad1: [u32; 1],
    pub translation: Vec3,
    pub _pad2: [u32; 1],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Instance {
    /// Each instance transforms that mesh
    pub transform: Transform,
    /// Each instance points at a mesh
    pub mesh: u32,
    /// Each instance has its own material
    pub material: u32,
    pub material_idx: u32,
    pub pad_0: [u32; 1], // to make the overall struct 16 byte aligned??
}

pub struct Instances {
    pub instances: Vec<Instance>,
    pub bindgroup_layout: wgpu::BindGroupLayout,
    pub bindgroup: wgpu::BindGroup,
}

impl Instances {
    pub fn new(device: &wgpu::Device, instances: Vec<Instance>) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Instance bindgroup layout descriptor"),
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

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Instance bindgroup"),
            layout: &bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            instances,
            bindgroup_layout,
            bindgroup,
        }
    }
}
