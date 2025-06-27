mod context;
mod geometry;
mod layout;
mod panel_widget;
mod render;
mod root_widget;
mod widget;

pub use context::Context;
pub use geometry::{Pos, Rect, Size};
pub use panel_widget::PanelWidget;
pub use render::RenderContext;

pub const UI_FIXED_SCALE: f32 = 60.0;

pub fn to_fixed_scale(value: f32) -> u32 {
    (value * UI_FIXED_SCALE).round() as u32
}

pub fn to_float_scale(value: u32) -> f32 {
    value as f32 / UI_FIXED_SCALE
}
