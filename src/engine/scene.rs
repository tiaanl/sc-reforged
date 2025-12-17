use glam::UVec2;

use crate::engine::{assets::AssetError, context::EngineContext, renderer::Renderer};

use super::{input::InputState, renderer::Frame};

/// Trait used to load [Scene]s.
pub trait SceneLoader: Send + Sync + 'static {
    fn load_scene(
        self: Box<Self>,
        engine_context: EngineContext,
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
    ) -> Result<Box<dyn Scene>, AssetError>;
}

/// Trait defining a scene with callbacks for each stage of the render pipeline.
/// Scenes are sent across threads when switching via the `EventLoopProxy`, so they must be `Send`.
pub trait Scene: Send {
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
