use glam::UVec2;

use crate::Asset;

use super::mesh_renderer::BlendMode;

pub struct Image {
    pub size: UVec2,
    pub blend_mode: BlendMode,
    pub data: image::RgbaImage,
}

impl Image {
    pub fn from_rgba(data: image::RgbaImage, blend_mode: BlendMode) -> Self {
        Self {
            size: UVec2::new(data.width(), data.height()),
            blend_mode,
            data,
        }
    }
}

impl Asset for Image {}
