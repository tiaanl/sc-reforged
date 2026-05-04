use std::{path::PathBuf, sync::Arc};

use glam::UVec2;

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{Frame, RenderContext, SurfaceDesc},
    },
    game::{
        assets::{images::Images, sprites::Sprites},
        config::{help_window_defs::HelpWindowDefs, load_config},
        file_system::FileSystem,
        render::textures::Textures,
        ui::windows::{help::HelpWindow, main_menu::MainMenuWindow, window_manager::WindowManager},
    },
};

/// The main state of the game.
pub struct GameState {
    window_manager: WindowManager,
}

impl GameState {
    pub fn new(
        file_system: Arc<FileSystem>,
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let images = Arc::new(Images::new(Arc::clone(&file_system)));
        let textures = Arc::new(Textures::new(render_context.clone(), images));
        let sprites = Arc::new(Sprites::new(Arc::clone(&textures), &file_system)?);

        let mut window_manager = WindowManager::new(
            Arc::clone(&file_system),
            render_context,
            surface_desc,
            textures,
            sprites,
        )?;

        let window_base = window_manager.get_window_base("main_menu")?;
        window_manager.push(Box::new(MainMenuWindow::new(&window_base)));

        {
            let help_window_defs: HelpWindowDefs = load_config(
                &file_system,
                PathBuf::from("config").join("help_window_defs.txt"),
            )?;

            if let Some(help_def) = help_window_defs.get("conf_exit_game") {
                window_manager.push(Box::new(HelpWindow::new(help_def, surface_desc.size)));
            }
        }

        Ok(Self { window_manager })
    }

    pub fn resize(&mut self, size: UVec2) {
        self.window_manager.resize(size);
    }

    pub fn input(&mut self, event: &InputEvent) {
        self.window_manager.input(event);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.window_manager.update(delta_time);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.window_manager.render(frame);
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, egui: &egui::Context, frame_index: u64) {
        let _ = (egui, frame_index);
    }
}
