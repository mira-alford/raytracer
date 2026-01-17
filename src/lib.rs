use winit::event_loop::EventLoop;

use crate::{app::BevyApp, winnit::WinitApp};

mod app;
mod blas;
mod bvh;
mod camera;
mod dielectric;
mod dims;
mod emissive;
mod extension;
mod instance;
mod lambertian;
mod logic;
mod material;
mod mesh;
mod metallic;
mod new_ray;
mod path;
mod pathtracer;
mod queue;
mod render;
mod render_resources;
mod sample;
mod scenes;
mod shadow;
mod texture;
mod tlas;
mod winnit;

pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut bevy_app = BevyApp::new();

    render_resources::initialize(&mut bevy_app);
    render::initialize(&mut bevy_app);
    pathtracer::initialize(&mut bevy_app);

    let event_loop = EventLoop::new()?;
    let mut app = WinitApp::new(bevy_app);
    event_loop.run_app(&mut app)?;

    Ok(())
}
