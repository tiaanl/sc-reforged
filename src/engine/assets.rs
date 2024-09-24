use super::{
    renderer::Renderer,
    vfs::{VirtualFileSystem, VirtualFileSystemError},
};
use crate::game::config::ConfigFile;
use std::{path::Path, sync::Arc};

pub struct Handle(usize);

pub struct Assets {
    vfs: VirtualFileSystem,
    renderer: Arc<Renderer>,
}

impl Assets {
    pub fn new(vfs: VirtualFileSystem, renderer: Arc<Renderer>) -> Self {
        Self { vfs, renderer }
    }

    pub fn load_config_file(&self, path: impl AsRef<Path>) -> Result<String, ()> {
        let data = self.vfs.open(path.as_ref()).map_err(|_| ())?;
        String::from_utf8(data).map_err(|_| ())
    }
}
