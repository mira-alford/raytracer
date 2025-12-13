mod bvh;
mod camera;
mod extension;
mod instance;
mod lambertian;
mod logic;
mod material;
mod mesh;
mod new_ray;
mod path;
mod queue;
mod render;
mod texture;

use std::{collections::HashSet, sync::Arc};

use rand::Rng;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, PhysicalPosition},
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    bvh::BLAS,
    extension::Sphere,
    instance::{Instance, Instances},
    mesh::Meshes,
};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    paths: path::Paths,
    new_ray_queue: queue::Queue,
    lambertian_queue: queue::Queue,
    extension_queue: queue::Queue,
    logic_phase: logic::LogicPhase,
    render_phase: render::RenderPhase,
    new_ray_phase: new_ray::NewRayPhase,
    lambertian_phase: lambertian::LambertianPhase,
    extension_phase: extension::ExtensionPhase,
    instances: Instances,
    blas: BLAS,
    camera: camera::Camera,
    window: Arc<Window>,
    dims: (u32, u32),
    keys_pressed: HashSet<KeyCode>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        // Configure mouse grab:
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .ok();
        window.set_cursor_visible(false);

        // Configure rendering stuff:
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

        let mut limits = wgpu::Limits::defaults();
        limits.max_bind_groups = 5;
        // limits.max_storage_buffer_binding_size = 184549552;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: limits,
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

        let dims = (1024, 1024);

        // Load models:
        let mut load_options = tobj::GPU_LOAD_OPTIONS;
        load_options.single_index = false;

        // Teapot!
        let (models, materials) = tobj::load_obj("assets/teapot.obj", &load_options).unwrap();
        let mesh = mesh::Mesh::from_model(&models[0].mesh);

        // Suzanne!
        let (models, materials) = tobj::load_obj("assets/suzanne.obj", &load_options).unwrap();
        let mesh2 = mesh::Mesh::from_model(&models[0].mesh);

        // Make the BLAS:
        let bvhs = vec![bvh::BVH::new(mesh), bvh::BVH::new(mesh2)];
        let blas = bvh::BLAS::new(&device, bvhs);

        // Instances
        let instances = vec![
            Instance {
                transform: instance::Transform {
                    scale: [1.0, 1.0, 1.0],
                    rotation: [0.0, 0.0, 0.0],
                    translation: [0.0, 0.0, 0.0],
                    ..Default::default()
                },
                mesh: 0,
                material: 1,
                ..Default::default()
            },
            Instance {
                transform: instance::Transform {
                    scale: [1.0, 1.0, 1.0],
                    rotation: [0.0, 0.0, 0.0],
                    translation: [0.0, 5.0, 0.0],
                    ..Default::default()
                },
                mesh: 1,
                material: 2,
                ..Default::default()
            },
        ];
        let instances = Instances::new(&device, instances);

        // Make a bunch of queues:
        let paths = path::Paths::new(&device, dims);
        let new_ray_queue = queue::Queue::new(&device, dims.0 * dims.1, Some("NewRayPhase"));
        let extension_queue = queue::Queue::new(&device, dims.0 * dims.1, Some("ExtensionPhase"));
        let lambertian_queue = queue::Queue::new(&device, dims.0 * dims.1, Some("LambertianQueue"));
        let camera = camera::Camera::new(&device, Some("MainCamera"));

        let render_phase = render::RenderPhase::new(&device, &config, dims);
        let logic_phase =
            logic::LogicPhase::new(&device, &paths, &new_ray_queue, &[&lambertian_queue], dims);
        let new_ray_phase =
            new_ray::NewRayPhase::new(&device, &paths, &new_ray_queue, &extension_queue, &camera);
        let lambertian_phase = lambertian::LambertianPhase::new(
            &device,
            &paths,
            &lambertian_queue,
            &extension_queue,
            Some("Lambertian"),
        );

        let mut rng = rand::rng();
        let mut spheres = (0..4)
            .map(|_| Sphere {
                position: [
                    rng.random_range(-10.0..=10.0),
                    rng.random_range(0.0..=5.0),
                    rng.random_range(0.0..=10.0),
                ],
                radius: rng.random_range(0.001..=1.0),
            })
            .collect::<Vec<_>>();
        spheres.push(Sphere {
            position: [5.0, -10000.0, 3.0],
            // position: [5.0, -10000000.0, 3.0],
            radius: 9999.0,
            ..Default::default()
        });

        let extension_phase = extension::ExtensionPhase::new(
            &device,
            &paths,
            &extension_queue,
            &blas,
            spheres.as_slice(),
            &instances,
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
            lambertian_phase,
            extension_phase,
            paths,
            new_ray_queue,
            lambertian_queue,
            extension_queue,
            instances,
            camera,
            dims,
            keys_pressed: HashSet::new(),
            blas,
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

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => {
                event_loop.exit();
            }
            _ => {}
        }

        if is_pressed {
            self.keys_pressed.insert(code);
        } else {
            self.keys_pressed.remove(&code);
        }
    }

    fn handle_mouse(&mut self, event_loop: &ActiveEventLoop, position: PhysicalPosition<f64>) {}

    fn handle_mouse_motion(&mut self, event_loop: &ActiveEventLoop, delta: (f32, f32)) {
        self.camera.rotate(delta);
    }

    fn update(&mut self) {
        const MOVE_SPEED: f32 = 0.01;
        for key in &self.keys_pressed {
            match key {
                KeyCode::KeyW => {
                    self.camera.translate((0.0, 0.0, MOVE_SPEED));
                }
                KeyCode::KeyA => {
                    self.camera.translate((-MOVE_SPEED, 0.0, 0.0));
                }
                KeyCode::KeyS => {
                    self.camera.translate((0.0, 0.0, -MOVE_SPEED));
                }
                KeyCode::KeyD => {
                    self.camera.translate((MOVE_SPEED, 0.0, 0.0));
                }
                KeyCode::Space => {
                    self.camera.translate((0.0, MOVE_SPEED, 0.0));
                }
                KeyCode::ControlLeft => {
                    self.camera.translate((0.0, -MOVE_SPEED, 0.0));
                }
                _ => {}
            };
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // Game logic stuff
        self.update();

        // Updating any buffers
        self.camera.update(&self.queue);

        // Rendering:
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
            &[&self.lambertian_queue],
            self.dims,
        );
        let new_ray_commands = self.new_ray_phase.render(
            &self.device,
            &self.paths,
            &self.new_ray_queue,
            &self.extension_queue,
            &self.camera,
        );
        let lambertian_commands = self.lambertian_phase.render(
            &self.device,
            &self.paths,
            &self.lambertian_queue,
            &self.extension_queue,
        );
        let extension_commands = self.extension_phase.render(
            &self.device,
            &self.paths,
            &self.extension_queue,
            &self.blas,
            &self.instances,
        );

        let renderer_commands =
            self.render_phase
                .render(&self.device, &self.logic_phase.output(), &view);

        self.queue.submit([
            logic_commands,
            new_ray_commands,
            lambertian_commands,
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

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            winit::event::DeviceEvent::MouseMotion { delta: (x, y) } => {
                const MOUSE_SENSITIVITY: f32 = 0.001;
                state.handle_mouse_motion(
                    event_loop,
                    (x as f32 * MOUSE_SENSITIVITY, y as f32 * MOUSE_SENSITIVITY),
                );
            }
            _ => {}
        }
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
                state.render().ok();
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
