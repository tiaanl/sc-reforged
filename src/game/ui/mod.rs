pub mod render;
pub mod widgets;
pub mod windows;

mod rect;

use glam::Vec4;
pub use rect::Rect;

#[inline]
pub fn u32_to_color(value: u32) -> Vec4 {
    const MAX: f32 = i8::MAX as f32;

    Vec4::new(
        (value & 0xFF) as f32 / MAX,
        (value >> 8 & 0xFF) as f32 / MAX,
        (value >> 16 & 0xFF) as f32 / MAX,
        (value >> 24 & 0xFF) as f32 / MAX,
    )
}

#[derive(Debug)]
pub enum EventResult {
    Ignore,
    Handled,
}
