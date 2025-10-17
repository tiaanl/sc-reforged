use std::path::{Path, PathBuf};

use ahash::HashMap;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        config::{LodModelProfileDefinition, SubModelDefinition},
        data_dir::data_dir,
        file_system::file_system,
        model::Model,
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
        let model_lod_defs = {
            let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

            let profiles_path = PathBuf::from("config").join("lod_model_profiles");
            for lod_path in file_system().dir(&profiles_path)?.filter(|path| {
                path.extension()
                    .filter(|ext| ext.eq_ignore_ascii_case("txt"))
                    .is_some()
            }) {
                let profile = data_dir().load_config::<LodModelProfileDefinition>(lod_path)?;
                lod_definitions.insert(
                    profile.lod_model_name.clone(),
                    profile.sub_model_definitions.clone(),
                );
            }

            lod_definitions
        };

        Ok(Self {
            models: Storage::default(),
            lookup: HashMap::default(),
            model_lod_defs,
        })
    }

    pub fn add(&mut self, name: impl Into<String>, model: Model) -> Handle<Model> {
        let handle = self.models.insert(model);
        self.lookup.insert(name.into(), handle);
        handle
    }

    pub fn load_object_model(&mut self, name: &str) -> Result<(Handle<Model>, &Model), AssetError> {
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

    pub fn load_biped_body_model(
        &mut self,
        name: &str,
    ) -> Result<(Handle<Model>, &Model), AssetError> {
        // TODO: No LOD's for bipedal models?

        let path = PathBuf::from("models")
            .join("people")
            .join("bodies")
            .join(name)
            .join(name)
            .with_extension("smf");
        self.load_model(name, path)
    }

    pub fn load_biped_head_model(
        &mut self,
        name: &str,
    ) -> Result<(Handle<Model>, &Model), AssetError> {
        // TODO: No LOD's for bipedal models?

        let path = PathBuf::from("models")
            .join("people")
            .join("heads")
            .join(name)
            .join(name)
            .with_extension("smf");
        self.load_model(name, path)
    }

    pub fn load_model(
        &mut self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> Result<(Handle<Model>, &Model), AssetError> {
        if let Some(handle) = self.lookup.get(name) {
            return Ok((*handle, self.get(*handle).unwrap()));
        }

        let data = file_system().load(&path)?;

        let smf = shadow_company_tools::smf::Model::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let model = Model::try_from(smf)?;

        if model.meshes.is_empty() {
            return Err(AssetError::custom(path, "Model does not contain meshes!"));
        }

        let handle = self.add(name, model);

        Ok((handle, self.get(handle).unwrap()))
    }

    pub fn get(&self, handle: Handle<Model>) -> Option<&Model> {
        self.models.get(handle)
    }

    pub fn _get_mut(&mut self, handle: Handle<Model>) -> Option<&mut Model> {
        self.models.get_mut(handle)
    }
}

global!(Models, scoped_models, models);
