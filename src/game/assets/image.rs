use glam::UVec2;

use crate::game::assets::asset_source::AssetSource;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum BlendMode {
    /// No blending.
    Opaque = 0,
    /// Color keyed (use black as the key).
    ColorKeyed = 1,
    /// Use the alpha channel of the texture.
    Alpha = 2,
    /// Adds the values of the texture to the image.
    _Additive = 3,
}

pub struct Image {
    #[allow(unused)]
    pub source: AssetSource,
    pub size: UVec2,
    #[allow(unused)]
    pub blend_mode: BlendMode,
    pub data: image::RgbaImage,
}

impl Image {
    /// Build an [Image] from RGBA pixel data.
    pub fn from_rgba(source: AssetSource, data: image::RgbaImage, blend_mode: BlendMode) -> Self {
        Self {
            source,
            size: UVec2::new(data.width(), data.height()),
            blend_mode,
            data,
        }
    }
}

/// Quantize RGBA pixels to 4 bits per channel and expand them back to 8 bits.
pub(crate) fn quantize_rgba4444(image: &mut image::RgbaImage) {
    for pixel in image.pixels_mut() {
        for channel in &mut pixel.0 {
            *channel = (*channel >> 4) * 0x11;
        }
    }
}

/// Quantize RGB pixels to 5:6:5 bits and expand them back to 8 bits.
pub(crate) fn quantize_rgb565(image: &mut image::RgbaImage) {
    for pixel in image.pixels_mut() {
        let red = pixel.0[0] >> 3;
        let green = pixel.0[1] >> 2;
        let blue = pixel.0[2] >> 3;

        pixel.0[0] = (red << 3) | (red >> 2);
        pixel.0[1] = (green << 2) | (green >> 4);
        pixel.0[2] = (blue << 3) | (blue >> 2);
    }
}
