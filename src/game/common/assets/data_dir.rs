use std::path::{Path, PathBuf};

use shadow_company_tools::bmf;

use crate::{
    engine::assets::{Asset, AssetError, assets},
    game::{config, image::Image, model::Model},
};

use super::Config;

/// Load game specific assets from [Assets].
#[derive(Clone)]
pub struct DataDir {}

impl DataDir {
    pub fn load_campaign_defs() -> Result<Asset<config::CampaignDefs>, AssetError> {
        assets().load(PathBuf::from("config").join("campaign_defs.txt"))
    }

    pub fn load_config<C: Config>(path: impl AsRef<Path>) -> Result<Asset<C>, AssetError> {
        assets().load(path)
    }

    pub fn load_campaign(campaign: &str) -> Result<Asset<config::Campaign>, AssetError> {
        assets().load(
            PathBuf::from("campaign")
                .join(campaign)
                .join(campaign)
                .with_extension("txt"),
        )
    }

    pub fn load_image(path: impl AsRef<Path>) -> Result<Asset<Image>, AssetError> {
        assets().load(path)
    }

    pub fn load_model_by_path(path: impl AsRef<Path>) -> Result<Asset<Image>, AssetError> {
        assets().load(path)
    }

    pub fn load_object_model(name: &str) -> Result<Asset<Model>, AssetError> {
        let path = PathBuf::from("models")
            .join(name)
            .join(name)
            .with_extension("smf");
        assets().load(path)
    }

    pub fn load_bipedal_model(name: &str) -> Result<Asset<Model>, AssetError> {
        let path = PathBuf::from("models")
            .join("people")
            .join("bodies")
            .join(name)
            .join(name)
            .with_extension("smf");
        assets().load(path)
    }

    pub fn load_motion(path: impl AsRef<Path>) -> Result<Asset<bmf::Motion>, AssetError> {
        assets().load(path)
    }
}
