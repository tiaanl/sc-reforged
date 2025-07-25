use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::game::file_system::FileSystemError;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("File not found ({0})")]
    FileNotFound(PathBuf),

    #[error("Decode error ({0})")]
    Decode(PathBuf),

    #[error("Unsupported asset ({0})")]
    NotSupported(PathBuf),

    #[error("File system error ({0})")]
    FileSystemError(#[from] FileSystemError),

    #[error("Unknown error ({0})")]
    Unknown(PathBuf, String),
}

impl AssetError {
    pub fn from_io_error(error: std::io::Error, path: &Path) -> Self {
        match error {
            err if err.kind() == std::io::ErrorKind::NotFound => {
                Self::FileNotFound(path.to_path_buf())
            }
            err => Self::Unknown(path.to_path_buf(), err.kind().to_string()),
        }
    }
}
