use super::vfs::VirtualFileSystem;
use std::path::Path;

pub struct Handle(usize);

pub struct Assets {
    vfs: VirtualFileSystem,
}

impl Assets {
    pub fn new(vfs: VirtualFileSystem) -> Self {
        Self { vfs }
    }

    pub fn load_config_file(&self, path: impl AsRef<Path>) -> Result<String, ()> {
        let data = self.vfs.open(path.as_ref()).map_err(|_| ())?;
        String::from_utf8(data).map_err(|_| ())
    }
}
