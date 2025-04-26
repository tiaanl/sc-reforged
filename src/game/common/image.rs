use std::path::PathBuf;

use glam::UVec2;

use crate::engine::assets::{AssetError, AssetLoadContext, AssetType};

#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    /// No blending.
    Opaque,
    /// Color keyed (use black as the key).
    ColorKeyed,
    /// Use the alpha channel of the texture.
    Alpha,
    /// Multiply the values from the texture with the background. Mostly used for light effects.
    Multiply,
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

fn image_error_to_asset_error(err: image::ImageError, path: PathBuf) -> AssetError {
    match err {
        image::ImageError::Decoding(_) => AssetError::Decode(path),
        image::ImageError::Encoding(_) => {
            AssetError::Unknown(path, String::from("ImageError::Encoding"))
        }
        image::ImageError::Parameter(_) => {
            AssetError::Unknown(path, String::from("ImageError::Encoding"))
        }
        image::ImageError::Limits(_) => {
            AssetError::Unknown(path, String::from("ImageError::Encoding"))
        }
        image::ImageError::Unsupported(_) => {
            AssetError::Unknown(path, String::from("ImageError::Encoding"))
        }
        image::ImageError::IoError(error) => AssetError::from_io_error(error, &path),
    }
}

impl AssetType for Image {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        let is_color_keyd = context
            .path
            .file_name()
            .map(|n| n.to_string_lossy().contains("_ck"))
            .unwrap_or(false);

        let ext = context.path.extension().unwrap().to_ascii_lowercase();
        if ext == "bmp" {
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(raw),
                is_color_keyd,
            )
            .map_err(|err| image_error_to_asset_error(err, context.path.to_path_buf()))?;

            let raw = if let Ok(raw_data) = context
                .assets
                .load_direct::<Vec<u8>>(context.path.with_extension("raw"))
            {
                Some(
                    shadow_company_tools::images::load_raw_file(
                        &mut std::io::Cursor::new(raw_data),
                        bmp.width(),
                        bmp.height(),
                    )
                    .map_err(|err| image_error_to_asset_error(err, context.path.to_path_buf()))?,
                )
            } else {
                None
            };

            return Ok(if is_color_keyd {
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
            });
        } else if ext == "jpg" || ext == "jpeg" {
            let image = image::load_from_memory_with_format(raw, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, context.path.to_path_buf()))?;
            return Ok(Image::from_rgba(image.into_rgba8(), BlendMode::Opaque));
        }

        Err(AssetError::NotSupported(context.path.to_path_buf()))
    }
}
