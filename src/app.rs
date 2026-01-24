use bevy_ecs::{prelude::*, schedule::ScheduleLabel};

use crate::schedule;

pub struct BevyApp {
    pub world: World,
    pub startup_has_run: bool,
}

impl BevyApp {
    pub fn new() -> Self {
        let world = World::new();

        Self {
            world,
            startup_has_run: false,
        }
    }

    pub fn run(&mut self) {
        if !self.startup_has_run {
            self.world.run_schedule(schedule::PreStartup);
            self.world.run_schedule(schedule::Startup);
            self.startup_has_run = true;
        }

        self.world.run_schedule(schedule::Update);
    }
}

// #[derive(Resource)]
// pub struct State {
//     surface: wgpu::Surface<'static>,
//     device: wgpu::Device,
//     queue: wgpu::Queue,
//     config: wgpu::SurfaceConfiguration,
//     is_surface_configured: bool,
//     paths: path::Paths,
//     samples: sample::Samples,
//     new_ray_queue: queue::Queue,
//     extension_queue: queue::Queue,
//     material_queues: Vec<queue::Queue>,
//     material_phases: Vec<material::Material>,
//     logic_phase: logic::LogicPhase,
//     render_phase: render::RenderPhase,
//     new_ray_phase: new_ray::NewRayPhase,
//     extension_phase: extension::ExtensionPhase,
//     shadow_phase: shadow::ShadowPhase,
//     instances: Instances,
//     blas_data: blas::BLASData,
//     tlas_data: TLASData,
//     camera: camera::Camera,
//     window: Arc<Window>,
//     dims: Dims,
//     keys_pressed: HashSet<KeyCode>,
//     // TODO: Abstract:
//     light_sample_bindgroup: wgpu::BindGroup,
//     light_sample_bindgroup_layout: wgpu::BindGroupLayout,
//     shadow_queue: queue::Queue,
// }

// #[derive(Resource)]
// pub struct WGPUHandles {
//     adapter: wgpu::Adapter,
//     instance: wgpu::Instance,
//     surface: wgpu::Surface<'static>,
//     surface_caps: wgpu::SurfaceCapabilities,
//     surface_format: wgpu::TextureFormat,
//     config: wgpu::SurfaceConfiguration,
// }

// pub fn winit_plugin(app: BevyApp) {}

// impl State {
//     pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
//         // Configure mouse grab:

//         let dims = Dims::new(&device, (1024, 1024), 1024 * 1024);
//         let mut sb = SceneBuilder::new();
//         boxes_scene(&mut sb);
//         // cornell_scene(&mut sb);
//         // grid_scene(&mut sb);
//         // windows_scene(&mut sb);

//         let Scene {
//             lambertian_data,
//             metallic_data,
//             dielectric_data,
//             emissive_data,
//             instances,
//             blas_data,
//             tlas_data,
//             light_samples,
//         } = sb.build(&device);

//         // TODO: Abstract this away pls
//         let light_sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Light Sample Buff"),
//             contents: bytemuck::cast_slice(&light_samples),
//             usage: BufferUsages::STORAGE,
//         });

//         let light_sample_bindgroup_layout =
//             device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//                 label: Some("Light Sample Bindgroup Layout"),
//                 entries: &[wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::COMPUTE,
//                     ty: wgpu::BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Storage { read_only: true },
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 }],
//             });

//         let light_sample_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label: Some("Light Sample Bindgroup"),
//             layout: &light_sample_bindgroup_layout,
//             entries: &[wgpu::BindGroupEntry {
//                 binding: 0,
//                 resource: light_sample_buffer.as_entire_binding(),
//             }],
//         });

//         // Make a bunch of queues:
//         let paths = path::Paths::new(&device, &dims);
//         let new_ray_queue = queue::Queue::new(&device, dims.threads, Some("NewRayPhase"));
//         let extension_queue = queue::Queue::new(&device, dims.threads, Some("ExtensionPhase"));
//         let shadow_queue = queue::Queue::new(&device, dims.threads, Some("ShadowPhase"));
//         let material_queues = vec![
//             queue::Queue::new(&device, dims.threads, Some("LambertianQueue")),
//             queue::Queue::new(&device, dims.threads, Some("MetallicQueue")),
//             queue::Queue::new(&device, dims.threads, Some("DielectricQueue")),
//             queue::Queue::new(&device, dims.threads, Some("EmissiveQueue")),
//         ];

//         let mut camera = camera::Camera::new(&device, Some("MainCamera"));

//         // Sample States
//         let samples = Samples::new(&device, dims.dims);

//         let render_phase = render::RenderPhase::new(&device, &config, dims.dims);
//         let logic_phase = logic::LogicPhase::new(
//             &device,
//             &paths,
//             &samples,
//             &camera,
//             &new_ray_queue,
//             material_queues.as_slice(),
//             &dims,
//         );
//         let new_ray_phase = new_ray::NewRayPhase::new(
//             &device,
//             &paths,
//             &samples,
//             &new_ray_queue,
//             &extension_queue,
//             &camera,
//             &dims,
//         );

//         let material_phases = vec![
//             material::Material::new(
//                 &device,
//                 device.create_shader_module(include_spirv!(concat!(
//                     env!("OUT_DIR"),
//                     "/lambertian.spv"
//                 ))),
//                 &paths,
//                 &material_queues[0],
//                 &extension_queue,
//                 &instances,
//                 &lambertian_data,
//                 &blas_data,
//                 &tlas_data,
//                 &light_sample_bindgroup_layout,
//                 Some("lambertian"),
//             ),
//             material::Material::new(
//                 &device,
//                 device.create_shader_module(include_spirv!(concat!(
//                     env!("OUT_DIR"),
//                     "/metallic.spv"
//                 ))),
//                 &paths,
//                 &material_queues[1],
//                 &extension_queue,
//                 &instances,
//                 &metallic_data,
//                 &blas_data,
//                 &tlas_data,
//                 &light_sample_bindgroup_layout,
//                 Some("metallic"),
//             ),
//             material::Material::new(
//                 &device,
//                 device.create_shader_module(include_spirv!(concat!(
//                     env!("OUT_DIR"),
//                     "/dielectric.spv"
//                 ))),
//                 &paths,
//                 &material_queues[2],
//                 &extension_queue,
//                 &instances,
//                 &dielectric_data,
//                 &blas_data,
//                 &tlas_data,
//                 &light_sample_bindgroup_layout,
//                 Some("dielectric"),
//             ),
//             material::Material::new(
//                 &device,
//                 device.create_shader_module(include_spirv!(concat!(
//                     env!("OUT_DIR"),
//                     "/emissive.spv"
//                 ))),
//                 &paths,
//                 &material_queues[3],
//                 &extension_queue,
//                 &instances,
//                 &emissive_data,
//                 &blas_data,
//                 &tlas_data,
//                 &light_sample_bindgroup_layout,
//                 Some("emissive"),
//             ),
//         ];

//         let mut rng = rand::rng();
//         let mut spheres = (0..0)
//             .map(|_| Sphere {
//                 position: [
//                     rng.random_range(-10.0..=10.0),
//                     rng.random_range(0.0..=5.0),
//                     rng.random_range(0.0..=10.0),
//                 ],
//                 radius: rng.random_range(0.001..=1.0),
//             })
//             .collect::<Vec<_>>();
//         spheres.push(Sphere {
//             position: [5.0, -10000.0, 3.0],
//             // position: [5.0, -10000000.0, 3.0],
//             radius: 0.0,
//             ..Default::default()
//         });

//         let extension_phase = extension::ExtensionPhase::new(
//             &device,
//             &paths,
//             &extension_queue,
//             &shadow_queue,
//             &blas_data,
//             &tlas_data,
//             &instances,
//         );

//         let shadow_phase = shadow::ShadowPhase::new(
//             &device,
//             &paths,
//             &shadow_queue,
//             &blas_data,
//             &tlas_data,
//             &light_sample_bindgroup_layout,
//             &material_phases[3],
//             &instances,
//         );

//         Ok(Self {
//             surface,
//             device,
//             queue,
//             config,
//             is_surface_configured: false,
//             window,
//             logic_phase,
//             render_phase,
//             samples,
//             new_ray_phase,
//             extension_phase,
//             material_queues,
//             material_phases,
//             paths,
//             new_ray_queue,
//             extension_queue,
//             instances,
//             camera,
//             dims,
//             keys_pressed: HashSet::new(),
//             blas_data,
//             tlas_data,
//             light_sample_bindgroup,
//             light_sample_bindgroup_layout,
//             shadow_queue,
//             shadow_phase,
//         })
//     }

//     pub fn resize(&mut self, width: u32, height: u32) {
//         if width > 0 && height > 0 {
//             self.config.width = width;
//             self.config.height = height;
//             self.surface.configure(&self.device, &self.config);
//             self.is_surface_configured = true;
//         }
//     }

//     fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
//         match (code, is_pressed) {
//             (KeyCode::Escape, true) => {
//                 event_loop.exit();
//             }
//             (KeyCode::KeyQ, true) => {
//                 dbg!(&self.camera.data);
//             }
//             _ => {}
//         }

//         if is_pressed {
//             self.keys_pressed.insert(code);
//         } else {
//             self.keys_pressed.remove(&code);
//         }
//     }

//     fn handle_mouse(&mut self, event_loop: &ActiveEventLoop, position: PhysicalPosition<f64>) {}

//     fn handle_mouse_motion(&mut self, event_loop: &ActiveEventLoop, delta: (f32, f32)) {
//         self.camera.rotate(delta);
//     }

//     fn update(&mut self) {
//         const MOVE_SPEED: f32 = 0.01;
//         for key in &self.keys_pressed {
//             match key {
//                 KeyCode::KeyW => {
//                     self.camera.translate((0.0, 0.0, MOVE_SPEED));
//                 }
//                 KeyCode::KeyA => {
//                     self.camera.translate((-MOVE_SPEED, 0.0, 0.0));
//                 }
//                 KeyCode::KeyS => {
//                     self.camera.translate((0.0, 0.0, -MOVE_SPEED));
//                 }
//                 KeyCode::KeyD => {
//                     self.camera.translate((MOVE_SPEED, 0.0, 0.0));
//                 }
//                 KeyCode::Space => {
//                     self.camera.translate((0.0, MOVE_SPEED, 0.0));
//                 }
//                 KeyCode::ControlLeft => {
//                     self.camera.translate((0.0, -MOVE_SPEED, 0.0));
//                 }
//                 _ => {}
//             };
//         }
//     }

//     fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
//         self.window.request_redraw();

//         // Game logic stuff
//         self.update();

//         // Updating any buffers
//         self.camera.update(&self.queue);

//         // Rendering:
//         if !self.is_surface_configured {
//             return Ok(());
//         }

//         let output = self.surface.get_current_texture()?;

//         let view = output
//             .texture
//             .create_view(&wgpu::TextureViewDescriptor::default());

//         for _ in 0..1 {
//             let mut commands = Vec::new();

//             commands.push(self.logic_phase.render(
//                 &self.device,
//                 &self.paths,
//                 &self.samples,
//                 &self.camera,
//                 &self.new_ray_queue,
//                 &self.dims,
//             ));

//             commands.push(self.new_ray_phase.render(
//                 &self.device,
//                 &self.paths,
//                 &self.samples,
//                 &self.new_ray_queue,
//                 &self.extension_queue,
//                 &self.camera,
//                 &self.dims,
//             ));

//             for (i, mat_phase) in self.material_phases.iter().enumerate() {
//                 commands.push(mat_phase.render(
//                     &self.device,
//                     &self.paths,
//                     &self.material_queues[i],
//                     &self.extension_queue,
//                     &self.blas_data,
//                     &self.tlas_data,
//                     &self.instances,
//                     &self.light_sample_bindgroup,
//                 ));
//             }

//             commands.push(self.extension_phase.render(
//                 &self.device,
//                 &self.paths,
//                 &self.extension_queue,
//                 &self.shadow_queue,
//                 &self.blas_data,
//                 &self.tlas_data,
//                 &self.instances,
//             ));

//             commands.push(self.shadow_phase.render(
//                 &self.device,
//                 &self.paths,
//                 &self.shadow_queue,
//                 &self.blas_data,
//                 &self.tlas_data,
//                 &self.light_sample_bindgroup,
//                 &self.material_phases[3],
//                 &self.instances,
//             ));

//             commands.push(self.render_phase.render(
//                 &self.device,
//                 &self.logic_phase.output(),
//                 &view,
//             ));

//             self.queue.submit(commands);
//         }

//         output.present();

//         Ok(())
//     }
// }

// fn redraw_system(mut state: ResMut<State>) {
//     state.render().unwrap();
// }

// fn resize_system(mut state: ResMut<State>, mut reader: MessageReader<WinitWindowEvent>) {
//     for WinitWindowEvent(e) in reader.read() {
//         match e {
//             WindowEvent::Resized(size) => state.resize(size.width, size.height),
//             _ => {}
//         }
//     }
// }
