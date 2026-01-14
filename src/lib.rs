pub mod map;
mod raycaster;
mod renderer;

use std::{mem::take, sync::Arc};

use anyhow::Ok;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::PhysicalKey,
    window::{CursorIcon, Fullscreen, Window},
};

use crate::{
    map::{Map, Maps},
    raycaster::Raycaster,
    renderer::Renderer,
};

struct State {
    window: Arc<Window>,
    raycaster: Raycaster,
}

impl State {
    fn new(
        window: Arc<Window>,
        maps: Arc<Maps>,
        current_map_key: &'static str,
    ) -> anyhow::Result<Self> {
        let map = maps.get(current_map_key).unwrap();
        let renderer = pollster::block_on(Renderer::new(&window, map))?;
        let raycaster = Raycaster::new(renderer, maps.clone(), current_map_key)?;

        Ok(Self { window, raycaster })
    }
}

struct App {
    state: Option<State>,
    width: u32,
    height: u32,
    maps: Arc<Maps>,
    current_map_key: &'static str,
}

impl App {
    fn new(width: u32, height: u32, maps: Arc<Maps>, current_map_key: &'static str) -> Self {
        Self {
            state: None,
            width,
            height,
            maps,
            current_map_key,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .with_resizable(false)
            // .with_fullscreen(Some(Fullscreen::Borderless(None)));
            .with_fullscreen(None);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // lock cursor
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .unwrap();
        window.set_cursor_visible(false);

        self.state = Some(State::new(window, take(&mut self.maps), self.current_map_key).unwrap());
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            DeviceEvent::MouseMotion { delta } => {
                state.raycaster.handle_cursor_move(delta);
            }
            _ => (),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.raycaster.update().unwrap();
            }
            WindowEvent::Resized(size) => {
                state.raycaster.renderer().resize(size.width, size.height)
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state
                .raycaster
                .handle_key(event_loop, code, key_state.is_pressed()),
            _ => (),
        }
    }
}

pub fn run(
    window_width: u32,
    window_height: u32,
    maps: Maps,
    current_map_key: &'static str,
) -> anyhow::Result<()> {
    let maps = Arc::new(maps);
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(window_width, window_height, maps, current_map_key);
    event_loop.run_app(&mut app)?;

    Ok(())
}
