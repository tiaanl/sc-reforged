use glam::{IVec2, UVec2, Vec3, Vec4};

use crate::{
    engine::{prelude::InputState, storage::Handle},
    game::{
        math::RaySegment,
        scenes::world::sim_world::{Object, SimWorld},
    },
};

#[derive(Debug)]
pub enum InteractionHit {
    Terrain {
        _world_position: Vec3,
        distance: f32,
        chunk_coord: IVec2,
    },
    Object {
        _world_position: Vec3,
        distance: f32,
        object: Handle<Object>,
    },
}

impl InteractionHit {
    pub fn distance(&self) -> f32 {
        match self {
            InteractionHit::Terrain {
                distance: _distance,
                ..
            }
            | InteractionHit::Object {
                distance: _distance,
                ..
            } => *_distance,
        }
    }
}

pub struct WorldInteractionSystem;

impl WorldInteractionSystem {
    pub fn input(&self, sim_world: &mut SimWorld, input_state: &InputState, viewport_size: UVec2) {
        sim_world.highlighted_chunks.clear();
        sim_world.highlighted_objects.clear();
        let _color = Vec4::new(1.0, 0.0, 0.0, 1.0);

        if false {
            if let Some(mouse_position) = input_state.mouse_position() {
                let camera_ray_segment = sim_world
                    .computed_camera
                    .create_ray_segment(mouse_position.as_uvec2(), viewport_size);

                if let Some(hit) =
                    Self::get_interaction_hit(sim_world, &camera_ray_segment, |_| true)
                {
                    match hit {
                        InteractionHit::Terrain { chunk_coord, .. } => {
                            sim_world.highlighted_chunks.insert(chunk_coord);
                        }
                        InteractionHit::Object { object, .. } => {
                            sim_world.highlighted_objects.insert(object);
                        }
                    }
                }
            }
        }
    }

    fn get_interaction_hit(
        sim_world: &SimWorld,
        camera_ray_segment: &RaySegment,
        object_pred: impl Fn(&Object) -> bool,
    ) -> Option<InteractionHit> {
        let mut hits = Vec::default();

        sim_world
            .quad_tree
            .with_nodes_ray_segment(camera_ray_segment, |node| {
                if let Some(chunk_coord) = node.chunk_coord {
                    if let Some(hit) = sim_world
                        .terrain
                        .chunk_intersect_ray_segment(chunk_coord, camera_ray_segment)
                    {
                        hits.push(InteractionHit::Terrain {
                            _world_position: hit.world_position,
                            distance: hit.t,
                            chunk_coord,
                        });
                    }
                }

                for object_handle in node.objects.iter() {
                    if let Some(object) = sim_world.objects.get(*object_handle) {
                        if !object_pred(object) {
                            continue;
                        }
                        if let Some(hit) = object.ray_intersection(camera_ray_segment) {
                            hits.push(InteractionHit::Object {
                                _world_position: hit.world_position,
                                distance: hit.t,
                                object: *object_handle,
                            });
                        }
                    }
                }
            });

        // Reverse sort the hits...
        hits.sort_by(|a, b| b.distance().partial_cmp(&a.distance()).unwrap());

        // ...and return the last one.
        hits.pop()
    }
}
