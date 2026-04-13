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

        let width = window_renderer.measure_text_width(&text, Font::FifteenPoint);
        let height = window_renderer.measure_text_height(&text, Font::FifteenPoint);

        let text_offset = (size + UVec2::new(width, height)).as_ivec2() / 2;

        Self {
            pos,
            size,
            text,

            text_offset,
        }
    }
}

impl Widget for TextButtonWidget {
    fn render(&mut self, window_render_items: &mut WindowRenderItems) {
        window_render_items.render_border(self.pos, self.size, 1, Vec4::ONE);
        window_render_items.render_text(
            self.pos + self.text_offset,
            &self.text,
            Font::FifteenPoint,
            None,
        );
    }
}
