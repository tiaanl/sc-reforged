use glam::{IVec2, Vec4};

use crate::game::{
    config::windows::WindowBase,
    ui::{
        EventResult, Rect,
        render::window_renderer::{WindowRenderItems, WindowRenderer},
    },
};

use super::window::Window;

pub struct MainMenuWindow {
    rect: Rect,
}

impl MainMenuWindow {
    pub fn new(_window_base: &WindowBase) -> Self {
        let size = IVec2::new(_window_base.render_dx, _window_base.render_dy);

        Self {
            rect: Rect::from_size(size),
        }
    }
}

impl Window for MainMenuWindow {
    fn is_visible(&self) -> bool {
        true
    }

    fn wants_input(&self) -> bool {
        true
    }

    fn hit_test(&self, position: IVec2) -> bool {
        let bottom_right = self.rect.bottom_right();
        position.x >= self.rect.position.x
            && position.y >= self.rect.position.y
            && position.x < bottom_right.x
            && position.y < bottom_right.y
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn on_primary_mouse_down(&mut self, _mouse: IVec2) -> EventResult {
        EventResult::Ignore
    }

    fn on_secondary_mouse_down(&mut self, _mouse: IVec2) -> EventResult {
        EventResult::Ignore
    }

    fn render(&mut self, _window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
        render_items.render_border(
            self.rect.offset(IVec2::splat(10)),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
