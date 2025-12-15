use glam::UVec2;

use crate::engine::prelude::Renderer;

use super::{input::InputState, renderer::Frame};

/// Trait defining a scene with callbacks for each stage of the render pipeline.
pub trait Scene {
    /// Handle a window surface resize to the given `size`.
    fn resize(&mut self, size: UVec2);

    /// Advance the scene by `delta_time` seconds using the current input state.
    fn update(&mut self, delta_time: f32, input: &InputState);

    /// Render the scene into the provided frame.
    fn render(&mut self, renderer: &Renderer, frame: &mut Frame);

    /// Hook for adding debug panels.
    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, egui: &egui::Context, frame_index: u64) {
        let _ = (egui, frame_index);
    }
}
