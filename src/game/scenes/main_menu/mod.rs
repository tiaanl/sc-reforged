use std::path::PathBuf;

use glam::UVec2;

use crate::{
    engine::{
        assets::AssetError,
        input::InputState,
        renderer::{Frame, Renderer, Surface},
        scene::Scene,
    },
    game::{
        config::{load_config, windows::WindowBase},
        file_system::FileSystem,
    },
};

mod render;

pub struct MainMenuScene {
    window_base: WindowBase,

    renderer: render::WindowRenderer,

    frames: Vec<render::TextureId>,
}

impl MainMenuScene {
    pub fn new(
        file_system: &FileSystem,
        renderer: &Renderer,
        surface: &Surface,
    ) -> Result<Self, AssetError> {
        let window_base: WindowBase = load_config(
            file_system,
            PathBuf::from("config")
                .join("window_bases")
                .join("main_menu.txt"),
        )?;

        let mut window_renderer = render::WindowRenderer::new(renderer, surface);

        let mut frames = vec![];

        for i in 0..5 {
            let path = PathBuf::from("textures")
                .join("interface")
                .join(format!("frame{i}.jpg"));

            let data = file_system.load(&path)?;

            let image = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .map_err(|err| AssetError::custom(path, format!("{err}")))?;
            let rgba = image.into_rgba8();

            frames.push(window_renderer.create_texture(renderer, rgba));
        }

        Ok(Self {
            window_base,
            renderer: window_renderer,
            frames,
        })
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: UVec2) {
        self.renderer.resize(size);
    }

    fn update(&mut self, _delta_time: f32, _input: &InputState) {}

    fn render(&mut self, renderer: &Renderer, frame: &mut Frame) {
        self.renderer.draw(renderer, frame);
    }
}
