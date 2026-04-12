use std::sync::Arc;

use glam::{IVec2, UVec2, Vec4};

use crate::{
    engine::{
        assets::AssetError,
        input::InputEvent,
        renderer::{Frame, RenderContext, SurfaceDesc},
        scene::Scene,
    },
    game::{
        file_system::FileSystem,
        ui::{
            render::window_renderer::WindowRenderItems,
            windows::{window::Window, window_manager::WindowManager},
        },
    },
};

pub struct MainMenuScene {
    window_manager: WindowManager,
}

impl MainMenuScene {
    const FRAME_FADE_SPEED: f32 = 0.4;

    pub fn new(
        file_system: Arc<FileSystem>,
        render_context: RenderContext,
        surface_desc: &SurfaceDesc,
    ) -> Result<Self, AssetError> {
        let mut window_manager = WindowManager::new(file_system, render_context, surface_desc)?;

        // Create the main menu.

        #[derive(Default)]
        struct MainMenuWindow {}

        impl Window for MainMenuWindow {
            fn render(&mut self, render_items: &mut WindowRenderItems) {
                render_items.render_border(
                    IVec2::new(10, 10),
                    UVec2::new(100, 100),
                    2,
                    Vec4::new(1.0, 0.0, 0.0, 1.0),
                );
            }
        }

        let main_menu_window = Box::new(MainMenuWindow::default());

        window_manager.push(main_menu_window);

        Ok(Self { window_manager })
    }
}

impl Scene for MainMenuScene {
    fn resize(&mut self, size: glam::UVec2) {
        self.window_manager.resize(size);
    }

    fn input_event(&mut self, _event: &InputEvent) {}

    fn update(&mut self, delta_time: f32) {
        self.window_manager.update(delta_time);
    }

    fn render(&mut self, context: &RenderContext, frame: &mut Frame) {
        self.window_manager.render(context, frame);
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
