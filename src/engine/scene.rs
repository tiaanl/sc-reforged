use glam::UVec2;

use crate::engine::prelude::Renderer;

use super::{input::InputState, renderer::Frame};

/// A trait that represents a scene in the engine. I splits each stage of the render pipeline into
/// separate function calls.
#[allow(unused)]
pub trait Scene {
    /// Called when the size of the window surface is changed.
    fn resize(&mut self, size: UVec2) {}

    /// Called each frame with the `delta_time` based on the time the last frame took and the state
    /// of all input devices.
    fn update(&mut self, delta_time: f32, input: &InputState) {}

    /// Called to render the the frame to the surface.
    fn render(&mut self, renderer: &Renderer, frame: &mut Frame);

    /// Called to allow debug panels to be added to the window.
    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, egui: &egui::Context, frame_index: u64) {}
}
