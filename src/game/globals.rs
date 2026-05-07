use std::{path::Path, sync::OnceLock};

use crate::game::file_system::FileSystem;

static FILE_SYSTEM: OnceLock<FileSystem> = OnceLock::new();

pub fn init(root_dir: impl AsRef<Path>) -> bool {
    if !FILE_SYSTEM.set(FileSystem::new(root_dir)).is_ok() {
        return false;
    }

    //

    true
}

pub fn file_system() -> &'static FileSystem {
    FILE_SYSTEM.get().unwrap()
}
