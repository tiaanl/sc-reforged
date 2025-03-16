use std::path::{Path, PathBuf};

use super::file_system::{FileSystem, FileSystemError, GutFileSystemLayer, OsFileSystemLayer};

pub struct VirtualFileSystem {
    file_system: FileSystem,
}

impl VirtualFileSystem {
    pub fn new(root_path: impl AsRef<Path>) -> std::io::Result<Self> {
        let root_path = root_path.as_ref().canonicalize()?;

        let mut file_system = FileSystem::default();
        file_system.push_layer(OsFileSystemLayer::new(&root_path));
        file_system.push_layer(GutFileSystemLayer::new(&root_path));

        Ok(Self { file_system })
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        Ok(self.file_system.load(path.as_ref())?)
    }

    pub fn enum_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, FileSystemError> {
        Ok(self.file_system.dir(path.as_ref())?)
    }
}
