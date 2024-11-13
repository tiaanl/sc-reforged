use shadow_company_tools::smf;

use super::vfs::{FileSystemError, VirtualFileSystem};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("File system error: {0}")]
    FileSystemError(#[from] FileSystemError),

    #[error("Decode error")]
    DecodeError,

    #[error("Image load error: {0}")]
    ImageLoadError(#[from] image::ImageError),
}

pub struct Assets {
    fs: VirtualFileSystem,
}

impl Assets {
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
