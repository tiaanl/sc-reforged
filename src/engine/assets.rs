use super::vfs::FileSystem;
use std::path::Path;

pub struct Handle(usize);

pub struct Assets {
    fs: FileSystem,
}

impl Assets {
    pub fn new(fs: FileSystem) -> Self {
        Self { fs }
    }

    pub fn load_config_file(&self, path: impl AsRef<Path>) -> Result<String, ()> {
        let data = self.fs.load(path).map_err(|_| ())?;
        String::from_utf8(data).map_err(|_| ())
    }
}
