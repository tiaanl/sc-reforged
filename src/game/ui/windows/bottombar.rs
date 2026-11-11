use std::sync::Arc;

use glam::IVec2;

use crate::{
    engine::assets::AssetError,
    game::{
        globals,
        ui::windows::{
            window::{DefaultWindowBehavior, Window, WindowBuilder},
            window_manager::WindowLayoutContext,
        },
    },
};

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
    let rect = rect.offset(IVec2::new(0, surface_size.y - rect.size.y));

    WindowBuilder::new(rect)
        .with_window_base(Arc::clone(&window_base), &layout_context)
        .build(|_| Ok(DefaultWindowBehavior))
}
