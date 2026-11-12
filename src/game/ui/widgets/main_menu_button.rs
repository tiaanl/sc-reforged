use glam::{IVec2, Vec2};

use crate::{
    engine::storage::Handle,
    game::{
        assets::sprites::Sprite3d,
        globals,
        ui::{
            Rect, WidgetResult,
            render::window_renderer::WindowRenderItems,
            widgets::{
                text_button::TextButtonBehavior,
                widget::{Widget, WidgetBehavior, WidgetCommon},
            },
            windows::{
                actions::WindowManagerAction, window::WindowRenderContext,
                window_manager_context::WindowManagerContext,
            },
        },
    },
};

/// Creates a layered main-menu button using the original widget bounds calculation.
pub fn create_main_menu_button(
    position: IVec2,
    bullet: ButtonLayer,
    text: ButtonLayer,
    shadow: ButtonLayer,
    action: Option<WindowManagerAction>,
) -> Widget {
    let rect = Rect::new(position, {
        let bullet_size = bullet.rect.size;
        let text_size = text.rect.size;
        IVec2::new(bullet_size.x + text_size.x, text_size.y)
    });

    let mut text_pressed = text.clone();
    text_pressed.frame += 2;
    let mut bullet_pressed = bullet.clone();
    bullet_pressed.frame += 2;

    Widget {
        common: WidgetCommon {
            rect,
            ..Default::default()
        },
        behavior: Some(Box::new(MainMenuButtonBehavior {
            text_button: TextButtonBehavior {
                action,
                ..Default::default()
            },
            bullet,
            shadow,
            text,
            text_pressed,
            bullet_pressed,
            hover_animation: HoverAnimation::default(),
        })),
    }
}

pub struct MainMenuButtonBehavior {
    text_button: TextButtonBehavior,

    bullet: ButtonLayer,
    shadow: ButtonLayer,
    text: ButtonLayer,
    text_pressed: ButtonLayer,
    bullet_pressed: ButtonLayer,
    hover_animation: HoverAnimation,
}

/// Linear shadow travel with a slower return when the pointer leaves.
#[derive(Default)]
struct HoverAnimation {
    progress_ms: f32,
}

impl HoverAnimation {
    /// Advances in seconds without discarding fractional milliseconds.
    fn update(&mut self, hovered: bool, delta_time: f32) {
        let rate = if hovered { 1.0 } else { -1.0 / 3.0 };
        self.progress_ms =
            (self.progress_ms + delta_time.max(0.0) * 1000.0 * rate).clamp(0.0, 250.0);
    }

    /// Moves the shadow from its resting position to the text anchor.
    fn shadow_position(&self, text: IVec2, shadow: IVec2) -> Vec2 {
        shadow
            .as_vec2()
            .lerp(text.as_vec2(), self.progress_ms / 250.0)
    }
}

impl WidgetBehavior for MainMenuButtonBehavior {
    fn on_primary_mouse_down(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        tracing::info!("Mouse down on {}!", &self.text_button.text);
        self.text_button
            .on_primary_mouse_down(widget, position, context)
    }

    fn on_primary_mouse_up(
        &mut self,
        widget: &mut WidgetCommon,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> WidgetResult {
        self.text_button
            .on_primary_mouse_up(widget, position, context)
    }

    fn update(&mut self, widget: &mut WidgetCommon, delta_time: f32) {
        self.hover_animation
            .update(widget.hovered && !self.text_button.is_disabled, delta_time);
    }

    fn render(
        &mut self,
        widget: &mut WidgetCommon,
        origin: IVec2,
        _delta_time_ms: i32,
        _window_render_context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    ) {
        if self.text_button.is_disabled {
            return;
        }
        let is_highlighted = widget.hovered || self.text_button.is_pressed;
        let shadow_position = self
            .hover_animation
            .shadow_position(self.text.rect.position, self.shadow.rect.position)
            .round()
            .as_ivec2();

        // Keep layer anchors fixed so reversing hover never accumulates drift.
        self.shadow.render(
            window_render_items,
            origin + shadow_position - self.shadow.rect.position,
            0.25,
        );
        if is_highlighted {
            &self.text_pressed
        } else {
            &self.text
        }
        .render(window_render_items, origin, 1.0);
        if is_highlighted {
            &self.bullet_pressed
        } else {
            &self.bullet
        }
        .render(window_render_items, origin, 1.0);
    }
}

#[derive(Clone)]
pub struct ButtonLayer {
    sprite: Handle<Sprite3d>,
    frame: usize,
    rect: Rect,
}

impl ButtonLayer {
    /// Creates a sprite layer anchored in window-local coordinates.
    pub fn new(sprite: Handle<Sprite3d>, frame: usize, position: IVec2) -> Self {
        let size = globals::sprites()
            .get(sprite)
            .and_then(|sprite| sprite.frame(frame))
            .map(|frame| frame.size())
            .unwrap_or_default();

        Self {
            sprite,
            frame,
            rect: Rect::new(position, size),
        }
    }

    /// Queues this layer with the supplied window origin and opacity.
    fn render(&self, render_items: &mut WindowRenderItems, origin: IVec2, alpha: f32) {
        render_items.render_sprite(self.rect.position + origin, self.sprite, self.frame, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_reaches_text_after_250_ms_and_returns_after_750_ms() {
        let mut animation = HoverAnimation::default();
        let text = IVec2::new(30, 5);
        let shadow = IVec2::new(43, 13);
        assert_eq!(animation.shadow_position(text, shadow), shadow.as_vec2());

        animation.update(true, 0.125);
        assert_eq!(
            animation.shadow_position(text, shadow),
            Vec2::new(36.5, 9.0)
        );
        animation.update(true, 0.125);
        assert_eq!(animation.shadow_position(text, shadow), text.as_vec2());
        animation.update(false, 0.75);
        assert_eq!(animation.shadow_position(text, shadow), shadow.as_vec2());
    }

    #[test]
    fn reversing_hover_preserves_progress_and_clamps_at_endpoints() {
        let mut animation = HoverAnimation::default();
        animation.update(true, 0.125);
        animation.update(false, 0.1875);
        assert_eq!(animation.progress_ms, 62.5);
        animation.update(true, 0.0625);
        assert_eq!(animation.progress_ms, 125.0);
        animation.update(true, 10.0);
        assert_eq!(animation.progress_ms, 250.0);
        animation.update(false, 10.0);
        assert_eq!(animation.progress_ms, 0.0);
    }

    #[test]
    fn return_keeps_fractional_progress_at_high_frame_rates() {
        let mut animation = HoverAnimation::default();
        animation.update(true, 0.25);
        for _ in 0..1500 {
            animation.update(false, 0.0005);
        }
        assert!(animation.progress_ms < 0.01);
    }
}
