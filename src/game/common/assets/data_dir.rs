use std::path::{Path, PathBuf};

use shadow_company_tools::{bmf, smf};

use crate::{
    engine::assets::{Asset, AssetError, Assets},
    game::{config, image::Image},
};

use super::Config;

/// Load game specific assets from [Assets].
#[derive(Clone)]
pub struct DataDir {
    assets: Assets,
}

impl DataDir {
    pub fn new(assets: Assets) -> Self {
        Self { assets }
    }

    // #[deprecated]
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn load_campaign_defs(&self) -> Result<Asset<config::CampaignDefs>, AssetError> {
        self.assets
            .load(PathBuf::from("config").join("campaign_defs.txt"))
    }

    pub fn load_config<C: Config>(&self, path: impl AsRef<Path>) -> Result<Asset<C>, AssetError> {
        self.assets.load(path)
    }

    pub fn load_campaign(&self, campaign: &str) -> Result<Asset<config::Campaign>, AssetError> {
        self.assets.load(
            PathBuf::from("campaign")
                .join(campaign)
                .join(campaign)
                .with_extension("txt"),
        )
    }

    pub fn load_image(&self, path: impl AsRef<Path>) -> Result<Asset<Image>, AssetError> {
        self.assets.load(path)
    }

    pub fn load_model_by_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Asset<smf::Model>, AssetError> {
        self.assets.load(path)
    }

    pub fn load_object_model(&self, name: &str) -> Result<Asset<smf::Model>, AssetError> {
        let path = PathBuf::from("models")
            .join(name)
            .join(name)
            .with_extension("smf");
        self.assets.load(path)
    }

    pub fn load_bipedal_model(&self, name: &str) -> Result<Asset<smf::Model>, AssetError> {
        let path = PathBuf::from("models")
            .join("people")
            .join("bodies")
            .join(name)
            .join(name)
            .with_extension("smf");
        self.assets.load(path)
    }

    pub fn load_motion(&self, path: impl AsRef<Path>) -> Result<Asset<bmf::Motion>, AssetError> {
        self.assets.load(path)
    }
}
