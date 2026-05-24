use std::sync::Arc;

use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::{
            render::window_renderer::WindowRenderItems,
            windows::{
                window::{Window, WindowCommon, WindowImpl},
                window_manager::WindowLayoutContext,
            },
        },
    },
};

use super::window::WindowRenderContext;

/// The in-game bottom bar containing the radar, clock, and command buttons.
///
/// Mirrors the original engine's window-base selection between the 640x480 and
/// 800x600 variants based on the active logical UI size.
pub struct BottomBarWindow;

/// Creates the bottom bar window for the current logical UI size.
pub fn new_bottombar_window(surface_size: IVec2) -> Result<Window, AssetError> {
    let window_base_name = if surface_size == IVec2::new(640, 480) {
        "bottombar_640x480"
    } else {
        "bottombar_800x600"
    };
    let window_base = globals::window_manager().get_window_base(window_base_name)?;
    let layout_context = WindowLayoutContext::from_logical_size(surface_size);
    let rect = window_base.resolve_layout_rect(&layout_context);

    let window =
        Window::from_window_base(Arc::clone(&window_base), rect, Box::new(BottomBarWindow))?;

    println!("geometries: {:?}", window.common.render_geometry);

    Ok(window)
}

impl WindowImpl for BottomBarWindow {
    fn render(
        &mut self,
        common: &mut WindowCommon,
        _context: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        common
            .render_geometry
            .render(common.rect.position, render_items);
    }
}
