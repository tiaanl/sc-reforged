use glam::{IVec2, UVec2, Vec4};

use crate::game::ui::{
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::widget::Widget,
};

pub struct TextButtonWidget {
    pos: IVec2,
    size: UVec2,

    pub text: String,
    pub font: Font,
    pub custom_color: Option<Vec4>,
}

impl TextButtonWidget {
    pub fn new(pos: IVec2, size: UVec2, text: impl Into<String>) -> Self {
        Self {
            pos,
            size,
            text: text.into(),

            font: Font::Default,
            custom_color: None,
        }
    }

    fn calculate_text_offset(
        text: &str,
        size: UVec2,
        font: Font,
        window_renderer: &WindowRenderer,
    ) -> IVec2 {
        let width = window_renderer.measure_text_width(text, font) as i32;
        let height = window_renderer.measure_text_height(text, font) as i32;

        (size.as_ivec2() / 2) - IVec2::new(width, height) / 2
    }
}

impl Widget for TextButtonWidget {
    fn render(
        &self,
        origin: IVec2,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        let color = if let Some(custom_color) = self.custom_color {
            custom_color
        } else {
            self.font.default_color()
        };
        window_render_items.render_border(origin + self.pos, self.size, 1, color);

        let text_offset = {
            let width = window_renderer.measure_text_width(&self.text, self.font) as i32;
            let height = window_renderer.measure_text_height(&self.text, self.font) as i32;

            (self.size.as_ivec2() / 2) - IVec2::new(width, height) / 2
        };

        window_render_items.render_text(
            origin + self.pos + text_offset,
            &self.text,
            self.font,
            self.custom_color,
        );
    }
}
