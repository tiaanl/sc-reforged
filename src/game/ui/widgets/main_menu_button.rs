use glam::IVec2;

use crate::{
    engine::storage::Handle,
    game::{
        assets::sprites::Sprite3d,
        globals,
        ui::{
            EventResult, Rect,
            render::window_renderer::WindowRenderItems,
            widgets::widget::Widget,
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
) -> Box<MainMenuButton> {
    let rect = Rect::new(position, {
        let bullet_size = bullet.rect.size;
        let text_size = text.rect.size;
        IVec2::new(bullet_size.x + text_size.x, text_size.y)
    });

    let mut text_pressed = text.clone();
    text_pressed.frame += 2;

    Box::new(MainMenuButton {
        rect,

        bullet,
        shadow,
        text,
        text_pressed,

        pressed: false,
    })
}

pub struct MainMenuButton {
    pub rect: Rect,

    bullet: ButtonLayer,
    shadow: ButtonLayer,
    text: ButtonLayer,
    text_pressed: ButtonLayer,

    pressed: bool,
}

impl Widget for MainMenuButton {
    fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        if self.rect.contains(position) {
            self.pressed = true;
        }

        EventResult::Handled
    }

    fn on_primary_mouse_up(
        &mut self,
        _position: IVec2,
        _context: &mut WindowManagerContext,
    ) -> EventResult {
        self.pressed = false;

        EventResult::Handled
    }

    fn render(
        &mut self,
        origin: IVec2,
        _delta_time_ms: i32,
        _window_render_context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    ) {
        let _frame = if self.pressed {
            self.text.frame + 2
        } else {
            self.text.frame
        };

        self.bullet.render(window_render_items, origin, 1.0);
        self.shadow.render(window_render_items, origin, 0.25);
        if self.pressed {
            &self.text_pressed
        } else {
            &self.text
        }
        .render(window_render_items, origin, 1.0);

        // window_render_items.render_border(self.rect, 1, Vec4::ONE);
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
