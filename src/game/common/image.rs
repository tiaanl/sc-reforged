use glam::UVec2;

#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    /// No blending.
    Opaque,
    /// Color keyed (use black as the key).
    ColorKeyed,
    /// Use the alpha channel of the texture.
    Alpha,
}

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
