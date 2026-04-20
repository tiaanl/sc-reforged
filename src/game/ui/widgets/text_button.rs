use glam::{IVec2, Vec4};

use crate::game::ui::{
    EventResult, Rect,
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::widget::Widget,
};

pub struct TextButtonWidget {
    rect: Rect,
    is_pressed: bool,
    on_click: Option<Box<dyn FnMut()>>,

    pub text: String,
    pub font: Font,
    pub custom_color: Option<Vec4>,
}

impl TextButtonWidget {
    /// Creates a new text button widget.
    pub fn new(rect: Rect, text: impl Into<String>) -> Self {
        Self {
            rect,
            is_pressed: false,
            on_click: None,
            text: text.into(),

            font: Font::Default,
            custom_color: None,
        }
    }

    /// Returns the widget with the provided click callback attached.
    pub fn with_on_click(mut self, on_click: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }

    /// Sets the callback to invoke when the button is clicked.
    pub fn set_on_click(&mut self, on_click: impl FnMut() + 'static) {
        self.on_click = Some(Box::new(on_click));
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
    fn rect(&self) -> Rect {
        self.rect
    }

    fn on_primary_mouse_down(&mut self, _mouse_position: IVec2) -> EventResult {
        self.is_pressed = true;
        EventResult::Handled
    }

    fn on_primary_mouse_up(&mut self, _mouse_position: IVec2) -> EventResult {
        let was_pressed = std::mem::replace(&mut self.is_pressed, false);

        if !was_pressed {
            return EventResult::Ignore;
        }

        if let Some(on_click) = self.on_click.as_mut() {
            on_click();
        }

        EventResult::Handled
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

        // TODO: Match the original button render states more closely. The
        // current button only nudges the text when pressed instead of using
        // the original style flags and pressed-state visuals.
        let text_offset =
            Self::calculate_text_offset(&self.text, self.rect.size, self.font, window_renderer)
                + IVec2::splat(self.is_pressed as i32);

        window_render_items.render_text(
            origin + self.rect.position + text_offset,
            self.text.as_bytes(),
            self.font,
            self.custom_color,
        );
    }
}
