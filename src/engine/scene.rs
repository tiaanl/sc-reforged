use super::renderer::Renderer;

pub trait Scene {
    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, delta_time: f32);

    fn begin_frame(&mut self) {}

    fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    );

    fn end_frame(&mut self) {}
}
