use std::{ffi::OsString, path::PathBuf};

use glam::UVec2;
use image::ImageError;

use crate::{
    engine::assets::AssetError,
    game::assets::{Asset, AssetLoadContext},
};

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

impl Asset for Image {
    fn from_memory(
        _context: &mut AssetLoadContext,
        path: PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError> {
        fn image_error_to_asset_error(err: ImageError, path: PathBuf) -> AssetError {
            match err {
                ImageError::Decoding(_) => AssetError::Decode(path),
                ImageError::IoError(error) => AssetError::from_io_error(error, &path),
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
            .map_err(|err| image_error_to_asset_error(err, path.clone()))?;

            let raw = if let Ok(data) = _context._assets.load_raw(path.with_extension("raw")) {
                Some(
                    shadow_company_tools::images::load_raw_file(
                        &mut std::io::Cursor::new(data),
                        bmp.width(),
                        bmp.height(),
                    )
                    .map_err(|err| image_error_to_asset_error(err, path))?,
                )
            } else {
                None
            };

            if is_color_keyd {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::ColorKeyed,
                )
            } else if let Some(raw) = raw {
                Image::from_rgba(
                    shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw),
                    BlendMode::Alpha,
                )
            } else {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::Opaque,
                )
            }
        } else if ext == "jpg" || ext == "jpeg" {
            let image = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, path))?;

            Image::from_rgba(image.into_rgba8(), BlendMode::Opaque)
        } else {
            return Err(AssetError::NotSupported(path));
        })
    }
}
