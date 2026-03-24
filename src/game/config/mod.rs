#![allow(unused)]

use std::path::Path;

use crate::{
    engine::assets::AssetError,
    game::{config::parser::ConfigLines, file_system::FileSystem},
};

mod campaign;
mod character_profiles;
mod image_defs;
mod model_lods;
mod mtf;
mod object_templates;
pub mod parser;
mod terrain_mapping;
pub mod windows;

pub use campaign::*;
pub use character_profiles::*;
pub use image_defs::*;
pub use model_lods::*;
pub use mtf::*;
pub use object_templates::*;
pub use terrain_mapping::*;

pub fn load_config<C: From<ConfigLines>>(
    file_system: &FileSystem,
    path: impl AsRef<Path>,
) -> Result<C, AssetError> {
    let data = file_system.load(path)?;
    let text = String::from_utf8_lossy(&data);
    let config_lines = ConfigLines::parse(&text);
    Ok(C::from(config_lines))
}
