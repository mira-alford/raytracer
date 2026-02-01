use bevy_ecs::prelude::*;
use wgpu::util::DeviceExt;

use crate::{
    app::BevyApp, camera::Camera, pathtracer_state::PathtracerState,
    render_resources::RenderDevice, schedule,
};

#[derive(Component)]
pub struct Pathtracer {
    pub is_primary: bool,
    pub dims: (u32, u32),
    pub threads: u32,
}

#[derive(Component)]
pub struct PathtracerOutput {
    pub source_bind_group_layout: wgpu::BindGroupLayout,
    pub source_bind_group: wgpu::BindGroup,
    pub source_buffer: wgpu::Buffer,
    pub out_texture: wgpu::Texture,
    pub out_sampler: wgpu::Sampler,
}

pub fn initialize(app: &mut BevyApp) {
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::Startup, setup_pathtracer)
        .add_systems(schedule::Update, pathtracer_output_sync_system);
}

fn setup_pathtracer(mut commands: Commands, device: Res<RenderDevice>) {
    commands.spawn((
        Pathtracer {
            is_primary: true,
            dims: (512, 512),
            threads: 512 * 512,
        },
        Camera::new(&device.0, Some("Camera")),
    ));
}

pub fn pathtracer_output_sync_system(
    mut commands: Commands,
    device: Res<RenderDevice>,
    query: Query<(Entity, &Pathtracer), Changed<Pathtracer>>,
) {
    for (id, pt) in query.iter() {
        commands
            .entity(id)
            .insert(PathtracerOutput::new(&device.0, pt.dims))
            .insert(PathtracerState::new(&device.0, pt.dims, pt.threads));
    }
}

impl PathtracerOutput {
    fn new(device: &wgpu::Device, dims: (u32, u32)) -> Self {
        let source_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LogicPhase Output"),
            contents: bytemuck::cast_slice(&(0..(dims.0 * dims.1)).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        });

        let size = wgpu::Extent3d {
            width: dims.0,
            height: dims.1,
            depth_or_array_layers: 1,
        };

        let out_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let out_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let source_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Output Bind Group Layout"),
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

        let source_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Output Bind Group"),
            layout: &source_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: source_buffer.as_entire_binding(),
            }],
        });

        Self {
            source_bind_group_layout,
            source_bind_group,
            source_buffer,
            out_texture,
            out_sampler,
        }
    }

    pub fn copy_to_texture(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfoBase {
                buffer: &self.source_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.out_texture.size().width),
                    rows_per_image: Some(self.out_texture.size().height),
                },
            },
            wgpu::TexelCopyTextureInfoBase {
                texture: &self.out_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.out_texture.size().width,
                height: self.out_texture.size().height,
                depth_or_array_layers: 1,
            },
        );
    }
}
