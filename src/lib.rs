pub mod map;
mod raycaster;
mod render;

use std::{mem::take, sync::Arc};

use anyhow::Ok;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{map::Map, raycaster::Raycaster, render::Renderer};

struct State {
    window: Arc<Window>,
    renderer: Renderer,
    raycaster: Raycaster,
}

impl State {
    fn new(window: Arc<Window>, maps: Arc<Vec<Map>>) -> anyhow::Result<Self> {
        let renderer = pollster::block_on(Renderer::new(&window))?;
        let raycaster = Raycaster::new(renderer.config(), maps.clone())?;

        Ok(Self {
            window,
            renderer,
            raycaster,
        })
    }
}

struct App {
    state: Option<State>,
    width: u32,
    height: u32,
    maps: Arc<Vec<Map>>,
}

impl App {
    fn new(width: u32, height: u32, maps: Arc<Vec<Map>>) -> Self {
        Self {
            state: None,
            width,
            height,
            maps,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.state = Some(State::new(window, take(&mut self.maps)).unwrap());
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
                state.raycaster.update();
                state.renderer.render().unwrap();
            }
            WindowEvent::Resized(_size) => {
                //
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(_code),
                        state: _key_state,
                        ..
                    },
                ..
            } => {
                //
            }
            _ => (),
        }
    }
}

pub fn run(window_width: u32, window_height: u32, maps: Arc<Vec<Map>>) -> anyhow::Result<()> {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(window_width, window_height, maps);
    event_loop.run_app(&mut app)?;

    Ok(())
}
