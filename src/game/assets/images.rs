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
        assets::{asset_factory::AssetFactory, image::Image},
        globals,
    },
};

#[derive(Default)]
pub struct Images {
    storage: RwLock<StorageMap<String, Image, Arc<Image>>>,
}

impl Images {
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

        let image = Image::load(globals::file_system(), &path)?;

        let handle = {
            let mut storage = self.storage.write().unwrap();
            storage.insert(Self::path_to_key(&path), Arc::new(image))
        };

        Ok(handle)
    }

    /// Insert an image or return the existing handle if that key has already
    /// exists.
    pub fn insert(&self, key: impl Into<String>, image: Image) -> Handle<Image> {
        let key = key.into();

        {
            let storage = self.storage.read().unwrap();
            if let Some(handle) = storage.get_handle_by_key(&key) {
                return handle;
            }
        }

        let mut storage = self.storage.write().unwrap();
        if let Some(handle) = storage.get_handle_by_key(&key) {
            return handle;
        }

        storage.insert(key, Arc::new(image))
    }

    fn path_to_key(path: &Path) -> String {
        path.to_string_lossy().to_ascii_lowercase()
    }
}
