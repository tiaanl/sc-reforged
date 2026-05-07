use std::{path::Path, sync::OnceLock};

use crate::game::{assets::images::Images, file_system::FileSystem};

static FILE_SYSTEM: OnceLock<FileSystem> = OnceLock::new();
static IMAGES: OnceLock<Images> = OnceLock::new();

pub fn init(root_dir: impl AsRef<Path>) -> bool {
    if FILE_SYSTEM.set(FileSystem::new(root_dir)).is_err() {
        return false;
    }

    if IMAGES.set(Images::default()).is_err() {
        return false;
    }

    true
}

pub fn file_system() -> &'static FileSystem {
    FILE_SYSTEM.get().unwrap()
}

pub fn images() -> &'static Images {
    IMAGES.get().unwrap()
}
