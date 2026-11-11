use glam::IVec2;

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
            windows::{window::WindowRenderContext, window_manager_context::WindowManagerContext},
        },
    },
};

/// Creates a layered main-menu button using the original widget bounds calculation.
pub fn create_main_menu_button(
    position: IVec2,
    bullet: ButtonLayer,
    text: ButtonLayer,
    shadow: ButtonLayer,
) -> Widget {
    let rect = Rect::new(position, {
        let bullet_size = bullet.rect.size;
        let text_size = text.rect.size;
        IVec2::new(bullet_size.x + text_size.x, text_size.y)
    });

    let mut text_pressed = text.clone();
    text_pressed.frame += 2;

    Widget {
        common: WidgetCommon {
            rect,
            ..Default::default()
        },
        behavior: Some(Box::new(MainMenuButton {
            text_button: TextButtonBehavior::default(),
            bullet,
            shadow,
            text,
            text_pressed,
        })),
    }
}

pub struct MainMenuButton {
    text_button: TextButtonBehavior,

    bullet: ButtonLayer,
    shadow: ButtonLayer,
    text: ButtonLayer,
    text_pressed: ButtonLayer,
}

impl WidgetBehavior for MainMenuButton {
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

    fn render(
        &mut self,
        widget: &mut WidgetCommon,
        origin: IVec2,
        _delta_time_ms: i32,
        _window_render_context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    ) {
        let is_pressed = self.text_button.is_pressed;

        let _frame = if is_pressed {
            self.text.frame + 2
        } else {
            self.text.frame
        };

        self.bullet.render(window_render_items, origin, 1.0);
        self.shadow.render(window_render_items, origin, 0.25);
        if is_pressed {
            &self.text_pressed
        } else {
            &self.text
        }
        .render(window_render_items, origin, 1.0);

        // Render a debug border around the button.
        window_render_items.render_border(widget.rect.offset(origin), 1, glam::Vec4::ONE);
    }
}

#[derive(Clone)]
pub struct ButtonLayer {
    sprite: Handle<Sprite3d>,
    frame: usize,
    rect: Rect,
}

impl ButtonLayer {
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

    fn render(&self, render_items: &mut WindowRenderItems, origin: IVec2, alpha: f32) {
        render_items.render_sprite(self.rect.position + origin, self.sprite, self.frame, alpha);
    }
}
