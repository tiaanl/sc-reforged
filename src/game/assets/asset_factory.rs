use std::path::Path;

use crate::{engine::assets::AssetError, game::file_system::FileSystem};

pub trait AssetFactory
where
    Self: Sized,
{
    fn load(file_system: &FileSystem, path: &Path) -> Result<Self, AssetError>;
}
