use glam::UVec2;

use crate::engine::renderer::{Frame, RenderContext};

use super::input::InputEvent;

/// Trait defining a scene with callbacks for each stage of the render pipeline.
/// Scenes are sent across threads when switching via the `EventLoopProxy`, so they must be `Send`.
pub trait Scene {
    /// Handle a window surface resize to the given `size`.
    fn resize(&mut self, size: UVec2);

    /// Handle a single input event. Called once per event, potentially multiple times per frame.
    fn input_event(&mut self, event: &InputEvent);

    /// Advance the scene by `delta_time` seconds from the previous frame.
    fn update(&mut self, delta_time: f32);

    /// Render the scene into the provided frame.
    fn render(&mut self, renderer: &RenderContext, frame: &mut Frame);

    /// Hook for adding debug panels.
    #[cfg(feature = "egui")]
    fn debug_panel(&mut self, egui: &egui::Context, frame_index: u64) {
        let _ = (egui, frame_index);
    }
}
