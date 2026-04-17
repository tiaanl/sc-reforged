use glam::{IVec2, Vec4};

use crate::game::{
    config::windows::WindowBase,
    ui::{
        Rect,
        render::window_renderer::{WindowRenderItems, WindowRenderer},
    },
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
            Rect::new(IVec2::splat(10), IVec2::splat(100)),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
