use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::{
            Rect,
            widgets::text_button::TextButtonWidget,
            windows::{
                actions::WindowManagerAction,
                geometries::Geometries,
                window::{Window, WindowCommon, WindowImpl},
            },
        },
    },
};

pub struct MainMenuWindow;

pub fn new_main_menu_window(surface_size: IVec2) -> Result<Window, AssetError> {
    let window_base = globals::window_manager().get_window_base("main_menu")?;
    let geometries = Geometries::from_window_base(window_base, surface_size);

    let layout = geometries.layout();
    let size = IVec2::new(layout.render_dx, layout.render_dy).max(IVec2::new(400, 300));

    let mut common = WindowCommon::new(Rect::from_size(size));
    common.geometries = geometries;

    common.widgets.add(Box::new(
        TextButtonWidget::new(
            Rect::new(IVec2::new(10, 10), IVec2::new(100, 30)),
            "Training",
        )
        .with_action(WindowManagerAction::StartCampaign(String::from("training"))),
    ));

    Ok(Window::new(common, Box::new(MainMenuWindow)))
}

impl WindowImpl for MainMenuWindow {}
