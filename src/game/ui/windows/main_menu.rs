use glam::{IVec2, Vec4};

use crate::{
    engine::assets::AssetError,
    game::ui::{
        Rect,
        render::window_renderer::{WindowRenderItems, WindowRenderer},
        widgets::widget::Widgets,
        windows::window_manager::WindowManager,
    },
};

use super::window::Window;

pub struct MainMenuWindow {
    rect: Rect,

    widgets: Widgets,
}

impl MainMenuWindow {
    pub fn new(window_manager: &WindowManager) -> Result<Self, AssetError> {
        let window_base = window_manager.get_window_base("main_menu")?;

        let size = IVec2::new(window_base.render_dx, window_base.render_dy);

        let widgets = Widgets::default();

        Ok(Self {
            rect: Rect::from_size(size),
            widgets,
        })
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

    fn render(&mut self, _window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
        render_items.render_border(
            self.rect.offset(IVec2::splat(10)),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
