use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ahash::HashSet;
use bevy_ecs::prelude::*;
use glam::{IVec2, Quat, Vec2, Vec3};

use crate::{
    engine::{
        assets::AssetError,
        input::{InputEvent, InputState},
        transform::Transform,
    },
    game::{
        assets::{
            config::campaign_def::CampaignDef, images::Images, models::Models, motions::Motions,
        },
        config::{CharacterProfiles, Mtf, ObjectType, TerrainMapping, load_config},
        globals,
        render::world::WorldRenderSnapshot,
    },
};

use ecs::{ActiveCamera, GizmoVertices, Viewport};
use free_camera_controller::FreeCameraController;
use orders::OrderRequest;
use sequences::MotionSequencer;
use systems::{
    SimulationControl, Time, build_extract_schedule, build_update_schedule,
    world_interaction::{self, WorldInteraction},
};
use top_down_camera_controller::TopDownCameraController;
use ui::Ui;

mod camera;
mod day_night_cycle;
mod dynamic_bvh;
pub mod ecs;
pub mod extract;
pub mod free_camera_controller;
mod height_map;
pub mod orders;
mod quad_tree;
pub mod sequences;
mod spawner;
mod static_bvh;
pub mod systems;
mod terrain;
pub mod top_down_camera_controller;
mod ui;

pub use camera::Camera;
pub use camera::ComputedCamera;
pub use day_night_cycle::DayNightCycle;
pub use dynamic_bvh::{DynamicBvh, DynamicBvhHandle};
pub use height_map::HeightMap;
pub use static_bvh::{StaticBvh, StaticBvhHandle};
pub use terrain::Terrain;
pub use ui::UiRect;

pub struct SimWorld {
    world: World,
    update_schedule: Schedule,
    extract_schedule: Schedule,
}

impl SimWorld {
    pub fn new(assets: GameAssets, campaign_def: &CampaignDef) -> Result<Self, AssetError> {
        let mut world = World::default();
        init_sim_world(&mut world, assets, campaign_def)?;

        let update_schedule = build_update_schedule();
        let extract_schedule = build_extract_schedule();

        Ok(Self {
            world,
            update_schedule,
            extract_schedule,
        })
    }

    #[inline]
    pub fn terrain(&self) -> &Terrain {
        self.world.resource::<Terrain>()
    }

    /// Update the viewport size used by camera systems.
    pub fn resize_viewport(&mut self, size: glam::UVec2) {
        self.world.resource_mut::<Viewport>().resize(size);
    }

    /// Forward an input event into the simulation's `InputState`.
    pub fn input(&mut self, event: &InputEvent) {
        self.world.resource_mut::<InputState>().apply(event);
    }

    /// Advance the simulation by `delta_time` seconds.
    pub fn update(&mut self, delta_time: f32) {
        self.world.resource_mut::<Time>().next_frame(delta_time);
        self.update_schedule.run(&mut self.world);
    }

    /// Run the extract schedule to populate the snapshot, then return a
    /// reference to it. The snapshot is owned by the simulation `World`; a
    /// later call will overwrite it.
    pub fn extract_snapshot(&mut self) -> &WorldRenderSnapshot {
        self.extract_schedule.run(&mut self.world);
        self.world.resource::<WorldRenderSnapshot>()
    }
}

#[derive(Resource)]
pub struct SimWorldState {
    pub time_of_day: f32,

    /// A list of chunks that should be highlighted during rendering.
    pub highlighted_chunks: HashSet<IVec2>,

    pub ui: Ui,
}

/// Asset containers shared with the ECS world. Each container is internally
/// thread-safe and may be cloned cheaply.
#[derive(Clone, Resource)]
pub struct GameAssets {
    pub images: Arc<Images>,
    pub models: Arc<Models>,
    pub motions: Arc<Motions>,
}

fn init_sim_world(
    world: &mut World,
    assets: GameAssets,
    campaign_def: &CampaignDef,
) -> Result<(), AssetError> {
    let campaign = load_config::<crate::game::config::Campaign>(
        PathBuf::from("campaign")
            .join(&campaign_def.base_name)
            .join(&campaign_def.base_name)
            .with_extension("txt"),
    )?;

    world.init_resource::<Time>();
    world.init_resource::<SimulationControl>();
    world.init_resource::<InputState>();

    world.init_resource::<WorldInteraction>();

    world.init_resource::<WorldRenderSnapshot>();

    world.insert_resource(GizmoVertices::with_capacity(1024));

    let time_of_day = 12.0;

    world.insert_resource(day_night_cycle::DayNightCycle::from_campaign(&campaign));

    world.add_observer(world_interaction::on_clicked);

    let motion_sequencer = {
        let mut motion_sequencer = MotionSequencer::default();

        motion_sequencer.load_motion_sequencer_defs(
            &assets.motions,
            PathBuf::from("config").join("mot_sequencer_defs.txt"),
        )?;

        motion_sequencer
    };

    world.insert_resource(motion_sequencer);

    world.add_observer(
        |request: On<sequences::MotionSequenceRequest>,
         motion_sequencer: Res<sequences::MotionSequencer>,
         mut motion_controllers: Query<&mut sequences::MotionController>| {
            if request.entity == Entity::PLACEHOLDER {
                tracing::warn!(
                    "Requesting motion sequence on a placeholder entity. {}",
                    request.caller()
                );
                return;
            }

            let Ok(mut motion_controller) = motion_controllers.get_mut(request.entity) else {
                tracing::warn!("Requesting sequence for entity without MotionController");
                return;
            };

            if !motion_sequencer.request_sequence(request.event().clone(), &mut motion_controller) {
                tracing::warn!("Could not request motion sequence {:?}", request.event());
            }
        },
    );

    // Orders
    world.init_resource::<Messages<OrderRequest>>();

    // Cameras

    world.spawn((
        Camera::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            10.0,
            13_300.0,
        ),
        {
            let camera_from = campaign.view_initial.from.extend(2500.0);
            let camera_to = campaign.view_initial.to.extend(0.0);

            let dir = (camera_to - camera_from).normalize();

            let flat = Vec2::new(dir.x, dir.y);
            let yaw = (-dir.x).atan2(dir.y).to_degrees();
            let pitch = dir.z.atan2(flat.length()).to_degrees();

            TopDownCameraController::new(camera_from, yaw, pitch, 4_000.0, 100.0)
        },
        ActiveCamera,
    ));

    world.spawn((
        Camera::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            45.0_f32.to_radians(),
            1.0,
            10.0,
            13_300.0,
        ),
        FreeCameraController::new(1000.0, 0.2),
    ));

    init_terrain(world, &assets, campaign_def)?;

    init_objects(world, &assets, campaign)?;

    let ui = Ui::new();

    let sim_world_state = SimWorldState {
        time_of_day,
        highlighted_chunks: HashSet::default(),
        ui,
    };

    world.insert_resource(sim_world_state);

    world.init_resource::<Viewport>();

    world.insert_resource(assets);

    Ok(())
}

fn init_terrain(
    world: &mut World,
    assets: &GameAssets,
    campaign_def: &CampaignDef,
) -> Result<(), AssetError> {
    let terrain = {
        let terrain_mapping = load_config::<TerrainMapping>(
            PathBuf::from("textures")
                .join("terrain")
                .join(&campaign_def.base_name)
                .join("terrain_mapping.txt"),
        )?;

        pub fn load_new_height_map(
            path: impl AsRef<Path>,
            elevation_scale: f32,
            cell_size: f32,
        ) -> Result<HeightMap, AssetError> {
            use glam::UVec2;

            let data = globals::file_system().load(path.as_ref())?;

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

        let height_map = {
            let path = PathBuf::from("maps").join(format!("{}.pcx", &campaign_def.base_name));
            tracing::info!("Loading terrain height map: {}", path.display());
            load_new_height_map(
                path,
                terrain_mapping.altitude_map_height_base,
                terrain_mapping.nominal_edge_size,
            )?
        };

        let terrain_texture = {
            let path = PathBuf::from("trnhigh")
                .join(&terrain_mapping.texture_map_base_name)
                .with_extension("jpg");
            assets.images.load(path)?
        };

        let strata_texture = {
            let path = PathBuf::from("textures").join("shared").join("strata.bmp");
            assets.images.load(path)?
        };

        Terrain::new(height_map, terrain_texture, strata_texture)
    };
    world.insert_resource(terrain);
    Ok(())
}

fn init_objects(
    world: &mut World,
    assets: &GameAssets,
    campaign: crate::game::config::Campaign,
) -> Result<(), AssetError> {
    world.insert_resource(StaticBvh::new(8));
    world.insert_resource(DynamicBvh::default());

    let character_profiles = {
        let mut character_profiles = CharacterProfiles::default();

        for file in globals::file_system()
            .dir(PathBuf::from("config").join("character_profiles"))?
            .filter(|p| {
                if let Some(e) = p.extension() {
                    e.eq_ignore_ascii_case("txt")
                } else {
                    false
                }
            })
        {
            let config = load_config(file)?;
            character_profiles.parse_lines(config);
        }

        character_profiles
    };

    let mut object_spawner = spawner::Spawner::new(character_profiles);

    if let Some(ref mtf_name) = campaign.mtf_name {
        let mtf = load_config::<Mtf>(PathBuf::from("maps").join(mtf_name))?;

        for object in mtf.objects.iter() {
            let object_type = ObjectType::from_string(&object.typ)
                .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

            let transform = Transform::from_translation(object.position)
                .with_euler_rotation(object.rotation * Vec3::new(1.0, 1.0, -1.0));

            let _ = match object_spawner.spawn(
                world,
                &assets.models,
                &object.title,
                &object.name,
                object_type,
                transform.clone(),
            ) {
                Ok(handle) => handle,
                Err(err) => {
                    tracing::warn!("Could not spawn object! ({})", err);
                    continue;
                }
            };
        }
    }

    Ok(())
}
