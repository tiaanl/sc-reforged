use std::{path::PathBuf, sync::Arc};

use crate::{
    engine::{
        assets::AssetError,
        renderer::{Frame, RenderContext, SurfaceDesc},
    },
    game::{
        assets::{images::Images, sprites::Sprites},
        config::{ImageDefs, load_config},
        file_system::FileSystem,
        render::textures::Textures,
        ui::render::window_renderer::{WindowRenderItems, WindowRenderer},
    },
};

use super::window::Window;

pub struct WindowManager {
    window_renderer: WindowRenderer,
    window_render_items_cache: WindowRenderItems,

    pub windows: Vec<Box<dyn Window>>,
}

impl WindowManager {
    pub fn new(
        file_system: Arc<FileSystem>,
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let window_renderer = {
            let images = Arc::new(Images::new(Arc::clone(&file_system)));
            let textures = Arc::new(Textures::new(render_context.clone(), Arc::clone(&images)));

            let mut sprites = Sprites::new(Arc::clone(&images));
            let image_defs: ImageDefs =
                load_config(&file_system, PathBuf::from("config").join("image_defs.txt"))?;

            sprites.load_image_defs(&image_defs);
            let sprites = Arc::new(sprites);

            WindowRenderer::new(
                render_context.clone(),
                surface_desc,
                Arc::clone(&textures),
                Arc::clone(&sprites),
            )
        };

        Ok(Self {
            window_renderer,
            window_render_items_cache: WindowRenderItems::default(),
            windows: Vec::default(),
        })
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
