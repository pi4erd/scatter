use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub trait Game {
    fn init(window: Arc<Window>) -> Self;

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}
}

pub struct GameWindow<T: Game> {
    window: Option<Arc<Window>>,
    game: Option<T>,
}

impl<T: Game> Default for GameWindow<T> {
    fn default() -> Self {
        Self {
            window: None,
            game: None,
        }
    }
}

impl<T: Game> GameWindow<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: Game> ApplicationHandler for GameWindow<T> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        ));

        self.game = Some(Game::init(self.window.clone().unwrap()));
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.game
            .as_mut()
            .unwrap()
            .device_event(event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.game.as_mut().unwrap().about_to_wait(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.game
            .as_mut()
            .unwrap()
            .window_event(event_loop, window_id, event);
    }
}
