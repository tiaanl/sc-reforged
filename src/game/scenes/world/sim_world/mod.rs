use std::path::PathBuf;

use ahash::HashSet;
use bevy_ecs::{lifecycle::HookContext, schedule::Schedule, world::World};
use glam::{IVec2, Quat, Vec3, vec3};
use strum::{EnumCount, EnumIter};

use crate::{
    engine::{assets::AssetError, gizmos::GizmoVertex, prelude::Transform, storage::Handle},
    game::{
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        model::Model,
        models::{ModelName, models},
        scenes::world::{
            animation::motion::Motion,
            render::RenderStore,
            sim_world::{ecs::PendingRenderStore, quad_tree::QuadTree, ui::Ui},
        },
        track::Track,
    },
};

mod camera;
mod ecs;
mod height_map;
mod quad_tree;
mod spawner;
mod terrain;
mod ui;

pub use camera::{Camera, ComputedCamera};
pub use ecs::GizmoVertices;
pub use height_map::HeightMap;
pub use terrain::Terrain;
pub use ui::UiRect;

/// Holds data for the sun and fog values throughout the day and night.
#[derive(Default)]
pub struct DayNightCycle {
    pub sun_dir: Track<Vec3>,
    pub sun_color: Track<Vec3>,

    pub fog_distance: Track<f32>,
    pub fog_near_fraction: Track<f32>,
    pub fog_color: Track<Vec3>,
}

#[derive(Clone, Copy, EnumCount, EnumIter, PartialEq)]
#[repr(usize)]
pub enum ActiveCamera {
    Game = 0,
    Debug = 1,
}

/// Holds all the data for the world we are simulating.
pub struct SimWorld {
    /// The ECS hodling the objects in the world.
    pub world: World,

    /// Schedule for running update systems.
    pub update_schedule: Schedule,

    /// Data for each camera.
    pub cameras: [Camera; ActiveCamera::COUNT],

    /// Computed values for the camera the player is viewing.
    pub computed_cameras: [ComputedCamera; ActiveCamera::COUNT],

    pub active_camera: ActiveCamera,

    pub time_of_day: f32,
    pub day_night_cycle: DayNightCycle,

    /// Used for determining visible elements in the world.
    pub quad_tree: QuadTree,

    pub terrain: Terrain,
    /// A list of chunks that should be highlighted during rendering.
    pub highlighted_chunks: HashSet<IVec2>,
    /// The visible chunks for the current frame.
    pub visible_chunks: Vec<IVec2>,

    pub gizmo_vertices: Vec<GizmoVertex>,

    pub test_model: Handle<Model>,
    pub test_motion: Motion,
    pub timer: f32,

    pub ui: Ui,
}

impl SimWorld {
    pub fn new(campaign_def: &CampaignDef) -> Result<Self, AssetError> {
        let campaign = data_dir().load_campaign(&campaign_def.base_name)?;

        let time_of_day = 12.0;
        let day_night_cycle = {
            let mut result = DayNightCycle::default();

            campaign.time_of_day.iter().enumerate().for_each(|(i, t)| {
                let index = i as u32;

                result.sun_dir.insert(index, t.sun_dir);
                result.sun_color.insert(index, t.sun_color);

                result.fog_distance.insert(index, t.fog_distance);
                result.fog_near_fraction.insert(index, t.fog_near_fraction);
                result.fog_color.insert(index, t.fog_color);
            });

            result
        };

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

        let terrain = {
            let terrain_texture =
                data_dir().load_terrain_texture(&terrain_mapping.texture_map_base_name)?;

            Terrain::new(height_map, terrain_texture)
        };

        let quad_tree = QuadTree::from_terrain(&terrain);

        let mut world = World::default();

        world.init_resource::<GizmoVertices>();

        world.init_resource::<PendingRenderStore>();

        world.register_component_hooks::<Handle<Model>>().on_insert(
            |mut world, HookContext { entity, .. }| {
                if let Some(model_handle) = world.get::<Handle<Model>>(entity).cloned() {
                    let mut pending_render_store = world.resource_mut::<PendingRenderStore>();
                    pending_render_store.models.insert(model_handle);
                }
            },
        );

        let update_schedule = Schedule::new(ecs::UpdateSchedule);

        let character_profiles = data_dir().load_character_profiles()?;
        let mut spawner = spawner::Spawner::new(character_profiles, &mut world);

        if let Some(ref mtf_name) = campaign.mtf_name {
            let mtf = data_dir().load_mtf(mtf_name)?;

            for object in mtf.objects.iter() {
                let object_type = ObjectType::from_string(&object.typ)
                    .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

                let transform = Transform::from_translation(object.position)
                    .with_euler_rotation(object.rotation * vec3(1.0, 1.0, -1.0));

                let _ = match spawner.spawn(transform, object_type, &object.name, &object.title) {
                    Ok(handle) => handle,
                    Err(err) => {
                        tracing::warn!("Could not spawn object! ({})", err);
                        continue;
                    }
                };

                // Insert the object into the quad tree.
                // quad_tree.insert_object(object_handle, &object.bounding_sphere);
            }
        }

        // quad_tree._print_nodes();

        let (test_model, _) = models().load_model(ModelName::Body(String::from("man1_enemy")))?;
        let test_motion = data_dir().load_motion("bipedal_stand_idle_smoke")?;

        let ui = Ui::new();

        Ok(SimWorld {
            world,
            update_schedule,

            cameras: [
                Camera::new(
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    45.0_f32.to_radians(),
                    1.0,
                    10.0,
                    13_300.0,
                ),
                Camera::new(
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    45.0_f32.to_radians(),
                    1.0,
                    10.0,
                    13_300.0,
                ),
            ],
            computed_cameras: [ComputedCamera::default(), ComputedCamera::default()],

            active_camera: ActiveCamera::Game,

            time_of_day,
            day_night_cycle,

            quad_tree,

            terrain,
            highlighted_chunks: HashSet::default(),
            visible_chunks: Vec::default(),

            gizmo_vertices: Vec::with_capacity(1024),

            test_model,
            test_motion,
            timer: 0.0,

            ui,
        })
    }

    /// Handle all pending render store resources.
    pub fn process_pending_render_store(&mut self, render_store: &mut RenderStore) {
        if !self.world.is_resource_changed::<PendingRenderStore>() {
            return;
        }

        let mut pending = HashSet::default();
        let mut current = self.world.resource_mut::<PendingRenderStore>();
        std::mem::swap(&mut pending, &mut current.models);

        for model_handle in pending.drain() {
            if let Err(err) = render_store.get_or_create_render_model(model_handle) {
                tracing::warn!("Could not create render model! ({err})");
            }
        }
    }
}
