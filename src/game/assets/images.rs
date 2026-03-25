use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, StorageMap},
    },
    game::{
        assets::{
            asset_source::AssetSource,
            image::{BlendMode, Image},
        },
        file_system::FileSystem,
    },
};

pub struct Images {
    file_system: Arc<FileSystem>,
    storage: RwLock<StorageMap<String, Image, Arc<Image>>>,
}

impl Images {
    pub fn new(file_system: Arc<FileSystem>) -> Self {
        Self {
            file_system,
            storage: RwLock::new(StorageMap::default()),
        }
    }

    pub fn get(&self, handle: Handle<Image>) -> Option<Arc<Image>> {
        self.storage.read().unwrap().get(handle).map(Arc::clone)
    }

    pub fn load(&self, path: impl Into<PathBuf>) -> Result<Handle<Image>, AssetError> {
        let path = path.into();

        // Return the cached value if it exists.
        {
            let storage = self.storage.read().unwrap();
            let key = Self::path_to_key(&path);
            if let Some(handle) = storage.get_handle_by_key(&key) {
                return Ok(handle);
            };
        }

        fn image_error_to_asset_error(err: image::ImageError, path: &PathBuf) -> AssetError {
            match err {
                image::ImageError::Decoding(_) => AssetError::Decode(path.clone()),
                image::ImageError::IoError(error) => AssetError::from_io_error(error, path),
                error => AssetError::custom(path, error),
            }
        }

        let data = self.file_system.load(&path)?;

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
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(data),
                is_color_keyd,
            )
            .map_err(|err| image_error_to_asset_error(err, &path))?;

            let raw = if let Ok(data) = self.file_system.load(path.with_extension("raw")) {
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
                Image::from_rgba(
                    AssetSource::FileSystem(path.clone()),
                    shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw),
                    BlendMode::Alpha,
                )
            } else {
                Image::from_rgba(
                    AssetSource::FileSystem(path.clone()),
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::Opaque,
                )
            }
        } else if ext == "jpg" || ext == "jpeg" {
            let image = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, &path))?;

            Image::from_rgba(
                AssetSource::FileSystem(path.clone()),
                image.into_rgba8(),
                BlendMode::Opaque,
            )
        } else {
            return Err(AssetError::NotSupported(path));
        };

        let handle = {
            let mut storage = self.storage.write().unwrap();
            storage.insert(Self::path_to_key(&path), Arc::new(image))
        };

        Ok(handle)
    }

    fn path_to_key(path: &Path) -> String {
        path.to_string_lossy().to_ascii_lowercase()
    }
}
