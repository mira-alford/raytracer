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
    pub changed: u32,
    pub _pad3: [u32; 1],
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
    pub data: CameraData,
    pub uniform: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub changed: bool,
}

impl Camera {
    pub fn new(device: &wgpu::Device, label: Option<&str>) -> Self {
        let label = label.unwrap_or_default();

        let camera_data = CameraData {
            position: [-3.8, 0.4, 6.0],
            forward: [0.55, -0.59, 0.66],
            up: [0.31, 0.86, 0.38],
            dims: [1.0, 1.0],
            focal_length: 1.0,
            changed: 0,
            ..Default::default()
        };

        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Camera Uniform", label)),
            contents: bytemuck::bytes_of(&camera_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
            data: camera_data,
            uniform,
            bind_group,
            bind_group_layout,
            changed: false,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.changed {
            queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&self.data));
            queue.submit([]);
            self.data.changed = 0;
        }
    }

    pub fn translate(&mut self, dir: impl Into<glam::Vec3>) {
        let dir = dir.into();
        let f = glam::Vec3::from(self.data.forward);
        let u = glam::Vec3::from(self.data.up);
        let r = u.cross(f).normalize();
        let mut pos = glam::Vec3::from_slice(&self.data.position);

        pos += dir.x * r;
        pos += dir.y * u;
        pos += dir.z * f;

        self.data.position = pos.to_array();
        self.data.changed = 1;
        self.changed = true;
    }

    pub fn rotate(&mut self, delta: impl Into<glam::Vec2>) {
        let delta = delta.into();
        let f = glam::Vec3::from(self.data.forward).normalize();
        let u = glam::Vec3::from(self.data.up).normalize();

        // Rotating about y axis for left/right is easy:
        let m = glam::Mat3::from_rotation_y(delta.x);
        let f = m * f;
        let u = m * u;

        // To rotate about r for up/down, we must rebase
        // so that r is x axis.
        let r = u.cross(f);
        let m = glam::Mat3::from_axis_angle(r, delta.y);
        let f = m * f;
        let u = m * u;

        self.data.forward = f.into();
        self.data.up = u.into();

        self.data.changed = 1;
        self.changed = true;
    }
}
