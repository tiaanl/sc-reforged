use glam::{IVec2, UVec2, Vec3, Vec4};

use crate::{
    engine::{prelude::InputState, storage::Handle},
    game::{
        math::RaySegment,
        scenes::world::sim_world::{Object, SimWorld, UiRect},
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

pub struct SelectionRect {
    /// The position where the rect was dragged from.
    pub pos: UVec2,
    /// The current size of the rect. We use an IVec, because we store negative
    /// size if the rect ends above or left of the `start_drag` location.
    pub size: IVec2,
}

#[derive(Default)]
pub struct WorldInteractionSystem {
    selection_rect: Option<SelectionRect>,
}

impl WorldInteractionSystem {
    pub fn input(
        &mut self,
        sim_world: &mut SimWorld,
        input_state: &InputState,
        viewport_size: UVec2,
    ) {
        use winit::event::MouseButton;

        if input_state.mouse_just_pressed(MouseButton::Left) {
            self.selection_rect = input_state.mouse_position().map(|pos| SelectionRect {
                pos,
                size: IVec2::ZERO,
            });
        } else if input_state.mouse_just_released(MouseButton::Left) {
            self.selection_rect = None;
        } else if let Some(ref mut selection_rect) = self.selection_rect {
            if let Some(mouse_position) = input_state.mouse_position() {
                selection_rect.size = mouse_position.as_ivec2() - selection_rect.pos.as_ivec2();
            }
        }

        if false {
            sim_world.highlighted_chunks.clear();
            sim_world.highlighted_objects.clear();
            if let Some(mouse_position) = input_state.mouse_position() {
                let camera_ray_segment = sim_world
                    .computed_camera
                    .create_ray_segment(mouse_position, viewport_size);

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

    pub fn update(&mut self, sim_world: &mut SimWorld) {
        if let Some(rect) = &self.selection_rect {
            let mut pos = rect.pos.as_ivec2();
            let mut size = rect.size;

            if size.x < 0 {
                size.x = -size.x;
                pos.x -= size.x;
            }

            if size.y < 0 {
                size.y = -size.y;
                pos.y -= size.y;
            }

            let pos = pos.as_uvec2();
            let size = size.as_uvec2();

            const DRAG_THRESHOLD: u32 = 2;

            if size.x > DRAG_THRESHOLD && size.y > DRAG_THRESHOLD {
                const THICKNESS: u32 = 1;
                debug_assert!(THICKNESS <= DRAG_THRESHOLD);

                sim_world.ui.ui_rects.push(UiRect {
                    pos,
                    size,
                    color: Vec4::new(0.0, 0.0, 0.0, 0.5),
                });

                // Left
                sim_world.ui.ui_rects.push(UiRect {
                    pos,
                    size: UVec2::new(THICKNESS, size.y),
                    color: Vec4::new(1.0, 1.0, 1.0, 0.5),
                });

                // Right
                sim_world.ui.ui_rects.push(UiRect {
                    pos: UVec2::new(pos.x + size.x - THICKNESS, pos.y),
                    size: UVec2::new(THICKNESS, size.y),
                    color: Vec4::new(1.0, 1.0, 1.0, 0.5),
                });

                // Top
                sim_world.ui.ui_rects.push(UiRect {
                    pos: UVec2::new(pos.x + THICKNESS, pos.y),
                    size: UVec2::new(size.x.max(THICKNESS * 2) - THICKNESS * 2, THICKNESS),
                    color: Vec4::new(1.0, 1.0, 1.0, 0.5),
                });

                // Bottom
                sim_world.ui.ui_rects.push(UiRect {
                    pos: UVec2::new(
                        pos.x + THICKNESS,
                        (pos.y + size.y).max(THICKNESS) - THICKNESS,
                    ),
                    size: UVec2::new(size.x.max(THICKNESS * 2) - THICKNESS * 2, THICKNESS),
                    color: Vec4::new(1.0, 1.0, 1.0, 0.5),
                });
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
