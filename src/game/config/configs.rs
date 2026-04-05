use std::{
    any::{Any, TypeId},
    path::Path,
    sync::{Arc, RwLock},
};

use ahash::HashMap;
use bevy_ecs::prelude::*;

use crate::{
    engine::assets::AssetError,
    game::{config::parser::ConfigLines, file_system::FileSystem},
};

use super::{help_window_defs::HelpWindowDefs, load_config};

/// A resource that lazily loads then caches config files.
#[derive(Resource)]
pub struct Configs {
    file_system: Arc<FileSystem>,
    configs: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Configs {
    pub fn new(file_system: Arc<FileSystem>) -> Self {
        Self {
            file_system,
            configs: Arc::new(RwLock::new(HashMap::default())),
        }
    }

    pub fn load<C>(&self, path: impl AsRef<Path>) -> Result<Arc<C>, AssetError>
    where
        C: From<ConfigLines> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<C>();

        if let Some(config) = self.configs.read().unwrap().get(&type_id) {
            let config = Arc::clone(config);
            return Arc::downcast::<C>(config).map_err(|err| {
                panic!(
                    "Config cache type mismatch for {:?}",
                    std::any::type_name::<C>()
                )
            });
        }

        let config = Arc::new(load_config::<C>(&self.file_system, path.as_ref())?);

        self.configs
            .write()
            .unwrap()
            .insert(type_id, config.clone() as Arc<dyn Any + Send + Sync>);

        Ok(config)
    }
}
