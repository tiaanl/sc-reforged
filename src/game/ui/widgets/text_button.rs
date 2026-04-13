use glam::{IVec2, UVec2, Vec4};

use crate::game::ui::{
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::widget::Widget,
};

pub struct TextButtonWidget {
    pos: IVec2,
    size: UVec2,
    text: String,

    text_offset: IVec2,
}

impl TextButtonWidget {
    pub fn new(
        pos: IVec2,
        size: UVec2,
        text: impl Into<String>,
        window_renderer: &WindowRenderer,
    ) -> Self {
        let text = text.into();

        let text_offset = Self::calculate_text_offset(&text, size, window_renderer);

        Self {
            pos,
            size,
            text,

            text_offset,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>, window_renderer: &WindowRenderer) {
        self.text = text.into();
        self.text_offset = Self::calculate_text_offset(&self.text, self.size, window_renderer);
    }

    fn calculate_text_offset(text: &str, size: UVec2, window_renderer: &WindowRenderer) -> IVec2 {
        let width = window_renderer.measure_text_width(text, Font::FifteenPoint) as i32;
        let height = window_renderer.measure_text_height(text, Font::FifteenPoint) as i32;

        (size.as_ivec2() / 2) - IVec2::new(width, height) / 2
    }
}

impl Widget for TextButtonWidget {
    fn render(&self, window_render_items: &mut WindowRenderItems) {
        window_render_items.render_border(self.pos, self.size, 1, Vec4::ONE);
        window_render_items.render_text(
            self.pos + self.text_offset,
            &self.text,
            Font::FifteenPoint,
            None,
        );
    }
}
