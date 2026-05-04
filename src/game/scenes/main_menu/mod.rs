use std::{path::PathBuf, sync::Arc};

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{Gpu, RenderContext, RenderTarget, SurfaceDesc},
        scene::Scene,
    },
    game::{
        assets::{images::Images, sprites::Sprites},
        config::{help_window_defs::HelpWindowDefs, load_config},
        file_system::FileSystem,
        render::textures::Textures,
        ui::windows::{help::HelpWindow, main_menu::MainMenuWindow, window_manager::WindowManager},
    },
};

pub struct MainMenuScene {
    window_manager: WindowManager,
}

impl MainMenuScene {
    pub fn new(
        file_system: Arc<FileSystem>,
        gpu: Gpu,
        surface_desc: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let images = Arc::new(Images::new(Arc::clone(&file_system)));
        let textures = Arc::new(Textures::new(gpu.clone(), Arc::clone(&images)));
        let sprites = Arc::new(Sprites::new(Arc::clone(&textures), &file_system)?);

        let help_window_defs: HelpWindowDefs = load_config(
            &file_system,
            PathBuf::from("config").join("help_window_defs.txt"),
        )?;

        let mut window_manager =
            WindowManager::new(file_system, gpu, surface_desc, textures, sprites)?;

        {
            window_manager.push(Box::new(MainMenuWindow::new(&window_manager)?));

            if let Some(help_def) = help_window_defs.get("conf_exit_game") {
                let surface_size = window_manager.window_renderer().surface_size();
                window_manager.push(Box::new(HelpWindow::new(help_def, surface_size)));
            }
        }

        Ok(Self { window_manager })
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.window_manager.resize(size);
    }

    fn input_event(&mut self, event: &InputEvent) {
        self.window_manager.input(event);
    }

    fn update(&mut self, delta_time: f32) {
        self.window_manager.update(delta_time);
    }

    fn render(
        &mut self,
        _gpu: &Gpu,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
    ) {
        self.window_manager.render(render_context, render_target);
    }
}

/*
struct ButtonData<'a> {
    name: &'a str,
    text_sprite: &'a str,
    text_frame: usize,
    shadow_sprite: &'a str,
    shadow_frame: usize,
    pressed_sprite: &'a str,
    pressed_frame: usize,
}

impl<'a> ButtonData<'a> {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        name: &'a str,
        text_sprite: &'a str,
        text_frame: usize,
        shadow_sprite: &'a str,
        shadow_frame: usize,
        pressed_sprite: &'a str,
        pressed_frame: usize,
    ) -> Self {
        Self {
            name,
            text_sprite,
            text_frame,
            shadow_sprite,
            shadow_frame,
            pressed_sprite,
            pressed_frame,
        }
    }
}

const BUTTONS: &[ButtonData<'static>] = &[
    ButtonData::new(
        "b_new_game",
        "interface_elements_14",
        0,
        "interface_elements_14",
        1,
        "interface_elements_14",
        2,
    ),
    ButtonData::new(
        "b_load_game",
        "interface_elements_13",
        0,
        "interface_elements_13",
        1,
        "interface_elements_13",
        2,
    ),
    ButtonData::new(
        "b_training",
        "interface_elements_17",
        0,
        "interface_elements_17",
        1,
        "interface_elements_17",
        2,
    ),
    ButtonData::new(
        "b_options",
        "interface_elements_15",
        0,
        "interface_elements_15",
        1,
        "interface_elements_15",
        2,
    ),
    ButtonData::new(
        "b_intro",
        "interface_elements_13",
        3,
        "interface_elements_13",
        4,
        "interface_elements_13",
        5,
    ),
    ButtonData::new(
        "b_multiplayer",
        "interface_elements_14",
        3,
        "interface_elements_14",
        4,
        "interface_elements_14",
        5,
    ),
    ButtonData::new(
        "b_exit",
        "interface_elements_15",
        3,
        "interface_elements_15",
        4,
        "interface_elements_15",
        5,
    ),
];
*/
