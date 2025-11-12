use std::path::{Path, PathBuf};

use ahash::{HashMap, HashMapExt};
use shadow_company_tools::bmf;

use crate::{
    engine::{assets::AssetError, storage::Handle},
    game::{
        common::image::Image,
        config::{self, CharacterProfiles, TerrainMapping, parser::ConfigLines},
        file_system::file_system,
        image::images,
        scenes::world::{animation::motion::Motion, height_map::HeightMap},
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

    #[inline]
    pub fn load_terrain_texture(&self, name: &str) -> Result<Handle<Image>, AssetError> {
        images().load_image(PathBuf::from("trnhigh").join(name).with_extension("jpg"))
    }

    pub fn load_new_height_map(
        &self,
        path: impl AsRef<Path>,
        elevation_scale: f32,
        cell_size: f32,
    ) -> Result<HeightMap, AssetError> {
        use glam::UVec2;

        let data = file_system().load(path.as_ref())?;

        let mut reader = pcx::Reader::from_mem(&data)
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let size = UVec2::new(reader.width() as u32, reader.height() as u32);
        if !reader.is_paletted() {
            return Err(AssetError::custom(path, "PCX file not not paletted!"));
        }

        let mut elevations = vec![0_u8; size.x as usize * size.y as usize];
        for row in 0..size.y {
            let start = row as usize * size.x as usize;
            let end = (row as usize + 1) * size.x as usize;
            let slice = &mut elevations[start..end];
            reader
                .next_row_paletted(slice)
                .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;
        }

        Ok(HeightMap::from_iter(
            size,
            cell_size,
            elevations
                .iter()
                .map(|e| (u8::MAX - *e) as f32 * elevation_scale),
        ))
    }

    pub fn _load_object_templates(&self) -> Result<config::ObjectTemplates, AssetError> {
        let path = PathBuf::from("config").join("object_templates.txt");
        self.load_config(&path)
    }

    pub fn load_mtf(&self, name: &str) -> Result<config::Mtf, AssetError> {
        let path = PathBuf::from("maps").join(name);
        self.load_config::<config::Mtf>(&path)
    }

    pub fn load_motion(&self, name: &str) -> Result<Motion, AssetError> {
        use glam::{Quat, Vec3};

        let path = PathBuf::from("motions").join(name).with_extension("bmf");
        let data = file_system().load(&path)?;

        let bmf = bmf::Motion::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let bones_len = bmf.bone_ids.len();
        let mut motion = Motion {
            bone_ids: bmf.bone_ids.iter().map(|&id| id as _).collect(),
            translations: HashMap::with_capacity(bones_len),
            rotations: HashMap::with_capacity(bones_len),
        };

        fn convert_position(p: Vec3) -> Vec3 {
            Vec3::new(-p.x, p.y, p.z)
        }

        fn convert_rotation(q: Quat) -> Quat {
            Quat::from_xyzw(-q.x, -q.y, -q.z, q.w)
        }

        for bmf_frame in bmf.key_frames.iter() {
            let frame_num = bmf_frame.frame;

            for bone_index in 0..bmf_frame.bones.len() {
                let bone_id = bmf.bone_ids[bone_index];
                let bone = &bmf_frame.bones[bone_index];

                if let Some(translation) = bone.position {
                    motion
                        .translations
                        .entry(bone_id)
                        .or_default()
                        .insert(frame_num, convert_position(translation));
                }

                if let Some(rotation) = bone.rotation {
                    motion
                        .rotations
                        .entry(bone_id)
                        .or_default()
                        .insert(frame_num, convert_rotation(rotation));
                }
            }
        }

        Ok(motion)
    }

    pub fn load_character_profiles(&self) -> Result<CharacterProfiles, AssetError> {
        let mut character_profiles = CharacterProfiles::default();

        for file in file_system()
            .dir(PathBuf::from("config").join("character_profiles"))?
            .filter(|p| {
                if let Some(e) = p.extension() {
                    e.eq_ignore_ascii_case("txt")
                } else {
                    false
                }
            })
        {
            let config = self.load_config(file)?;
            character_profiles.parse_lines(config);
        }

        Ok(character_profiles)
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
