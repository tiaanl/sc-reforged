use super::{input::InputState, renderer::Frame};

/// A trait that represents a scene in the engine. I splits each stage of the render pipeline into
/// separate function calls.
#[allow(unused)]
pub trait Scene {
    /// Called when the size of the window surface is changed.
    fn resize(&mut self, width: u32, height: u32);

    /// Called each frame with the `delta_time` based on the time the last frame took and the state
    /// of all input devices.
    fn update(&mut self, delta_time: f32, input: &InputState);

    /// Called before `render_frame`, but after `update` to allow render resources to be created and
    /// updated before rendering.
    fn begin_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {}

    /// Called to render the the frame to the surface.
    fn render_frame(&self, frame: &mut Frame) {}

    /// Called after rendering the frame to do any cleanup.
    fn end_frame(&mut self) {}

    /// Called to allow debug panels to be added to the window.
    fn debug_panel(&mut self, egui: &egui::Context) {}
}
