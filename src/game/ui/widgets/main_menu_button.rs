use glam::{IVec2, Vec4};

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
#[allow(clippy::too_many_arguments)]
pub fn create_main_menu_button(
    position: IVec2,
    bullet_sprite: Handle<Sprite3d>,
    bullet_frame: usize,
    text_sprite: Handle<Sprite3d>,
    text_frame: usize,
    shadow_sprite: Handle<Sprite3d>,
    shadow_frame: usize,
    button_offset: IVec2,
    shadow_offset: IVec2,
) -> Box<MainMenuButton> {
    let bullet = ButtonLayer::new(bullet_sprite, bullet_frame, position);
    let text = ButtonLayer::new(text_sprite, text_frame, position + button_offset);
    let shadow = ButtonLayer::new(shadow_sprite, shadow_frame, position + shadow_offset);

    let rect = Rect::new(position, {
        let bullet_size = bullet.rect.size;
        let text_size = text.rect.size;
        IVec2::new(bullet_size.x + text_size.x, text_size.y)
    });

    Box::new(MainMenuButton {
        rect,

        bullet,
        text,
        shadow,

        pressed: false,
    })
}

pub struct MainMenuButton {
    pub rect: Rect,

    bullet: ButtonLayer,
    text: ButtonLayer,
    shadow: ButtonLayer,

    pressed: bool,
}

impl Widget for MainMenuButton {
    fn rect(&self) -> Rect {
        self.rect
    }

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
        _origin: glam::IVec2,
        _delta_time_ms: i32,
        _window_render_context: &mut WindowRenderContext<'_>,
        window_render_items: &mut WindowRenderItems,
    ) {
        let frame = if self.pressed {
            self.text.frame + 2
        } else {
            self.text.frame
        };

        window_render_items.render_sprite(self.text.rect.position, self.text.sprite, frame, 1.0);
        window_render_items.render_border(self.rect, 1, Vec4::ONE);
    }
}

struct ButtonLayer {
    sprite: Handle<Sprite3d>,
    frame: usize,
    rect: Rect,
}

impl ButtonLayer {
    fn new(sprite: Handle<Sprite3d>, frame: usize, position: IVec2) -> Self {
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
}
