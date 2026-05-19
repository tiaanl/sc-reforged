use glam::{IVec2, Vec4};

use crate::{
    engine::assets::AssetError,
    game::{
        config::windows::WindowCtx,
        globals,
        ui::{
            Rect,
            render::window_renderer::WindowRenderItems,
            widgets::text_button::TextButtonWidget,
            windows::{
                actions::WindowManagerAction,
                window::{Window, WindowCommon},
            },
        },
    },
};

use super::window::{WindowImpl, WindowRenderContext};

pub struct MainMenuWindow;

impl MainMenuWindow {
    pub fn new(surface_size: IVec2) -> Result<Window, AssetError> {
        let window_base = globals::window_manager().get_window_base("main_menu")?;

        let layout = window_base.layout(&WindowCtx::from_logical_size(surface_size));
        let size = IVec2::new(layout.render_dx, layout.render_dy);
        let size = size.max(IVec2::new(400, 300));

        let mut common = WindowCommon::new(Rect::from_size(size));

        common.widgets.add(Box::new(
            TextButtonWidget::new(
                Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
                "Training",
            )
            .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
        ));

        Ok(Window::new(common, Box::new(MainMenuWindow)))
    }
}

impl WindowImpl for MainMenuWindow {
    fn render(
        &mut self,
        common: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        common
            .widgets
            .render(common.rect.position, 100, context, render_items);

        render_items.render_border(
            common.rect.offset(IVec2::splat(10)),
            2,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
    }
}
