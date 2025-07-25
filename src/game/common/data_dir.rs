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
        image::{BlendMode, Image},
        model::Model,
    },
};

#[derive(Clone)]
pub struct DataDir {}

impl DataDir {
    pub fn load_campaign_defs() -> Result<config::CampaignDefs, AssetError> {
        Self::load_config_new(PathBuf::from("config").join("campaign_defs.txt"))
    }

    pub fn load_campaign(campaign: &str) -> Result<config::Campaign, AssetError> {
        Self::load_config_new::<config::Campaign>(
            PathBuf::from("campaign")
                .join(campaign)
                .join(campaign)
                .with_extension("txt"),
        )
    }

    pub fn load_terrain_mapping(campaign: &str) -> Result<TerrainMapping, AssetError> {
        let path = PathBuf::from("textures")
            .join("terrain")
            .join(campaign)
            .join("terrain_mapping.txt");

        tracing::info!("Loading terrain mapping: {}", path.display());

        Self::load_config_new(path)
    }

    pub fn load_height_map(path: impl AsRef<Path>) -> Result<HeightMap, AssetError> {
        HeightMap::from_data(file_system().load(path.as_ref())?)
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))
    }

    pub fn load_lod_model_profiles() -> Result<HashMap<String, Vec<SubModelDefinition>>, AssetError>
    {
        let mut lod_definitions: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

        let profiles_path = PathBuf::from("config").join("lod_model_profiles");
        for lod_path in file_system().dir(&profiles_path)?.filter(|path| {
            path.extension()
                .filter(|ext| ext.eq_ignore_ascii_case("txt"))
                .is_some()
        }) {
            let profile = Self::load_config_new::<LodModelProfileDefinition>(lod_path)?;
            lod_definitions.insert(
                profile.lod_model_name.clone(),
                profile.sub_model_definitions.clone(),
            );
        }

        Ok(lod_definitions)
    }

    pub fn load_mtf(name: &str) -> Result<config::Mtf, AssetError> {
        let path = PathBuf::from("maps").join(name);
        Self::load_config_new::<config::Mtf>(&path)
    }

    pub fn load_image(path: impl AsRef<Path>) -> Result<Image, AssetError> {
        fn image_error_to_asset_error(err: image::ImageError, path: PathBuf) -> AssetError {
            match err {
                image::ImageError::Decoding(_) => AssetError::Decode(path),
                image::ImageError::Encoding(_) => {
                    AssetError::Unknown(path, String::from("ImageError::Encoding"))
                }
                image::ImageError::Parameter(_) => {
                    AssetError::Unknown(path, String::from("ImageError::Encoding"))
                }
                image::ImageError::Limits(_) => {
                    AssetError::Unknown(path, String::from("ImageError::Encoding"))
                }
                image::ImageError::Unsupported(_) => {
                    AssetError::Unknown(path, String::from("ImageError::Encoding"))
                }
                image::ImageError::IoError(error) => AssetError::from_io_error(error, &path),
            }
        }

        let is_color_keyd = path
            .as_ref()
            .file_name()
            .filter(|n| n.to_string_lossy().contains("_ck"))
            .is_some();

        let ext = path.as_ref().extension().unwrap().to_ascii_lowercase();

        if ext == "bmp" {
            let data = file_system().load(path.as_ref())?;
            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(data),
                is_color_keyd,
            )
            .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?;

            let raw = if let Ok(data) = file_system().load(path.as_ref().with_extension("raw")) {
                Some(
                    shadow_company_tools::images::load_raw_file(
                        &mut std::io::Cursor::new(data),
                        bmp.width(),
                        bmp.height(),
                    )
                    .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?,
                )
            } else {
                None
            };

            return Ok(if is_color_keyd {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::ColorKeyed,
                )
            } else if let Some(raw) = raw {
                Image::from_rgba(
                    shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw),
                    BlendMode::Alpha,
                )
            } else {
                Image::from_rgba(
                    image::DynamicImage::from(bmp).into_rgba8(),
                    BlendMode::Opaque,
                )
            });
        } else if ext == "jpg" || ext == "jpeg" {
            let data = file_system().load(path.as_ref())?;
            let image = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .map_err(|err| image_error_to_asset_error(err, path.as_ref().to_path_buf()))?;

            return Ok(Image::from_rgba(image.into_rgba8(), BlendMode::Opaque));
        }

        Err(AssetError::NotSupported(path.as_ref().to_path_buf()))
    }

    fn load_model(path: impl AsRef<Path>) -> Result<Model, AssetError> {
        let data = file_system().load(path.as_ref())?;

        let smf = shadow_company_tools::smf::Model::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        Model::try_from(smf)
    }

    pub fn load_object_model(name: &str) -> Result<Model, AssetError> {
        Self::load_model(
            PathBuf::from("models")
                .join(name)
                .join(name)
                .with_extension("smf"),
        )
    }

    pub fn load_bipedal_model(name: &str) -> Result<Model, AssetError> {
        Self::load_model(
            PathBuf::from("models")
                .join("people")
                .join("bodies")
                .join(name)
                .join(name)
                .with_extension("smf"),
        )
    }

    pub fn load_motion(path: impl AsRef<Path>) -> Result<bmf::Motion, AssetError> {
        let data = file_system().load(path.as_ref())?;

        bmf::Motion::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))
    }

    fn load_config_new<C: From<ConfigLines>>(path: impl AsRef<Path>) -> Result<C, AssetError> {
        let data = file_system().load(path)?;
        let text = String::from_utf8_lossy(&data);
        let config_lines = ConfigLines::parse(&text);
        Ok(C::from(config_lines))
    }
}
