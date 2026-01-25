use std::path::PathBuf;

use ahash::HashSet;
use bevy_ecs::prelude::*;
use bevy_ecs::resource::Resource;
use glam::{IVec2, Quat, Vec2, Vec3};

use crate::{
    engine::{assets::AssetError, input::InputState, transform::Transform},
    game::{
        AssetLoader,
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        scenes::world::{
            sim_world::{
                ecs::{ActiveCamera, GizmoVertices, Viewport},
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
    pub time_of_day: f32,

    /// A list of chunks that should be highlighted during rendering.
    pub highlighted_chunks: HashSet<IVec2>,

    pub selected_objects: HashSet<Entity>,

    pub _sequences: Sequences,

    pub ui: Ui,
}

pub fn init_sim_world(
    world: &mut World,
    assets: &mut AssetLoader,
    campaign_def: &CampaignDef,
) -> Result<(), AssetError> {
    let campaign = assets.load_campaign(&campaign_def.base_name)?;

    world.init_resource::<Time>();
    world.init_resource::<InputState>();

    world.init_resource::<WorldInteraction>();

    world.init_resource::<super::extract::RenderSnapshot>();

    world.insert_resource(GizmoVertices::with_capacity(1024));

    let time_of_day = 12.0;

    world.insert_resource(day_night_cycle::DayNightCycle::from_campaign(&campaign));

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

    init_terrain(world, assets, campaign_def)?;

    init_objects(world, assets, campaign)?;

    let sequences = Sequences::new(assets)?;

    let ui = Ui::new();

    let sim_world_state = SimWorldState {
        time_of_day,

        highlighted_chunks: HashSet::default(),

        selected_objects: HashSet::default(),

        // gizmo_vertices: Vec::with_capacity(1024),
        _sequences: sequences,

        ui,
    };

    world.insert_resource(sim_world_state);

    world.init_resource::<Viewport>();

    Ok(())
}

fn init_terrain(
    world: &mut World,
    assets: &mut AssetLoader,
    campaign_def: &CampaignDef,
) -> Result<(), AssetError> {
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

        let terrain_texture = {
            let path = PathBuf::from("trnhigh")
                .join(&terrain_mapping.texture_map_base_name)
                .with_extension("jpg");
            let (handle, _) = assets.get_or_load_image(path)?;
            handle
        };

        let strata_texture = {
            let path = PathBuf::from("textures").join("shared").join("strata.bmp");
            let (handle, _) = assets.get_or_load_image(path)?;
            handle
        };

        Terrain::new(height_map, terrain_texture, strata_texture)
    };
    world.insert_resource(terrain);
    Ok(())
}

fn init_objects(
    world: &mut World,
    assets: &mut AssetLoader,
    campaign: crate::game::config::Campaign,
) -> Result<(), AssetError> {
    world.insert_resource(StaticBvh::new(8));
    world.insert_resource(DynamicBvh::default());

    let mut object_spawner = spawner::Spawner::new(data_dir().load_character_profiles()?);
    if let Some(ref mtf_name) = campaign.mtf_name {
        let mtf = data_dir().load_mtf(mtf_name)?;

        for object in mtf.objects.iter() {
            let object_type = ObjectType::from_string(&object.typ)
                .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

            let transform = Transform::from_translation(object.position)
                .with_euler_rotation(object.rotation * Vec3::new(1.0, 1.0, -1.0));

            let _ = match object_spawner.spawn(
                world,
                assets,
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
