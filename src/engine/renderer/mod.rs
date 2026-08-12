mod gpu;
mod mipmaps;
mod render_window;

use glam::UVec2;

pub use gpu::Gpu;
pub use render_window::{RenderWindow, SurfaceDesc};

pub struct RenderContext {
    pub encoder: wgpu::CommandEncoder,
    pub frame_index: u64,
}

pub struct RenderTarget {
    pub view: wgpu::TextureView,
    pub size: UVec2,
}
