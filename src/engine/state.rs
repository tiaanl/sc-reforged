use super::{renderer::Renderer, scene::Scene, vfs::FileSystem};

pub struct State {
    pub fs: FileSystem,
    pub renderer: Renderer,
    pub scene: Option<Box<dyn Scene>>,

    // The instant that the last frame started to render.
    pub last_frame_time: std::time::Instant,
}

impl State {
    pub fn new(fs: FileSystem, renderer: Renderer, scene: Option<Box<dyn Scene>>) -> Self {
        Self {
            fs,
            renderer,
            scene,

            last_frame_time: std::time::Instant::now(),
        }
    }
}
