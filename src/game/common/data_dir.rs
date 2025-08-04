use std::path::{Path, PathBuf};

use ahash::HashMap;
use shadow_company_tools::bmf;

use crate::{
    engine::assets::AssetError,
    game::{
        config::{
            self, LodModelProfileDefinition, SubModelDefinition, TerrainMapping,
            parser::ConfigLines,
        },
        file_system::file_system,
        height_map::HeightMap,
    },
    global,
};

pub struct DataDir;

impl DataDir {
    pub fn load_campaign_defs(&self) -> Result<config::CampaignDefs, AssetError> {
        self.load_config(PathBuf::from("config").join("campaign_defs.txt"))
    }

    pub fn load_campaign(&self, campaign: &str) -> Result<config::Campaign, AssetError> {
        self.load_config::<config::Campaign>(
            PathBuf::from("campaign")
                .join(campaign)
                .join(campaign)
                .with_extension("txt"),
        )
    }

    pub fn load_terrain_mapping(&self, campaign: &str) -> Result<TerrainMapping, AssetError> {
        let path = PathBuf::from("textures")
            .join("terrain")
            .join(campaign)
            .join("terrain_mapping.txt");

        tracing::info!("Loading terrain mapping: {}", path.display());

        self.load_config(path)
    }

    pub fn load_height_map(&self, path: impl AsRef<Path>) -> Result<HeightMap, AssetError> {
        HeightMap::from_data(file_system().load(path.as_ref())?)
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))
    }

    pub fn load_object_templates(&self) -> Result<config::ObjectTemplates, AssetError> {
        let path = PathBuf::from("config").join("object_templates.txt");
        self.load_config(&path)
    }

    pub fn load_mtf(&self, name: &str) -> Result<config::Mtf, AssetError> {
        let path = PathBuf::from("maps").join(name);
        self.load_config::<config::Mtf>(&path)
    }

    fn load_model_defs(&self) -> Result<HashMap<String, Vec<SubModelDefinition>>, AssetError> {
        let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

        let profiles_path = PathBuf::from("config").join("lod_model_profiles");

        let files = file_system()
            .dir(&profiles_path)
            .expect("Could not load model LOD's.");
        for lod_path in files.filter(|path| {
            path.extension()
                .filter(|ext| ext.eq_ignore_ascii_case("txt"))
                .is_some()
        }) {
            let profile = self
                .load_config::<LodModelProfileDefinition>(lod_path)
                .expect("Could not load model LOD definition.");
            lod_definitions.insert(
                profile.lod_model_name.clone(),
                profile.sub_model_definitions.clone(),
            );
        }

        Ok(lod_definitions)
    }

    pub fn load_motion(&self, path: impl AsRef<Path>) -> Result<bmf::Motion, AssetError> {
        let data = file_system().load(path.as_ref())?;

        bmf::Motion::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))
    }

    pub fn load_config<C: From<ConfigLines>>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<C, AssetError> {
        let data = file_system().load(path)?;
        let text = String::from_utf8_lossy(&data);
        let config_lines = ConfigLines::parse(&text);
        Ok(C::from(config_lines))
    }
}

global!(DataDir, scoped_data_dir, data_dir);
