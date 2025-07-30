use std::path::{Path, PathBuf};

use ahash::HashMap;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        config::SubModelDefinition, data_dir::data_dir, file_system::file_system, model::Model,
    },
    global,
};

pub struct Models {
    models: Storage<Model>,
    lookup: HashMap<String, Handle<Model>>,

    model_lod_defs: HashMap<String, Vec<SubModelDefinition>>,
}

impl Models {
    pub fn new() -> Result<Self, AssetError> {
        let model_lod_defs = data_dir().load_lod_model_profiles()?;

        Ok(Self {
            models: Storage::default(),
            lookup: HashMap::default(),
            model_lod_defs,
        })
    }

    pub fn load_object_model(&mut self, name: &str) -> Result<Handle<Model>, AssetError> {
        let path = if let Some(def) = self.model_lod_defs.get(name) {
            PathBuf::from(&def[0].sub_model_model)
        } else {
            PathBuf::from(name)
        };

        let path = PathBuf::from("models")
            .join(&path)
            .join(&path)
            .with_extension("smf");

        self.load_model(name, path)
    }

    pub fn load_bipedal_model(&mut self, name: &str) -> Result<Handle<Model>, AssetError> {
        // TODO: No LOD's for bipedal models?

        let path = PathBuf::from("models")
            .join("people")
            .join("bodies")
            .join(name)
            .join(name)
            .with_extension("smf");
        self.load_model(name, path)
    }

    fn load_model(
        &mut self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> Result<Handle<Model>, AssetError> {
        if let Some(handle) = self.lookup.get(name) {
            return Ok(*handle);
        }

        let data = file_system().load(&path)?;

        let smf = shadow_company_tools::smf::Model::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let model = Model::try_from(smf)?;

        let handle = self.models.insert(model);
        self.lookup.insert(name.to_owned(), handle);

        Ok(handle)
    }

    pub fn get(&self, handle: Handle<Model>) -> Option<&Model> {
        self.models.get(handle)
    }
}

global!(Models, scoped_models, models);
