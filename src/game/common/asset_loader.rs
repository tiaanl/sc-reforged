use std::{
    hash::Hash,
    path::{Path, PathBuf},
};

use ahash::HashMap;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        AssetReader,
        config::{ImageDefs, LodModelProfileDefinition, SubModelDefinition, parser::ConfigLines},
        file_system::file_system,
        image::Image,
        model::Model,
        models::ModelName,
        scenes::world::animation::motion::Motion,
    },
};

use super::Asset;

/// Interface for loading assets from the file system.
pub struct AssetLoader {
    _image_defs: ImageDefs,
    model_lod_defs: HashMap<String, Vec<SubModelDefinition>>,

    images: TypedAssetLoader<PathBuf, Image>,
    models: TypedAssetLoader<ModelName, Model>,
    motions: TypedAssetLoader<String, Motion>,
}

impl AssetLoader {
    pub fn new() -> Result<Self, AssetError> {
        let _image_defs = load_config(PathBuf::from("config").join("image_defs.txt"))?;

        let model_lod_defs = {
            let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

            let profiles_path = PathBuf::from("config").join("lod_model_profiles");
            for lod_path in file_system().dir(&profiles_path)?.filter(|path| {
                path.extension()
                    .filter(|ext| ext.eq_ignore_ascii_case("txt"))
                    .is_some()
            }) {
                let profile = load_config::<LodModelProfileDefinition>(lod_path)?;
                lod_definitions.insert(
                    profile.lod_model_name,
                    profile.sub_model_definitions.clone(),
                );
            }

            lod_definitions
        };

        Ok(Self {
            _image_defs,
            model_lod_defs,
            images: TypedAssetLoader::default(),
            models: TypedAssetLoader::default(),
            motions: TypedAssetLoader::default(),
        })
    }

    pub fn into_reader(self) -> AssetReader {
        AssetReader::new(self.images.assets, self.models.assets, self.motions.assets)
    }

    #[inline]
    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(file_system().load(path)?)
    }

    pub fn get_or_load_image(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(Handle<Image>, &Image), AssetError> {
        let path = path.as_ref().to_path_buf();

        if let Some(handle) = self.images.lookup.get(&path).cloned() {
            let image = self
                .images
                .assets
                .get(handle)
                .ok_or_else(|| AssetError::custom(&path, "Image handle missing"))?;
            return Ok((handle, image));
        }

        tracing::info!("Loading image: {}", path.display());

        let data = self.load_raw(&path)?;
        let mut context = AssetLoadContext { loader: self };
        let image = Image::from_memory(&mut context, path.clone(), &data)?;

        Ok(self.images.insert(path, image))
    }

    pub fn get_or_load_model(
        &mut self,
        name: ModelName,
    ) -> Result<(Handle<Model>, &Model), AssetError> {
        if let Some(handle) = self.models.lookup.get(&name).cloned() {
            let model = self
                .models
                .assets
                .get(handle)
                .ok_or_else(|| AssetError::custom(PathBuf::new(), "Model handle missing"))?;
            return Ok((handle, model));
        }

        let path = match &name {
            ModelName::Object(name) => {
                let lod_name = self
                    .model_lod_defs
                    .get(name)
                    .map(|def| def[0].sub_model_model.as_str())
                    .unwrap_or(name);

                PathBuf::from(lod_name).join(lod_name)
            }
            ModelName::Body(name) => PathBuf::from("people").join("bodies").join(name).join(name),
            ModelName::Head(name) => PathBuf::from("people").join("heads").join(name).join(name),
            ModelName::_Misc(name) => PathBuf::from("people").join("misc").join(name).join(name),
            ModelName::BodyDefinition(..) => panic!("Can't load body definition models!"),
        };

        let path = PathBuf::from("models").join(&path).with_extension("smf");

        let data = self.load_raw(&path)?;
        let mut context = AssetLoadContext { loader: self };
        let model = Model::from_memory(&mut context, path.clone(), &data)?;

        if model.meshes.is_empty() {
            return Err(AssetError::custom(path, "Model does not contain meshes!"));
        }

        Ok(self.models.insert(name, model))
    }

    pub fn add_model(&mut self, name: ModelName, model: Model) -> (Handle<Model>, &Model) {
        self.models.insert(name, model)
    }

    pub fn get_or_load_motion(
        &mut self,
        name: &str,
    ) -> Result<(Handle<Motion>, &Motion), AssetError> {
        let path = PathBuf::from("motions").join(name).with_extension("bmf");

        let data = self.load_raw(&path)?;

        let mut context = AssetLoadContext { loader: self };
        let motion = Motion::from_memory(&mut context, path, &data)?;

        Ok(self.motions.insert(name.to_string(), motion))
    }
}

pub struct AssetLoadContext<'assets> {
    pub loader: &'assets mut AssetLoader,
}

struct TypedAssetLoader<K, T: Asset> {
    assets: Storage<T>,
    lookup: HashMap<K, Handle<T>>,
}

impl<K, T: Asset> Default for TypedAssetLoader<K, T> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
            lookup: Default::default(),
        }
    }
}

impl<K: Eq + Hash, T: Asset> TypedAssetLoader<K, T> {
    pub fn insert(&mut self, key: K, asset: T) -> (Handle<T>, &T) {
        let handle = self.assets.insert(asset);
        self.lookup.insert(key, handle);
        (handle, self.assets.get(handle).unwrap())
    }
}

pub fn load_config<C: From<ConfigLines>>(path: impl AsRef<Path>) -> Result<C, AssetError> {
    let data = file_system().load(path)?;
    let text = String::from_utf8_lossy(&data);
    let config_lines = ConfigLines::parse(&text);
    Ok(C::from(config_lines))
}
