use bevy_ecs::prelude::*;

use crate::{
    engine::{assets::AssetError, transform::Transform},
    game::scenes::world::sim_world::{dynamic_bvh::DynamicBvh, ecs::BoundingBoxComponent},
};

use super::static_bvh::StaticBvh;

/*
pub enum ObjectData {
    Scenery {
        model: Handle<Model>,
    },
    Biped {
        model: Handle<Model>,
        order_queue: OrderQueue,
        _sequencer: Sequencer,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel {
        model: Handle<Model>,
    },
}

impl ObjectData {
    fn interact(&mut self, hit: &_InteractionHit) {
        match self {
            ObjectData::Scenery { .. } => {}
            ObjectData::Biped { order_queue, .. } => match hit {
                _InteractionHit::Terrain { world_position, .. } => {
                    // User clicked on the terrain, order a move.
                    order_queue.enqueue(Order::Move(OrderMove {
                        target_location: *world_position,
                        move_speed: 10.0,
                        rotation_speed: 0.1,
                    }));
                }
                _InteractionHit::Object { .. } => {}
            },
            ObjectData::SingleModel { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RayIntersectionMode {
    _CollisionBoxes,
    Meshes,
}

pub struct Object {
    pub name: String,
    pub title: String,
    pub transform: Transform,
    pub bounding_box: BoundingBox,
    pub data: ObjectData,
}

impl Object {
    /// Intersect this object with a world-space ray segment using the selected model data.
    pub fn ray_intersection(
        &self,
        ray_segment: &RaySegment,
        mode: RayIntersectionMode,
    ) -> Option<ModelRayHit> {
        // Quad tree already applied coarse culling; do only fine model test here.
        let object_to_world = self.transform.to_mat4();

        let model_handle = match &self.data {
            ObjectData::Scenery { model }
            | ObjectData::Biped { model, .. }
            | ObjectData::SingleModel { model } => *model,
        };

        let model = models().get(model_handle)?;
        match mode {
            RayIntersectionMode::_CollisionBoxes => {
                model.intersect_ray_segment_with_transform(object_to_world, ray_segment)
            }
            RayIntersectionMode::Meshes => model.intersect_ray_segment_meshes_with_transform(
                object_to_world,
                ray_segment,
                false,
            ),
        }
    }

    /// The user is interacting with the object.
    pub fn interact(&mut self, hit: &_InteractionHit) {
        self.data.interact(hit);
    }
}
*/

#[derive(Resource)]
pub struct Objects {
    pub static_bvh: StaticBvh<Entity>,

    bounding_boxes_query: QueryState<(Entity, &'static Transform, &'static BoundingBoxComponent)>,
}

impl Objects {
    pub fn new(world: &mut World) -> Result<Self, AssetError> {
        let static_bvh = StaticBvh::new(8);

        Ok(Self {
            static_bvh,
            bounding_boxes_query: world.query(),
        })
    }

    pub fn finalize(&mut self, world: &World) {
        // TODO: Filter out static/dynamic objects.

        let bounding_boxes = self
            .bounding_boxes_query
            .query(world)
            .iter()
            .map(|(entity, transform, bounding_box)| {
                let bounding_box = bounding_box.0.transformed(transform.to_mat4());
                (entity, bounding_box)
            })
            .collect::<Vec<_>>();

        self.static_bvh.rebuild(&bounding_boxes);
    }
}
