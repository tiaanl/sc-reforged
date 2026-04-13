use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ahash::HashMap;

use crate::{
    engine::{
        assets::AssetError,
        renderer::{Frame, RenderContext, SurfaceDesc},
    },
    game::{
        assets::sprites::Sprites,
        config::{load_config, windows::WindowBase},
        file_system::FileSystem,
        render::textures::Textures,
        ui::render::window_renderer::{WindowRenderItems, WindowRenderer},
    },
};

use super::window::Window;

pub struct WindowManager {
    file_system: Arc<FileSystem>,

    window_bases: Mutex<HashMap<String, Arc<WindowBase>>>,

    window_renderer: WindowRenderer,
    window_render_items_cache: WindowRenderItems,

    windows: Vec<Box<dyn Window>>,
}

impl WindowManager {
    pub fn new(
        file_system: Arc<FileSystem>,
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
        textures: Arc<Textures>,
        sprites: Arc<Sprites>,
    ) -> Result<Self, AssetError> {
        let window_renderer =
            WindowRenderer::new(render_context.clone(), surface_desc, textures, sprites);

        Ok(Self {
            file_system,

            window_bases: Mutex::new(HashMap::default()),

            window_renderer,
            window_render_items_cache: WindowRenderItems::default(),

            windows: Vec::default(),
        })
    }

    pub fn window_renderer(&self) -> &WindowRenderer {
        &self.window_renderer
    }

    pub fn get_window_base(&self, name: &str) -> Result<Arc<WindowBase>, AssetError> {
        if let Some(def) = self.window_bases.lock().unwrap().get(name).cloned() {
            return Ok(def);
        }

        let path = PathBuf::from("config")
            .join("window_bases")
            .join(name)
            .with_extension("txt");

        let loaded: Arc<WindowBase> = Arc::new(load_config(self.file_system.as_ref(), path)?);

        let mut defs = self.window_bases.lock().unwrap();
        let def = defs
            .entry(name.to_string())
            .or_insert_with(|| Arc::clone(&loaded))
            .clone();

        Ok(def)
    }

    /// Push a new window to the top of the stack.
    pub fn push(&mut self, window: Box<dyn Window>) {
        self.windows.push(window);
    }

    pub fn resize(&mut self, size: glam::UVec2) {
        self.window_renderer.resize(size);
    }

    pub fn update(&mut self, _delta_time: f32) {
        //
    }

    pub fn render(&mut self, _render_context: &RenderContext, frame: &mut Frame) {
        self.window_render_items_cache.clear();

        for window in self.windows.iter_mut() {
            window.render(&mut self.window_render_items_cache);
        }

        self.window_renderer
            .submit_render_items(frame, &self.window_render_items_cache);
    }
}
