use glam::{IVec2, UVec2};

use crate::game::ui::{
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::{text_button::TextButtonWidget, widget::Widgets},
};

use super::window::Window;

pub struct HelpWindow {
    widgets: Widgets,
}

impl HelpWindow {
    pub fn new(window_renderer: &WindowRenderer) -> Self {
        let mut widgets = Widgets::default();

        widgets.add(Box::new(TextButtonWidget::new(
            IVec2::new(20, 20),
            UVec2::new(100, 30),
            "Ok",
            window_renderer,
        )));
        widgets.add(Box::new(TextButtonWidget::new(
            IVec2::new(20, 60),
            UVec2::new(100, 30),
            "Cancel",
            window_renderer,
        )));

        Self { widgets }
    }
}

impl Window for HelpWindow {
    fn render(&mut self, render_items: &mut WindowRenderItems) {
        render_items.render_text(
            glam::IVec2::new(20, 20),
            "Hello, World!",
            Font::FifteenPoint,
            None,
        );
    }
}
