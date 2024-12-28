use mygame::MyGame;
use window::GameWindow;
use winit::event_loop::EventLoop;

mod mygame;
mod window;

fn main() {
    pretty_env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut window: GameWindow<MyGame<'_>> = GameWindow::new();

    event_loop.run_app(&mut window).expect("Application error");
}
