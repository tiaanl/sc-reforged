use glam::{IVec2, Vec4};

use crate::game::ui::{
    EventResult,
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    widgets::widget::{Widget, WidgetBase},
    windows::{
        actions::WindowManagerAction, window::WindowRenderContext,
        window_manager_context::WindowManagerContext,
    },
};

#[derive(Default)]
pub struct TextButtonWidget {
    pub widget: WidgetBase,

    pub is_pressed: bool,
    pub is_disabled: bool,

    pub action: Option<WindowManagerAction>,
    pub on_click: Option<Box<dyn FnMut()>>,

    pub text: String,
    pub font: Font,
    pub custom_color: Option<Vec4>,
}

impl TextButtonWidget {
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
    fn on_primary_mouse_down(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        tracing::info!("TextButton::on_primary_mouse_down");

        if self.is_disabled {
            return EventResult::Ignore;
        }

        self.is_pressed = true;

        // TODO: sound_manager().play_sound(5);

        // TODO: user data?
        // return (this->m_user_data != (Widget *)0x0) + '\x02';

        EventResult::Handled
    }

    fn on_primary_mouse_up(
        &mut self,
        _position: IVec2,
        context: &mut WindowManagerContext,
    ) -> EventResult {
        tracing::info!("TextButton::on_primary_mouse_up");

        let was_pressed = self.is_pressed;
        self.is_pressed = false;

        if !was_pressed {
            return EventResult::Ignore;
        }

        if let Some(action) = &self.action {
            println!("action");
            context.post_action(action.clone());
        }

        if let Some(on_click) = self.on_click.as_mut() {
            on_click();
        }

        EventResult::Handled
    }

    fn render(
        &mut self,
        origin: IVec2,
        _delta_time_ms: i32,
        context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    ) {
        let color = if let Some(custom_color) = self.custom_color {
            custom_color
        } else {
            self.font.primary_color()
        };
        window_render_items.render_border(self.widget.rect.offset(origin), 1, color);

        // TODO: Match the original button render states more closely. The
        // current button only nudges the text when pressed instead of using
        // the original style flags and pressed-state visuals.
        let text_offset = Self::calculate_text_offset(
            &self.text,
            self.widget.rect.size,
            self.font,
            context.window_renderer,
        ) + IVec2::splat(self.is_pressed as i32);

        window_render_items.render_text(
            origin + self.widget.rect.position + text_offset,
            self.text.as_bytes(),
            self.font,
            self.custom_color,
        );
    }
}
