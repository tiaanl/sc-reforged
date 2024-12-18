use glam::UVec2;

use crate::Asset;

pub struct Image {
    pub size: UVec2,
    pub has_alpha: bool,
    pub data: image::RgbaImage,
}

impl Image {
    pub fn from_rgba(data: image::RgbaImage) -> Self {
        Self {
            size: UVec2::new(data.width(), data.height()),
            has_alpha: true,
            data,
        }
    }

    pub fn from_rgb(data: image::RgbaImage) -> Self {
        Self {
            size: UVec2::new(data.width(), data.height()),
            has_alpha: false,
            data,
        }
    }
}

impl Asset for Image {}
