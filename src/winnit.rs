use std::sync::Arc;

use bevy_ecs::prelude::*;
use itertools::Itertools;
use wgpu::{Instance, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{
    app::{BevyApp, State},
    render_resources::{RenderDevice, RenderQueue, RenderSurface},
};

#[derive(Message)]
pub struct WinitWindowEvent(pub winit::event::WindowEvent);

#[derive(Message)]
pub struct WinitDeviceEvent(pub winit::event::DeviceEvent);

#[derive(Message)]
pub struct WinitResizeEvent(pub PhysicalSize<u32>);

#[derive(Resource)]
pub struct WinitWindow(pub Arc<winit::window::Window>);

pub fn resize_system(
    mut reader: MessageReader<WinitResizeEvent>,
    mut surface: ResMut<RenderSurface>,
    device: Res<RenderDevice>,
) {
    if let Some(WinitResizeEvent(size)) = reader.read().last() {
        let width = size.width;
        let height = size.height;
        if width > 0 && height > 0 {
            surface.size = *size;
            surface.config.width = width;
            surface.config.height = height;
            surface.surface.configure(&device.0, &surface.config);
            surface.is_surface_configured = true;
        }
    }
}

#[derive(Resource)]
pub struct OutputBuffer(wgpu::Buffer);

pub fn render_system(
    surface: Res<RenderSurface>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let out = surface.surface.get_current_texture().unwrap();
}

pub struct WinitApp {
    bevy_app: BevyApp,
    window: Option<Arc<Window>>,
    window_events: Vec<winit::event::WindowEvent>,
    device_events: Vec<winit::event::DeviceEvent>,
    resize_event: Option<PhysicalSize<u32>>,
    first_resume: bool,
}

impl WinitApp {
    pub fn new(bevy_app: BevyApp) -> Self {
        Self {
            bevy_app,
            window: None,
            window_events: Vec::new(),
            device_events: Vec::new(),
            resize_event: None,
            first_resume: false,
        }
    }
}

impl WinitApp {
    fn redraw_requested(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.as_ref().map(|w| w.request_redraw());

        let window_events = self
            .window_events
            .drain(..)
            .map(|e| WinitWindowEvent(e))
            .collect_vec();

        self.bevy_app
            .world
            .resource_mut::<Messages<WinitWindowEvent>>()
            .write_batch(window_events);

        let device_events = self
            .device_events
            .drain(..)
            .map(|e| WinitDeviceEvent(e))
            .collect_vec();

        self.bevy_app
            .world
            .resource_mut::<Messages<WinitDeviceEvent>>()
            .write_batch(device_events);

        self.resize_event.take().map(|e| {
            self.bevy_app
                .world
                .resource_mut::<Messages<WinitResizeEvent>>()
                .write(WinitResizeEvent(e))
        });

        self.bevy_app.run();
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.first_resume {
            return;
        };
        self.first_resume = true;

        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        // winit stuff:
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .ok();
        window.set_cursor_visible(false);

        // register some systems
        self.bevy_app.update.add_systems(resize_system);

        // Prior to ever running we make sure there is a window resource:
        self.bevy_app
            .world
            .insert_resource(WinitWindow(window.clone()));

        self.bevy_app
            .world
            .init_resource::<Messages<WinitWindowEvent>>();

        self.bevy_app
            .world
            .init_resource::<Messages<WinitDeviceEvent>>();

        self.bevy_app
            .world
            .init_resource::<Messages<WinitResizeEvent>>();
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta: (x, y) } => {
                self.device_events.push(event)
                // const MOUSE_SENSITIVITY: f32 = 0.001;
                // state.handle_mouse_motion(
                //     event_loop,
                //     (x as f32 * MOUSE_SENSITIVITY, y as f32 * MOUSE_SENSITIVITY),
                // );
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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize_event = Some(size),
            WindowEvent::RedrawRequested => self.redraw_requested(event_loop),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => self.window_events.push(event),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => self.window_events.push(event),
            _ => {}
        }
    }
}
