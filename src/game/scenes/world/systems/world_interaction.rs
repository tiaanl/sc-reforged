use glam::{IVec2, UVec2, Vec3, Vec4};
use winit::event::MouseButton;

use crate::{
    engine::{prelude::InputState, storage::Handle},
    game::scenes::world::{objects::Object, sim_world::SimWorld},
};

#[derive(Debug)]
pub enum InteractionHit {
    Terrain {
        world_position: Vec3,
        distance: f32,
        _chunk_coord: IVec2,
    },
    Object {
        world_position: Vec3,
        distance: f32,
        object: Handle<Object>,
    },
}

impl InteractionHit {
    pub fn distance(&self) -> f32 {
        match self {
            InteractionHit::Terrain { distance, .. } | InteractionHit::Object { distance, .. } => {
                *distance
            }
        }
    }
}

pub struct WorldInteractionSystem;

impl WorldInteractionSystem {
    pub fn input(&self, sim_world: &mut SimWorld, input_state: &InputState, viewport_size: UVec2) {
        sim_world.highlighted_chunks.clear();
        let _color = Vec4::new(1.0, 0.0, 0.0, 1.0);

        if input_state.mouse_just_pressed(MouseButton::Left) {
            if let Some(mouse_position) = input_state.mouse_position() {
                let camera_ray_segment = sim_world
                    .computed_camera
                    .create_ray_segment(mouse_position.as_uvec2(), viewport_size);

                let mut hits: Vec<InteractionHit> = Vec::default();

                sim_world
                    .quad_tree
                    .with_nodes_ray_segment(&camera_ray_segment, |node| {
                        if let Some(chunk_coord) = node.chunk_coord {
                            if let Some(hit) = sim_world
                                .terrain
                                .chunk_intersect_ray_segment(chunk_coord, &camera_ray_segment)
                                .first()
                            {
                                hits.push(InteractionHit::Terrain {
                                    world_position: hit.world_position,
                                    distance: hit.t,
                                    _chunk_coord: chunk_coord,
                                });
                                sim_world.highlighted_chunks.insert(chunk_coord);
                            }
                        }

                        for object_handle in node.objects.iter() {
                            if let Some(object) = sim_world.objects.get(*object_handle) {
                                if let Some(hit) = object.ray_intersection(&camera_ray_segment) {
                                    hits.push(InteractionHit::Object {
                                        world_position: hit.world_position,
                                        distance: hit.t,
                                        object: *object_handle,
                                    });
                                }
                            }
                        }
                    });

                hits.sort_by(|a, b| a.distance().partial_cmp(&b.distance()).unwrap());

                if let Some(_hit) = hits.first() {
                    match _hit {
                        InteractionHit::Terrain { world_position, .. } => {
                            println!("terrain at {world_position}")
                        }
                        InteractionHit::Object {
                            world_position,
                            object,
                            ..
                        } => {
                            println!("object ({object}) at {world_position}")
                        }
                    }
                }
            }
        }
    }
}
