use std::sync::Arc;

use crate::{app::BevyApp, schedule, winnit::WinitWindow};

use bevy_ecs::prelude::*;
use winit::dpi::PhysicalSize;

#[derive(Resource, Clone)]
pub struct RenderDevice(pub Arc<wgpu::Device>);

#[derive(Resource, Clone)]
pub struct RenderQueue(pub Arc<wgpu::Queue>);

#[derive(Resource, Clone, Debug)]
pub struct RenderAdapter(pub Arc<wgpu::Adapter>);

#[derive(Resource, Clone)]
pub struct RenderInstance(pub Arc<wgpu::Instance>);

#[derive(Resource, Clone)]
pub struct RenderAdapterInfo(pub wgpu::AdapterInfo);

#[derive(Resource, Clone)]
pub struct RenderSurface {
    pub size: PhysicalSize<u32>,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: Arc<wgpu::Surface<'static>>,
    pub is_surface_configured: bool,
}

pub fn initialize(app: &mut BevyApp) {
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::PreStartup, setup_renderer);
}

fn setup_renderer(mut commands: Commands, window: Option<Res<WinitWindow>>) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Configure rendering stuff:
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let surface = window
        .as_ref()
        .map(|w| instance.create_surface(w.0.clone()).unwrap());

    let adapter = rt
        .block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            force_fallback_adapter: false,
        }))
        .unwrap();

    let mut limits = wgpu::Limits::defaults();
    limits.max_bind_groups = 8;
    limits.max_storage_buffer_binding_size = 402653184;
    limits.max_buffer_size = 402653184;
    let required_features = wgpu::Features::empty()
        .union(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
        .union(wgpu::Features::BUFFER_BINDING_ARRAY)
        .union(wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY);

    let (device, queue) = rt
        .block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: limits,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

    if let (Some(surface), Some(window)) = (surface, window) {
        let size = window.0.inner_size();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        commands.insert_resource(RenderSurface {
            size,
            config,
            surface: Arc::new(surface),
            is_surface_configured: false,
        });
    }

    commands.insert_resource(RenderAdapterInfo(adapter.get_info()));
    commands.insert_resource(RenderAdapter(Arc::new(adapter)));
    commands.insert_resource(RenderInstance(Arc::new(instance)));
    commands.insert_resource(RenderQueue(Arc::new(queue)));
    commands.insert_resource(RenderDevice(Arc::new(device)));
}
