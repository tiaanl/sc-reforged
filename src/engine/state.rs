use std::sync::Arc;

use super::{renderer::Renderer, scene::Scene, vfs::VirtualFileSystem};

pub struct State {
    pub vfs: Arc<VirtualFileSystem>,
    pub renderer: Renderer,
    pub scene: Option<Box<dyn Scene>>,

    // The instant that the last frame started to render.
    pub last_frame_time: std::time::Instant,
}

impl State {
    pub fn new(
        vfs: Arc<VirtualFileSystem>,
        renderer: Renderer,
        scene: Option<Box<dyn Scene>>,
    ) -> Self {
        Self {
            vfs,
            renderer,
            scene,

            last_frame_time: std::time::Instant::now(),
        }
    }
}
