use glam::{IVec2, UVec2, Vec4};

use crate::game::{
    config::windows::WindowBase,
    ui::render::window_renderer::{WindowRenderItems, WindowRenderer},
};

use super::window::Window;

pub struct MainMenuWindow {}

impl MainMenuWindow {
    pub fn new(_window_base: &WindowBase) -> Self {
        Self {}
    }
}

impl Window for MainMenuWindow {
    fn render(&mut self, _window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
        render_items.render_border(
            IVec2::new(10, 10),
            UVec2::new(100, 100),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
