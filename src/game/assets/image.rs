use std::path::Path;

use glam::UVec2;
use image::{ColorType, ImageDecoder, RgbaImage};
use thiserror::Error;

use crate::{
    engine::assets::AssetError,
    game::{assets::asset_factory::AssetFactory, file_system::FileSystem},
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
    #[allow(unused)]
    pub blend_mode: BlendMode,
    pub data: image::RgbaImage,
}

impl Image {
    /// Build an [Image] from RGBA pixel data.
    pub fn from_rgba(data: image::RgbaImage, blend_mode: BlendMode) -> Self {
        Self {
            size: UVec2::new(data.width(), data.height()),
            blend_mode,
            data,
        }
    }
}

#[derive(Debug, Error)]
pub enum ImageLoadError {
    #[error(".raw image has invalid dimensions: {0}")]
    InvalidDimensions(u32),
}

impl AssetFactory for Image {
    fn load(file_system: &FileSystem, path: &Path) -> Result<Self, AssetError> {
        fn image_error_to_asset_error(err: image::ImageError, path: &Path) -> AssetError {
            match err {
                image::ImageError::Decoding(error) => {
                    AssetError::Decode(path.to_path_buf(), Some(Box::new(error)))
                }
                image::ImageError::IoError(error) => AssetError::from_io_error(error, path),
                error => AssetError::custom(path, error),
            }
        }

        let data = file_system.load(path)?;

        let is_color_keyd = path
            .file_name()
            .filter(|n| n.to_string_lossy().contains("_ck"))
            .is_some();

        let ext = match path.extension() {
            Some(ext) => ext.to_ascii_lowercase(),
            None => {
                tracing::warn!("Image path has no extension: {}", path.display());
                std::ffi::OsString::new()
            }
        };

        let image = if ext == "bmp" {
            use image::codecs::bmp::BmpDecoder;

            let decoder = BmpDecoder::new(std::io::Cursor::new(data))
                .map_err(|err| image_error_to_asset_error(err, path))?;
            let (width, height) = decoder.dimensions();
            let mut rgba =
                read_as_rgba(decoder).map_err(|err| image_error_to_asset_error(err, path))?;

            if is_color_keyd {
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel[3] = if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
                        0
                    } else {
                        255
                    };
                }
            }

            let bmp =
                RgbaImage::from_raw(width, height, rgba).expect("Could not create RGBA image!");

            let raw = if let Ok(data) = file_system.load(path.with_extension("raw")) {
                let pixel_count = width as usize * height as usize;
                if data.len() != pixel_count {
                    return Err(AssetError::custom(
                        path,
                        format!(
                            ".raw alpha mask has invalid size: expected {pixel_count}, got {}",
                            data.len()
                        ),
                    ));
                }

                Some(data)
            } else {
                None
            };

            if is_color_keyd {
                Image::from_rgba(bmp, BlendMode::ColorKeyed)
            } else if let Some(raw) = raw {
                let mut rgba = bmp;
                for (pixel, alpha) in rgba.pixels_mut().zip(raw.iter()) {
                    // Set the alpha component from the raw image.
                    pixel.0[3] = *alpha;
                }

                quantize_rgba4444(&mut rgba);

                Image::from_rgba(rgba, BlendMode::Alpha)
            } else {
                Image::from_rgba(bmp, BlendMode::Opaque)
            }
        } else if ext == "raw" {
            let pixel_count = data.len();

            let size = calculate_raw_size(data.as_slice())
                .map_err(|err| AssetError::decode_with_error(path.to_path_buf(), err))?;

            // Make `data` mutable.
            let mut data = data;

            // Reserve an additional size * 3 pixels for the RGB values.
            data.reserve_exact((size.x * size.y * 3) as usize);

            // Force the length to avoid filling in the new values.
            unsafe {
                data.set_len(pixel_count * 4);
            }

            for i in (0..pixel_count).rev() {
                let alpha = data[i];
                let index = i * 4;

                data[index] = 255;
                data[index + 1] = 255;
                data[index + 2] = 255;
                data[index + 3] = alpha;
            }

            // SAFETY: We unwrap here because we ensured the buffer is big enough.
            let image = image::RgbaImage::from_vec(size.x, size.y, data).unwrap();

            Image::from_rgba(image, BlendMode::Alpha)
        } else if ext == "jpg" || ext == "jpeg" {
            use image::codecs::jpeg::JpegDecoder;

            let decoder = JpegDecoder::new(std::io::Cursor::new(data))
                .map_err(|err| image_error_to_asset_error(err, path))?;
            let (width, height) = decoder.dimensions();
            let rgba =
                read_as_rgba(decoder).map_err(|err| image_error_to_asset_error(err, path))?;

            let mut rgba =
                RgbaImage::from_raw(width, height, rgba).expect("Could not create RGBA image!");
            quantize_rgb565(&mut rgba);

            Image::from_rgba(rgba, BlendMode::Opaque)
        } else {
            return Err(AssetError::NotSupported(path.to_path_buf()));
        };

        Ok(image)
    }
}

/// Read a three-channel image decoder into an RGBA8 byte buffer.
fn read_as_rgba<D: ImageDecoder>(decoder: D) -> image::ImageResult<Vec<u8>> {
    let color_type = decoder.color_type();
    if color_type != ColorType::Rgb8 {
        return Err(image::ImageError::Unsupported(
            image::error::UnsupportedError::from_format_and_kind(
                image::error::ImageFormatHint::Unknown,
                image::error::UnsupportedErrorKind::Color(color_type.into()),
            ),
        ));
    }

    let (width, height) = decoder.dimensions();
    let pixel_count = width as usize * height as usize;
    let rgb_len = pixel_count * 3;
    let mut rgba = vec![0; pixel_count * 4];
    decoder.read_image(&mut rgba[..rgb_len])?;
    expand_rgb_to_rgba(&mut rgba, pixel_count);

    Ok(rgba)
}

/// Expand packed RGB8 pixels in-place into RGBA8 pixels.
fn expand_rgb_to_rgba(rgba: &mut [u8], pixel_count: usize) {
    for i in (0..pixel_count).rev() {
        let read_index = i * 3;
        let write_index = i * 4;

        rgba[write_index] = rgba[read_index];
        rgba[write_index + 1] = rgba[read_index + 1];
        rgba[write_index + 2] = rgba[read_index + 2];
        rgba[write_index + 3] = 255;
    }
}

/// Calculate the dimensions for square power-of-two raw alpha images.
fn calculate_raw_size(data: &[u8]) -> Result<UVec2, ImageLoadError> {
    let pixel_count = data.len();
    let side = (pixel_count as f32).sqrt() as u32;
    if side == 0 || (side as usize * side as usize) != pixel_count || !side.is_power_of_two() {
        return Err(ImageLoadError::InvalidDimensions(side));
    }

    Ok(UVec2::splat(side))
}

/// Quantize RGBA pixels to 4 bits per channel and expand them back to 8 bits.
fn quantize_rgba4444(image: &mut image::RgbaImage) {
    for pixel in image.pixels_mut() {
        for channel in &mut pixel.0 {
            *channel = (*channel >> 4) * 0x11;
        }
    }
}

/// Quantize RGB pixels to 5:6:5 bits and expand them back to 8 bits.
fn quantize_rgb565(image: &mut image::RgbaImage) {
    for pixel in image.pixels_mut() {
        let red = pixel.0[0] >> 3;
        let green = pixel.0[1] >> 2;
        let blue = pixel.0[2] >> 3;

        pixel.0[0] = (red << 3) | (red >> 2);
        pixel.0[1] = (green << 2) | (green >> 4);
        pixel.0[2] = (blue << 3) | (blue >> 2);
    }
}
