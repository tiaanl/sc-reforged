use std::path::PathBuf;

use glam::{IVec2, Quat, UVec2, Vec2, Vec3, Vec4, vec3};

use crate::{
    engine::{assets::AssetError, gizmos::GizmoVertex, prelude::Transform, storage::Handle},
    game::{
        camera::{self, Camera},
        config::{CampaignDef, ObjectType},
        data_dir::data_dir,
        math::{Frustum, Ray, RaySegment, ViewProjection},
        scenes::world::{
            objects::{Object, Objects},
            quad_tree::QuadTree,
            terrain::Terrain,
        },
        track::Track,
    },
};

/// Holds data for the sun and fog values throughout the day and night.
#[derive(Default)]
pub struct DayNightCycle {
    pub sun_dir: Track<Vec3>,
    pub sun_color: Track<Vec3>,

    pub fog_distance: Track<f32>,
    pub fog_near_fraction: Track<f32>,
    pub fog_color: Track<Vec3>,
}

#[derive(Default)]
pub struct ComputedCamera {
    pub view_proj: ViewProjection,
    pub frustum: Frustum,
    pub position: Vec3,
    pub forward: Vec3,
}

impl ComputedCamera {
    pub fn create_ray_segment(&self, mouse_position: UVec2, viewport_size: UVec2) -> RaySegment {
        let viewport_pos = (mouse_position.as_vec2() + Vec2::splat(0.5)) / viewport_size.as_vec2();
        let ndc_x = viewport_pos.x * 2.0 - 1.0;
        let ndc_y = 1.0 - viewport_pos.y * 2.0;

        let near_clip = Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_clip = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let mut near_world = self.view_proj.inv * near_clip;
        let mut far_world = self.view_proj.inv * far_clip;

        // Homogeneous divide.
        near_world /= near_world.w;
        far_world /= far_world.w;

        let near_world = near_world.truncate();
        let far_world = far_world.truncate();

        let direction = far_world - near_world;
        RaySegment {
            ray: Ray {
                origin: near_world,
                direction: direction.normalize(),
            },
            distance: direction.length(),
        }
    }
}

/// Holds all the data for the world we are simulating.
pub struct SimWorld {
    pub camera: Camera,
    pub computed_camera: ComputedCamera,

    pub time_of_day: f32,
    pub day_night_cycle: DayNightCycle,

    /// Used for determining visible elements in the world.
    pub quad_tree: QuadTree,

    pub terrain: Terrain,
    /// The visible chunks for the current frame.
    pub visible_chunks: Vec<IVec2>,

    pub objects: Objects,

    /// A list of visible objects this frame.
    pub visible_objects: Vec<Handle<Object>>,

    pub gizmo_vertices: Vec<GizmoVertex>,
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

        let mut quad_tree = QuadTree::from_new_terrain(&terrain);

        let mut objects = Objects::new()?;

        if let Some(ref mtf_name) = campaign.mtf_name {
            let mtf = data_dir().load_mtf(mtf_name)?;

            for object in mtf.objects.iter() {
                let object_type = ObjectType::from_string(&object.typ)
                    .unwrap_or_else(|| panic!("missing object type: {}", object.typ));

                let (object_handle, object) = match objects.spawn(
                    Transform::from_translation(object.position)
                        .with_euler_rotation(object.rotation * vec3(1.0, 1.0, -1.0)),
                    object_type,
                    &object.name,
                    &object.title,
                ) {
                    Ok(handle) => handle,
                    Err(err) => {
                        tracing::warn!("Could not spawn object! ({})", err);
                        continue;
                    }
                };

                // Insert the object into the quad tree.
                quad_tree.insert_object(object_handle, &object.bounding_sphere);
            }
        }

        // quad_tree._print_nodes();

        Ok(SimWorld {
            camera: camera::Camera::new(
                Vec3::ZERO,
                Quat::IDENTITY,
                45.0_f32.to_radians(),
                1.0,
                10.0,
                13_300.0,
            ),
            computed_camera: ComputedCamera::default(),
            time_of_day,
            day_night_cycle,

            quad_tree,

            terrain,
            visible_chunks: Vec::default(),

            objects,
            visible_objects: Vec::default(),

            gizmo_vertices: Vec::with_capacity(1024),
        })
    }
}
