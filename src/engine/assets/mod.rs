use ahash::HashMap;
use shadow_company_tools::smf;

use super::vfs::{FileSystemError, VirtualFileSystem};
use std::path::Path;

pub mod texture;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("File system error: {0}")]
    FileSystemError(#[from] FileSystemError),

    #[error("Decode error")]
    DecodeError,

    #[error("Image load error: {0}")]
    ImageLoadError(#[from] image::ImageError),
}

pub struct AssetLoader {
    fs: VirtualFileSystem,
}

impl AssetLoader {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            fs: VirtualFileSystem::new(data_dir),
        }
    }

    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(self.fs.load(path)?)
    }

    pub fn load_bmp(&self, path: impl AsRef<Path>) -> Result<image::DynamicImage, AssetError> {
        let data = self.load_raw(path.as_ref())?;
        Ok(image::load_from_memory_with_format(
            data.as_ref(),
            image::ImageFormat::Bmp,
        )?)
    }

    pub fn load_jpeg(&self, path: impl AsRef<Path>) -> Result<image::DynamicImage, AssetError> {
        let data = self.load_raw(path.as_ref())?;

        Ok(image::load_from_memory_with_format(
            data.as_ref(),
            image::ImageFormat::Jpeg,
        )?)
    }

    pub fn load_config_file(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
        String::from_utf8(self.load_raw(path)?).map_err(|_| AssetError::DecodeError)
    }

    pub fn load_smf(&self, path: impl AsRef<Path>) -> Result<smf::Model, AssetError> {
        let data = self.load_raw(path)?;
        let mut cursor = std::io::Cursor::new(data);
        smf::Model::read(&mut cursor)
            .map_err(|err| AssetError::FileSystemError(FileSystemError::Io(err)))
    }
}

pub trait Asset {}

pub struct Handle<A: Asset>(usize, std::marker::PhantomData<A>);

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
    }
}

impl<A: Asset> std::fmt::Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}

pub struct Assets<A: Asset> {
    next_id: usize,
    storage: HashMap<usize, A>,
}

impl<A: Asset> Default for Assets<A> {
    fn default() -> Self {
        Self {
            next_id: 0,
            storage: HashMap::default(),
        }
    }
}

impl<A: Asset> Assets<A> {
    pub fn add(&mut self, asset: impl Into<A>) -> Handle<A> {
        let id = self.next_id;
        self.next_id += 1;

        let _result = self.storage.insert(id, asset.into());
        assert!(_result.is_none(), "Killed an existing asset!");

        Handle(id, std::marker::PhantomData)
    }

    pub fn get(&self, handle: &Handle<A>) -> Option<&A> {
        self.storage.get(&handle.0)
    }

    pub fn remove(&mut self, handle: Handle<A>) {
        self.storage.remove(&handle.0);
    }
}
