use michaels_screensaver::run;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

fn main() {
    pollster::block_on(run());
}
