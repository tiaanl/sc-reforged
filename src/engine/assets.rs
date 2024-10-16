use super::vfs::{FileSystem, FileSystemError};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("File system error: {0}")]
    FileSystemError(#[from] FileSystemError),

    #[error("Decode error")]
    DecodeError,
}

pub struct Assets {
    fs: FileSystem,
}

impl Assets {
    pub fn new(fs: FileSystem) -> Self {
        Self { fs }
    }

    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(self.fs.load(path)?)
    }

    pub fn load_config_file(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
        String::from_utf8(self.load_raw(path)?).map_err(|_| AssetError::DecodeError)
    }
}
