use std::{path::Path, sync::OnceLock};

use crate::{
    engine::renderer::Gpu,
    game::{
        assets::{images::Images, models::Models},
        file_system::FileSystem,
        render::textures::Textures,
    },
};

static FILE_SYSTEM: OnceLock<FileSystem> = OnceLock::new();
static IMAGES: OnceLock<Images> = OnceLock::new();
static MODELS: OnceLock<Models> = OnceLock::new();
static TEXTURES: OnceLock<Textures> = OnceLock::new();

pub fn init(root_dir: impl AsRef<Path>, gpu: Gpu) -> bool {
    if FILE_SYSTEM.set(FileSystem::new(root_dir)).is_err() {
        return false;
    }

    if IMAGES.set(Images::default()).is_err() {
        return false;
    }

    if MODELS.set(Models::default()).is_err() {
        return false;
    }

    if TEXTURES.set(Textures::new(gpu.clone())).is_err() {
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

pub fn models() -> &'static Models {
    MODELS.get().unwrap()
}

pub fn textures() -> &'static Textures {
    TEXTURES.get().unwrap()
}
