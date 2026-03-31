use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use glam::UVec2;
use image::ImageError;

use crate::{
    engine::assets::AssetError,
    game::{AssetLoadContext, assets::asset_source::AssetSource},
};

use super::Asset;

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

impl Asset for Image {
    fn from_memory(
        _context: &mut AssetLoadContext,
        path: PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError> {
        fn image_error_to_asset_error(err: ImageError, path: &Path) -> AssetError {
            match err {
                ImageError::Decoding(_) => AssetError::Decode(path.into()),
                ImageError::IoError(error) => AssetError::from_io_error(error, path),
                error => AssetError::custom(path, error),
            }
        }

        let is_color_keyd = path
            .file_name()
            .filter(|n| n.to_string_lossy().contains("_ck"))
            .is_some();

        let ext = match path.extension() {
            Some(ext) => ext.to_ascii_lowercase(),
            None => {
                tracing::warn!("Image path has no extension: {}", path.display());
                OsString::new()
            }
        };

        Ok(if ext == "bmp" {
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(data),
                is_color_keyd,
            )
            .map_err(|err| image_error_to_asset_error(err, &path))?;

            let raw = if let Ok(data) = _context.loader.load_raw(path.with_extension("raw")) {
                Some(
                    shadow_company_tools::images::load_raw_file(
                        &mut std::io::Cursor::new(data),
                        bmp.width(),
                        bmp.height(),
                    )
                    .map_err(|err| image_error_to_asset_error(err, &path))?,
                )
            } else {
                None
            };

            if is_color_keyd {
                Image::from_rgba(
                    AssetSource::FileSystem(path.clone()),
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::ColorKeyed,
                )
            } else if let Some(raw) = raw {
                let mut rgba = shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw);
                quantize_rgba4444(&mut rgba);
                Image::from_rgba(
                    AssetSource::FileSystem(path.clone()),
                    rgba,
                    BlendMode::Alpha,
                )
            } else {
                Image::from_rgba(
                    AssetSource::FileSystem(path.clone()),
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::Opaque,
                )
            }
        } else if ext == "raw" {
            // Standalone .raw files are headerless grayscale (one byte per pixel),
            // used as alpha-mapped textures (e.g. fonts). The original engine
            // (FUN_0048acd0) determines dimensions from the file size using a
            // hardcoded lookup: 256→16x16, 1024→32x32, 4096→64x64, 16384→128x128,
            // 65536→256x256. We generalize this to any square power-of-two size.
            let pixel_count = data.len();
            let side = (pixel_count as f64).sqrt() as u32;
            if (side * side) as usize != pixel_count || !side.is_power_of_two() {
                return Err(AssetError::Decode(path));
            }

            let raw = shadow_company_tools::images::load_raw_file(
                &mut std::io::Cursor::new(data),
                side,
                side,
            )
            .map_err(|err| image_error_to_asset_error(err, &path))?;

            let mut rgba = image::RgbaImage::new(side, side);
            for (dest, alpha) in rgba.pixels_mut().zip(raw.pixels()) {
                dest.0 = [255, 255, 255, alpha.0[0]];
            }
            quantize_rgba4444(&mut rgba);

            Image::from_rgba(AssetSource::FileSystem(path.clone()), rgba, BlendMode::Alpha)
        } else if ext == "jpg" || ext == "jpeg" {
            let image = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, &path))?;
            let mut rgba = image.to_rgba8();
            quantize_rgb565(&mut rgba);

            Image::from_rgba(
                AssetSource::FileSystem(path.clone()),
                rgba,
                BlendMode::Opaque,
            )
        } else {
            return Err(AssetError::NotSupported(path));
        })
    }
}
