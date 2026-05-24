use std::sync::Arc;

use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        config::windows::WindowLayoutContext,
        globals,
        ui::{
            Rect,
            render::window_renderer::WindowRenderItems,
            widgets::text_button::TextButtonWidget,
            windows::{
                actions::WindowManagerAction,
                window::{Window, WindowImpl},
            },
        },
    },
};

use super::window::{WindowCommon, WindowRenderContext};

pub struct MainMenuWindow;

pub fn new_main_menu_window(context: &WindowLayoutContext) -> Result<Window, AssetError> {
    let window_base = globals::window_manager().get_window_base("main_menu")?;

    let rect = window_base.resolve_layout_rect(context);

    let mut window =
        Window::from_window_base(Arc::clone(&window_base), rect, Box::new(MainMenuWindow))?;

    window.common.widgets.add(Box::new(
        TextButtonWidget::new(
            Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
            "Training",
        )
        .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
    ));

    Ok(window)
}

impl WindowImpl for MainMenuWindow {
    fn render(
        &mut self,
        common: &mut WindowCommon,
        _context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        common.render_geometry.render(render_items);
    }
}
