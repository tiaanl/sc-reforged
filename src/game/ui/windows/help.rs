use crate::game::ui::render::window_renderer::{Font, WindowRenderItems};

use super::window::Window;

pub struct HelpWindow {}

impl HelpWindow {
    pub fn new() -> Self {
        Self {}
    }
}

impl Window for HelpWindow {
    fn render(&mut self, render_items: &mut WindowRenderItems) {
        render_items.render_text(
            glam::Vec2::new(20.0, 20.0),
            "Hello, World!",
            Font::FifteenPoint,
            None,
        );
    }
}
