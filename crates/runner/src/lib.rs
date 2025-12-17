mod blas;
mod bvh;
mod camera;
mod dims;
mod extension;
mod instance;
mod lambertian;
mod logic;
mod material;
mod mesh;
mod metallic;
mod new_ray;
mod path;
mod queue;
mod render;
mod texture;
mod tlas;

use core::f32;
use std::{collections::HashSet, f32::consts::PI, sync::Arc};

use glam::Vec3;
use itertools::Itertools;
use rand::{Rng, random_range};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, PhysicalPosition},
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    blas::BLASData,
    dims::Dims,
    extension::Sphere,
    instance::{Instance, Instances},
    lambertian::LambertianData,
    mesh::Meshes,
    metallic::MetallicData,
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
    metallic_queue: queue::Queue,
    extension_queue: queue::Queue,
    logic_phase: logic::LogicPhase,
    render_phase: render::RenderPhase,
    new_ray_phase: new_ray::NewRayPhase,
    lambertian_phase: lambertian::LambertianPhase,
    metallic_phase: metallic::MetallicPhase,
    extension_phase: extension::ExtensionPhase,
    instances: Instances,
    blas_data: blas::BLASData,
    tlas_data: tlas::TLASData,
    camera: camera::Camera,
    window: Arc<Window>,
    dims: Dims,
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
        limits.max_bind_groups = 6;
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

        let dims = Dims::new(&device, (256, 256));

        // Load models:
        let mut load_options = tobj::GPU_LOAD_OPTIONS;
        load_options.single_index = false;

        let mut meshes = Vec::new();

        // Suzanne!
        let (models, materials) = tobj::load_obj("assets/suzanne.obj", &load_options).unwrap();
        meshes.push(mesh::Mesh::from_model(&models[0].mesh));

        // Teapot!
        let (models, materials) = tobj::load_obj("assets/teapot.obj", &load_options).unwrap();
        meshes.push(mesh::Mesh::from_model(&models[0].mesh));

        // Teapot!
        let (models, materials) = tobj::load_obj("assets/dragon.obj", &load_options).unwrap();
        meshes.push(mesh::Mesh::from_model(&models[0].mesh));

        // Make material data for lambertian:
        let mut lambertian_data = vec![
            // For now, 3 instances, r/g/b each
            LambertianData {
                albedo: [0.9, 0.9, 0.9, 0.0],
            },
            LambertianData {
                albedo: [0.8, 0.8, 0.9, 0.0],
            },
            LambertianData {
                albedo: [0.8, 0.9, 0.8, 0.0],
            },
            LambertianData {
                albedo: [0.9, 0.8, 0.8, 0.0],
            },
        ];
        for _ in 0..10 {
            lambertian_data.push(LambertianData {
                albedo: [0.0, 0.0, 0.0, 0.0].map(|_| random_range(0.0..=1.0)),
            });
        }

        // Make material data for metallics:
        let metallic_data = lambertian_data
            .clone()
            .into_iter()
            .map(|ld| MetallicData {
                albedo: ld.albedo.map(|_| random_range(0.0..=1.0)),
                fuzz: random_range(0.0..=1.0),
                ..Default::default()
            })
            .collect_vec();

        // Instances:
        let mut instances = vec![];
        for x in 0..10 {
            for y in 0..1 {
                for z in 0..10 {
                    let material = random_range(1..=2);
                    let material_idx = match material {
                        1 => random_range(0..lambertian_data.len() as u32),
                        2 => random_range(0..metallic_data.len() as u32),
                        _ => panic!(),
                    };
                    instances.push(Instance {
                        transform: instance::Transform {
                            scale: Vec3::splat(random_range(1.0..=2.0)),
                            rotation: Vec3::ZERO.map(|_| random_range(0.0..2.0 * f32::consts::PI)),
                            translation: Vec3::new(
                                x as f32 * 3.0 - 15.0,
                                y as f32 * 3.0,
                                z as f32 * 3.0 + 5.0,
                            ),
                            ..Default::default()
                        },
                        mesh: random_range(0..meshes.len() as u32),
                        material: material, // Lambertian
                        material_idx: material_idx,
                        ..Default::default()
                    });
                }
            }
        }
        let instances = Instances::new(&device, instances);

        // Make the BLAS & TLAS
        let blases = meshes.into_iter().map(|m| blas::BLAS::new(m)).collect_vec();
        let tlas = tlas::TLAS::new(&blases, &instances.instances);

        let blas_data = blas::BLASData::new(&device, blases);
        let tlas_data = tlas::TLASData::new(&device, tlas);

        // Make a bunch of queues:
        let paths = path::Paths::new(&device, dims.dims);
        let new_ray_queue = queue::Queue::new(&device, dims.size(), Some("NewRayPhase"));
        let extension_queue = queue::Queue::new(&device, dims.size(), Some("ExtensionPhase"));
        let lambertian_queue = queue::Queue::new(&device, dims.size(), Some("LambertianQueue"));
        let metallic_queue = queue::Queue::new(&device, dims.size(), Some("MetallicQueue"));
        let camera = camera::Camera::new(&device, Some("MainCamera"));

        let render_phase = render::RenderPhase::new(&device, &config, dims.dims);
        let logic_phase = logic::LogicPhase::new(
            &device,
            &paths,
            &new_ray_queue,
            &[&lambertian_queue, &metallic_queue],
            &dims,
        );
        let new_ray_phase = new_ray::NewRayPhase::new(
            &device,
            &paths,
            &new_ray_queue,
            &extension_queue,
            &camera,
            &dims,
        );
        let lambertian_phase = lambertian::LambertianPhase::new(
            &device,
            &paths,
            &lambertian_queue,
            &extension_queue,
            lambertian_data,
            Some("Lambertian"),
        );
        let metallic_phase = metallic::MetallicPhase::new(
            &device,
            &paths,
            &metallic_queue,
            &extension_queue,
            metallic_data,
            Some("Metallic"),
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
            &blas_data,
            &tlas_data,
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
            metallic_phase,
            extension_phase,
            paths,
            new_ray_queue,
            lambertian_queue,
            metallic_queue,
            extension_queue,
            instances,
            camera,
            dims,
            keys_pressed: HashSet::new(),
            blas_data,
            tlas_data,
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
            &[&self.lambertian_queue, &self.metallic_queue],
            &self.dims,
        );
        let new_ray_commands = self.new_ray_phase.render(
            &self.device,
            &self.paths,
            &self.new_ray_queue,
            &self.extension_queue,
            &self.camera,
            &self.dims,
        );
        let lambertian_commands = self.lambertian_phase.render(
            &self.device,
            &self.paths,
            &self.lambertian_queue,
            &self.extension_queue,
        );
        let metallic_commands = self.metallic_phase.render(
            &self.device,
            &self.paths,
            &self.metallic_queue,
            &self.extension_queue,
        );
        let extension_commands = self.extension_phase.render(
            &self.device,
            &self.paths,
            &self.extension_queue,
            &self.blas_data,
            &self.tlas_data,
            &self.instances,
        );

        let renderer_commands =
            self.render_phase
                .render(&self.device, &self.logic_phase.output(), &view);

        self.queue.submit([
            logic_commands,
            new_ray_commands,
            metallic_commands,
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
