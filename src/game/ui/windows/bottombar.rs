use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::{
            Rect,
            windows::{
                geometries::Geometries,
                window::{Window, WindowCommon, WindowImpl},
            },
        },
    },
};

/// The in-game bottom bar containing the radar, clock, and command buttons.
///
/// Mirrors the original engine's `bottombar_640x480` window base. The 800x600
/// variant is not implemented yet; switching variants requires runtime
/// `%screen_dx`/`%screen_dy` resolution.
pub struct BottomBarWindow;

pub fn new_bottombar_window(surface_size: IVec2) -> Result<Window, AssetError> {
    // Matches the original engine's selection rule: the 640x480 base is
    // only used on an exact 640x480 surface, otherwise the 800x600 base is
    // the default. Both share WINDOW_BASE_DY 194 and use %screen_dx to fill
    // the screen width, so the same resolver works for either.
    let window_base = globals::window_manager().get_window_base("bottombar_800x600")?;
    let geometries = Geometries::from_window_base(window_base, surface_size);

    let mut common = WindowCommon::new(compute_rect(
        geometries.layout().dx,
        geometries.layout().dy,
        surface_size,
    ));
    common.geometries = geometries;

    Ok(Window::new(common, Box::new(BottomBarWindow)))
}

/// Places the bar flush against the bottom of the screen, spanning the full
/// window-base width — matches the original engine layout.
fn compute_rect(layout_dx: i32, layout_dy: i32, logical: IVec2) -> Rect {
    Rect::new(
        IVec2::new(0, logical.y - layout_dy),
        IVec2::new(layout_dx, layout_dy),
    )
}

impl WindowImpl for BottomBarWindow {
    fn on_resize(&mut self, common: &mut WindowCommon, logical_size: IVec2) {
        // `common.geometries` was already re-resolved against `logical_size`
        // by `Window::on_resize` before this call.
        let layout = common.geometries.layout();
        common.rect = compute_rect(layout.dx, layout.dy, logical_size);
    }
}
