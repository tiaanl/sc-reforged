use glam::Vec2;
use winit::{event::MouseButton, keyboard::KeyCode};

use super::renderer::Renderer;

pub trait Scene {
    fn resize(&mut self, width: u32, height: u32);

    fn on_key_pressed(&mut self, key: KeyCode) {}
    fn on_key_released(&mut self, key: KeyCode) {}

    fn on_mouse_moved(&mut self, position: Vec2) {}
    fn on_mouse_pressed(&mut self, button: MouseButton) {}
    fn on_mouse_released(&mut self, button: MouseButton) {}

    fn update(&mut self, delta_time: f32);

    fn debug_panel(&mut self, egui: &egui::Context) {}

    fn begin_frame(&mut self) {}

    fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    );

    fn end_frame(&mut self) {}
}
