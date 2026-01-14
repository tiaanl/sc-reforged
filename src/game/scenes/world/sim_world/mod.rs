use std::path::PathBuf;

use ahash::HashSet;
use bevy_ecs::prelude::*;
use bevy_ecs::resource::Resource;
use glam::{IVec2, Quat, Vec2, Vec3};

use crate::{
    engine::{assets::AssetError, input::InputState, transform::Transform},
    game::{
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        scenes::world::{
            sim_world::{
                ecs::{ActiveCamera, GizmoVertices, Snapshots, Viewport},
                free_camera_controller::FreeCameraController,
                sequences::Sequences,
                top_down_camera_controller::TopDownCameraController,
                ui::Ui,
            },
            systems::{Time, world_interaction::WorldInteraction},
        },
    },
};

mod camera;
mod day_night_cycle;
mod dynamic_bvh;
pub mod ecs;
pub mod free_camera_controller;
mod height_map;
mod order_queue;
mod orders;
mod quad_tree;
mod sequences;
mod spawner;
mod static_bvh;
mod terrain;
pub mod top_down_camera_controller;
mod ui;

pub use camera::Camera;
pub use camera::ComputedCamera;
pub use day_night_cycle::DayNightCycle;
pub use dynamic_bvh::{DynamicBvh, DynamicBvhHandle};
pub use height_map::HeightMap;
pub use orders::Order;
pub use static_bvh::{StaticBvh, StaticBvhHandle};
pub use terrain::Terrain;
pub use ui::UiRect;

#[derive(Resource)]
pub struct SimWorldState {
    /// Instant that the simulation started.
    pub sim_start: std::time::Instant,

    pub time_of_day: f32,

    /// A list of chunks that should be highlighted during rendering.
    pub highlighted_chunks: HashSet<IVec2>,

    pub selected_objects: HashSet<Entity>,

    pub _sequences: Sequences,

    pub ui: Ui,
}

/// Holds all the data for the world we are simulating.
pub struct SimWorld {
    pub ecs: World,
}

impl SimWorld {
    pub fn new(campaign_def: &CampaignDef) -> Result<Self, AssetError> {
        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let mut ecs = World::default();

        ecs.init_resource::<Time>();
        ecs.init_resource::<InputState>();

        ecs.init_resource::<WorldInteraction>();

        ecs.init_resource::<Snapshots>();

        ecs.insert_resource(GizmoVertices::with_capacity(1024));

        let time_of_day = 12.0;

        ecs.insert_resource(day_night_cycle::DayNightCycle::from_campaign(&campaign));

        // Cameras

        ecs.spawn((
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

        ecs.spawn((
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

        init_terrain(&mut ecs, campaign_def)?;

        init_objects(&mut ecs, campaign)?;

        let sequences = Sequences::new()?;

        let ui = Ui::new();

        let sim_world_state = SimWorldState {
            sim_start: std::time::Instant::now(),

            time_of_day,

            highlighted_chunks: HashSet::default(),

            selected_objects: HashSet::default(),

            // gizmo_vertices: Vec::with_capacity(1024),
            _sequences: sequences,

            ui,
        };

        ecs.insert_resource(sim_world_state);

        ecs.init_resource::<Viewport>();

        Ok(SimWorld { ecs })
    }

    #[inline]
    pub fn state(&self) -> &SimWorldState {
        self.ecs.resource::<SimWorldState>()
    }

    #[inline]
    pub fn state_mut(&mut self) -> Mut<'_, SimWorldState> {
        self.ecs.resource_mut::<SimWorldState>()
    }
}

fn init_terrain(ecs: &mut World, campaign_def: &CampaignDef) -> Result<(), AssetError> {
    let terrain = {
        let terrain_mapping = data_dir().load_terrain_mapping(&campaign_def.base_name)?;

        let height_map = {
            let path = PathBuf::from("maps").join(format!("{}.pcx", &campaign_def.base_name));
            tracing::info!("Loading terrain height map: {}", path.display());
            data_dir().load_new_height_map(
                path,
                terrain_mapping.altitude_map_height_base,
                terrain_mapping.nominal_edge_size,
            )?
        };

        let terrain_texture =
            data_dir().load_terrain_texture(&terrain_mapping.texture_map_base_name)?;

        Terrain::new(height_map, terrain_texture)
    };
    ecs.insert_resource(terrain);
    Ok(())
}

fn init_objects(
    ecs: &mut World,
    campaign: crate::game::config::Campaign,
) -> Result<(), AssetError> {
    ecs.insert_resource(StaticBvh::new(8));
    ecs.insert_resource(DynamicBvh::default());

    let mut object_spawner = spawner::Spawner::new(data_dir().load_character_profiles()?);
    if let Some(ref mtf_name) = campaign.mtf_name {
        let mtf = data_dir().load_mtf(mtf_name)?;

        for object in mtf.objects.iter() {
            let object_type = ObjectType::from_string(&object.typ)
                .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

            let transform = Transform::from_translation(object.position)
                .with_euler_rotation(object.rotation * Vec3::new(1.0, 1.0, -1.0));

            let _ = match object_spawner.spawn(
                ecs,
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
