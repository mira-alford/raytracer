mod extension;
mod logic;
mod new_ray;
mod path;
mod queue;
mod render;
mod texture;

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::extension::Sphere;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    paths: path::Paths,
    new_ray_queue: queue::Queue,
    extension_queue: queue::Queue,
    logic_phase: logic::LogicPhase,
    render_phase: render::RenderPhase,
    new_ray_phase: new_ray::NewRayPhase,
    extension_phase: extension::ExtensionPhase,
    window: Arc<Window>,
    dims: (u32, u32),
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let x = wgpu::WgslLanguageFeatures::UnrestrictedPointerParameters;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

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

        let dims = (512, 512);

        let paths = path::Paths::new(&device, dims);
        let new_ray_queue = queue::Queue::new(&device, dims.0 * dims.1, Some("NewRayPhase"));
        let extension_queue = queue::Queue::new(&device, dims.0 * dims.1, Some("ExtensionPhase"));

        let render_phase = render::RenderPhase::new(&device, &config, dims);
        let logic_phase = logic::LogicPhase::new(&device, &paths, &new_ray_queue, &[], dims);
        let new_ray_phase =
            new_ray::NewRayPhase::new(&device, &paths, &new_ray_queue, &extension_queue, dims);
        let extension_phase = extension::ExtensionPhase::new(
            &device,
            &paths,
            &extension_queue,
            &[Sphere {
                position: [0.0, 0.0, 4.0],
                radius: 1.0,
            }],
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            logic_phase,
            render_phase,
            new_ray_phase,
            extension_phase,
            paths,
            new_ray_queue,
            extension_queue,
            dims,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        };
    }

    fn handle_mouse(&mut self, event_loop: &ActiveEventLoop, position: PhysicalPosition<f64>) {}

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let logic_commands = self.logic_phase.render(
            &self.device,
            &self.paths,
            &self.new_ray_queue,
            &[],
            self.dims,
        );
        let new_ray_commands = self.new_ray_phase.render(
            &self.device,
            &self.paths,
            &self.new_ray_queue,
            &self.extension_queue,
        );
        let extension_commands =
            self.extension_phase
                .render(&self.device, &self.paths, &self.extension_queue);

        let renderer_commands =
            self.render_phase
                .render(&self.device, &self.logic_phase.output(), &view);

        self.queue.submit([
            logic_commands,
            new_ray_commands,
            extension_commands,
            renderer_commands,
        ]);

        output.present();

        Ok(())
    }
}

pub struct App {
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(State::new(window))
                .unwrap(),
        );
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: State) {
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => state.handle_mouse(event_loop, position),
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
