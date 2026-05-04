use glam::{IVec2, Vec4};

use crate::{
    engine::assets::AssetError,
    game::ui::{
        Rect,
        render::window_renderer::{WindowRenderItems, WindowRenderer},
        widgets::{text_button::TextButtonWidget, widget::Widgets},
        windows::{actions::WindowManagerAction, window_manager::WindowManager},
    },
};

use super::{window::Window, window_manager_context::WindowManagerContext};

pub struct MainMenuWindow {
    rect: Rect,

    widgets: Widgets,
}

impl MainMenuWindow {
    pub fn new(window_manager: &WindowManager) -> Result<Self, AssetError> {
        let window_base = window_manager.get_window_base("main_menu")?;

        let size = IVec2::new(window_base.render_dx, window_base.render_dy);
        let size = size.max(IVec2::new(400, 300));

        let mut widgets = Widgets::default();

        widgets.add(Box::new(
            TextButtonWidget::new(
                Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
                "Training",
            )
            .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
        ));

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
        self.rect.contains(position)
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn on_primary_mouse_down(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> crate::game::ui::EventResult {
        println!("primary_mouse_down");
        self.widgets.on_primary_mouse_down(position, context)
    }

    fn on_primary_mouse_up(
        &mut self,
        position: IVec2,
        context: &mut WindowManagerContext,
    ) -> crate::game::ui::EventResult {
        println!("primary_mouse_up");
        self.widgets.on_primary_mouse_up(position, context)
    }

    fn render(&mut self, window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems) {
        self.widgets
            .render(self.rect.position, 100, window_renderer, render_items);

        render_items.render_border(
            self.rect.offset(IVec2::splat(10)),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
