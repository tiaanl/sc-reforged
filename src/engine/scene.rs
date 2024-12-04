use super::{input::InputState, renderer::Renderer};

#[allow(unused)]
pub trait Scene {
    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, delta_time: f32, input: &InputState);

    fn debug_panel(&mut self, egui: &egui::Context) {}

    fn begin_frame(&mut self) {}

    fn render(
        &mut self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    );

    fn end_frame(&mut self) {}
}
