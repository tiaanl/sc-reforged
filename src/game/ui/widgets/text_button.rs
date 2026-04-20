use glam::{IVec2, Vec4};

use crate::game::ui::{
    Rect,
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::widget::{EventResult, Widget},
};

pub struct TextButtonWidget {
    rect: Rect,

    pub text: String,
    pub font: Font,
    pub custom_color: Option<Vec4>,
}

impl TextButtonWidget {
    pub fn new(rect: Rect, text: impl Into<String>) -> Self {
        Self {
            rect,
            text: text.into(),

            font: Font::Default,
            custom_color: None,
        }
    }

    fn calculate_text_offset(
        text: &str,
        size: IVec2,
        font: Font,
        window_renderer: &WindowRenderer,
    ) -> IVec2 {
        let width = window_renderer.measure_text_width(text.as_bytes(), font);
        let height = window_renderer.measure_text_height(text.as_bytes(), font);

        (size / 2) - IVec2::new(width, height) / 2
    }
}

impl Widget for TextButtonWidget {
    fn on_primary_mouse_down(&mut self, _mouse_position: IVec2) -> EventResult {
        EventResult::Ignore
    }

    fn on_primary_mouse_up(&mut self, _mouse_position: IVec2) -> EventResult {
        EventResult::Ignore
    }

    fn on_mouse_wheel(&mut self, _wheel_steps: i32) -> EventResult {
        EventResult::Ignore
    }

    fn render(
        &mut self,
        origin: IVec2,
        _delta_time_ms: i32,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        let color = if let Some(custom_color) = self.custom_color {
            custom_color
        } else {
            self.font.default_color()
        };
        window_render_items.render_border(self.rect.offset(origin), 1, color);

        let text_offset = {
            let width = window_renderer.measure_text_width(self.text.as_bytes(), self.font);
            let height = window_renderer.measure_text_height(self.text.as_bytes(), self.font);

            (self.rect.size / 2) - IVec2::new(width, height) / 2
        };

        window_render_items.render_text(
            origin + self.rect.position + text_offset,
            self.text.as_bytes(),
            self.font,
            self.custom_color,
        );
    }
}
