use super::{input::InputState, renderer::Frame};

#[allow(unused)]
pub trait Scene {
    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, delta_time: f32, input: &InputState);

    fn debug_panel(&mut self, egui: &egui::Context) {}

    fn render_update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {}

    fn render_frame(&self, frame: &mut Frame) {}
}
