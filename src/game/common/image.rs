use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use ahash::HashMap;
use egui::TextBuffer;
use glam::UVec2;
use image::ImageError;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{config::ImageDefs, data_dir::data_dir, file_system::file_system},
    global,
};

#[derive(Clone, Copy, Debug, PartialEq)]
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

pub struct Images {
    images: Storage<Image>,
    lookup: HashMap<String, Handle<Image>>,

    image_defs: ImageDefs,
}

impl Images {
    pub fn new() -> Result<Self, AssetError> {
        let image_defs =
            data_dir().load_config::<ImageDefs>(PathBuf::from("config").join("image_defs.txt"))?;

        Ok(Self {
            images: Storage::default(),
            lookup: HashMap::default(),
            image_defs,
        })
    }

    pub fn get(&self, handle: Handle<Image>) -> Option<&Image> {
        self.images.get(handle)
    }

    pub fn load_image(&mut self, path: impl AsRef<Path>) -> Result<Handle<Image>, AssetError> {
        if let Some(handle) = self.lookup.get(path.as_ref().to_string_lossy().as_str()) {
            return Ok(*handle);
        }

        let image = self.load_image_internal(path.as_ref())?;
        let handle = self.images.insert(image);
        self.lookup
            .insert(path.as_ref().to_string_lossy().to_string(), handle);

        Ok(handle)
    }

    pub fn load_image_direct(&mut self, path: impl AsRef<Path>) -> Result<&Image, AssetError> {
        let handle = self.load_image(path)?;
        Ok(self.images.get(handle).unwrap())
    }

    fn load_image_internal(&self, path: impl AsRef<Path>) -> Result<Image, AssetError> {
        fn image_error_to_asset_error(err: ImageError, path: PathBuf) -> AssetError {
            match err {
                ImageError::Decoding(_) => AssetError::Decode(path),
                ImageError::IoError(error) => AssetError::from_io_error(error, &path),
                error => AssetError::Unknown(path, format!("{error:?}")),
            }
        }

        let is_color_keyd = path
            .as_ref()
            .file_name()
            .filter(|n| n.to_string_lossy().contains("_ck"))
            .is_some();

        let ext = match path.as_ref().extension() {
            Some(ext) => ext.to_ascii_lowercase(),
            None => {
                tracing::warn!("Image path has no extension: {}", path.as_ref().display());
                OsString::new()
            }
        };

        Ok(if ext == "bmp" {
            let data = file_system().load(path.as_ref())?;
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(data),
                is_color_keyd,
            )
            .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?;

            let raw = if let Ok(data) = file_system().load(path.as_ref().with_extension("raw")) {
                Some(
                    shadow_company_tools::images::load_raw_file(
                        &mut std::io::Cursor::new(data),
                        bmp.width(),
                        bmp.height(),
                    )
                    .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?,
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
            let data = file_system().load(path.as_ref())?;
            let image = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?;

            Image::from_rgba(image.into_rgba8(), BlendMode::Opaque)
        } else {
            return Err(AssetError::NotSupported(path.as_ref().to_path_buf()));
        })
    }
}

global!(Images, scoped_images, images);
