use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::game::file_system::FileSystemError;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("File not found ({0})")]
    FileNotFound(PathBuf),

    #[error("Decode error ({0})")]
    Decode(PathBuf, Option<Box<dyn std::error::Error>>),

    #[error("Unsupported asset ({0})")]
    NotSupported(PathBuf),

    #[error("File system error ({0})")]
    FileSystemError(#[from] FileSystemError),

    #[error("{1} ({0})")]
    Custom(PathBuf, String),
}

impl AssetError {
    pub fn decode_with_error(path: PathBuf, error: impl std::error::Error + 'static) -> Self {
        Self::Decode(path, Some(Box::new(error)))
    }

    pub fn custom(path: impl AsRef<Path>, description: impl std::fmt::Display) -> Self {
        Self::Custom(path.as_ref().to_path_buf(), description.to_string())
    }

    pub fn from_io_error(error: std::io::Error, path: &Path) -> Self {
        match error {
            err if err.kind() == std::io::ErrorKind::NotFound => {
                Self::FileNotFound(path.to_path_buf())
            }
            err => Self::custom(path, err.kind().to_string()),
        }
    }
}
