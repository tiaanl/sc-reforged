use std::{path::PathBuf, sync::Arc};

use crate::{
    engine::assets::AssetError,
    game::{
        config::{load_config, windows::WindowLayoutContext},
        ui::{
            render::window_renderer::WindowRenderItems,
            windows::window::{Window, WindowImpl},
        },
    },
};

use super::window::{WindowCommon, WindowRenderContext};

pub struct MainMenuWindow;

pub fn new_main_menu_window(context: &WindowLayoutContext) -> Result<Window, AssetError> {
    let window_base = load_config::<crate::game::config::window_base::WindowBase>(
        PathBuf::from("config")
            .join("window_bases")
            .join("main_menu.txt"),
    )?;

    // common.widgets.add(Box::new(
    //     TextButtonWidget::new(
    //         Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
    //         "Training",
    //     )
    //     .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
    // ));

    let window_base = Arc::new(window_base);

    let window = Window::from_window_base(
        Arc::clone(&window_base),
        window_base.resolve_layout_rect(context),
        Box::new(MainMenuWindow),
    )?;

    Ok(window)
}

impl WindowImpl for MainMenuWindow {
    fn render(
        &mut self,
        common: &mut WindowCommon,
        context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        common.render_geometry.render(render_items);
    }
}
