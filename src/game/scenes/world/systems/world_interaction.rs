use glam::{IVec2, Mat4, UVec2, Vec2, Vec3, Vec4};

use crate::{
    engine::{input::InputState, storage::Handle},
    game::{
        math::{Frustum, RaySegment},
        scenes::world::sim_world::{Object, RayIntersectionMode, SimWorld, UiRect},
    },
};

#[derive(Debug)]
pub enum InteractionHit {
    Terrain {
        _world_position: Vec3,
        _distance: f32,
        chunk_coord: IVec2,
    },
    Object {
        _world_position: Vec3,
        _distance: f32,
        object: Handle<Object>,
    },
}

impl InteractionHit {
    pub fn _distance(&self) -> f32 {
        match self {
            InteractionHit::Terrain { _distance, .. }
            | InteractionHit::Object { _distance, .. } => *_distance,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SelectionRect {
    /// The position where the rect was dragged from.
    pub pos: UVec2,
    /// The current size of the rect. We use an IVec, because we store negative
    /// size if the rect ends above or left of the `start_drag` location.
    pub size: IVec2,
}

impl SelectionRect {
    pub fn normalize(self) -> Self {
        let mut pos = self.pos.as_ivec2();
        let mut size = self.size;

        if size.x < 0 {
            size.x = -size.x;
            pos.x -= size.x;
        }

        if size.y < 0 {
            size.y = -size.y;
            pos.y -= size.y;
        }

        debug_assert!(pos.x >= 0 && pos.y >= 0);
        debug_assert!(size.x >= 0 && size.y >= 0);

        Self {
            pos: pos.as_uvec2(),
            size,
        }
    }

    #[inline]
    pub fn min_max(&self) -> (UVec2, UVec2) {
        let n = self.normalize();
        (n.pos, n.pos + n.size.as_uvec2())
    }
}

#[derive(Default)]
pub struct WorldInteractionSystem {
    selection_rect: Option<SelectionRect>,
}

impl WorldInteractionSystem {
    const DRAG_THRESHOLD: u32 = 2;

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
            if let Some(rect) = self.selection_rect {
                if rect.size.x.abs() > Self::DRAG_THRESHOLD as i32
                    && rect.size.y.abs() > Self::DRAG_THRESHOLD as i32
                {
                    self.set_selection_by_rect(sim_world, rect, viewport_size);
                } else {
                    self.set_selection_by_ray(sim_world, rect.pos, viewport_size);
                }
            }

            self.selection_rect = None;
        } else if let Some(ref mut selection_rect) = self.selection_rect
            && let Some(mouse_position) = input_state.mouse_position()
        {
            selection_rect.size = mouse_position.as_ivec2() - selection_rect.pos.as_ivec2();
        }

        if false {
            sim_world.highlighted_chunks.clear();
            sim_world.highlighted_objects.clear();
            if let Some(mouse_position) = input_state.mouse_position() {
                let computed_camera = &sim_world.computed_cameras[sim_world.active_camera as usize];
                let camera_ray_segment =
                    computed_camera.create_ray_segment(mouse_position, viewport_size);

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
            let SelectionRect { pos, size } = rect.normalize();
            let size = size.as_uvec2();

            if size.x > Self::DRAG_THRESHOLD && size.y > Self::DRAG_THRESHOLD {
                const THICKNESS: u32 = 1;
                // debug_assert!(THICKNESS <= Self::DRAG_THRESHOLD);

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

    /// Update the selected objects by using a rectangle in screen coordinates.
    fn set_selection_by_rect(
        &mut self,
        sim_world: &SimWorld,
        rect: SelectionRect,
        viewport_size: UVec2,
    ) {
        let (min, max) = rect.min_max();
        debug_assert!(min.x <= max.x);
        debug_assert!(min.y <= max.y);

        let _frustum = {
            const NDC_Z_NEAR: f32 = 0.0;
            const NDC_Z_FAR: f32 = 1.0;

            let computed_camera = &sim_world.computed_cameras[sim_world.active_camera as usize];

            let viewport = viewport_size.as_vec2();

            let screen_tl = min.as_vec2();
            let screen_br = max.as_vec2();
            let screen_tr = Vec2::new(screen_br.x, screen_tl.y);
            let screen_bl = Vec2::new(screen_tl.x, screen_br.y);

            let ndc_tl = Self::screen_to_ndc(screen_tl, viewport);
            let ndc_tr = Self::screen_to_ndc(screen_tr, viewport);
            let ndc_br = Self::screen_to_ndc(screen_br, viewport);
            let ndc_bl = Self::screen_to_ndc(screen_bl, viewport);

            let inv = &computed_camera.view_proj.inv;

            let ntl = Self::unproject(ndc_tl.extend(NDC_Z_NEAR), inv);
            let ntr = Self::unproject(ndc_tr.extend(NDC_Z_NEAR), inv);
            let nbr = Self::unproject(ndc_br.extend(NDC_Z_NEAR), inv);
            let nbl = Self::unproject(ndc_bl.extend(NDC_Z_NEAR), inv);

            let ftl = Self::unproject(ndc_tl.extend(NDC_Z_FAR), inv);
            let ftr = Self::unproject(ndc_tr.extend(NDC_Z_FAR), inv);
            let fbr = Self::unproject(ndc_br.extend(NDC_Z_FAR), inv);
            let fbl = Self::unproject(ndc_bl.extend(NDC_Z_FAR), inv);

            Frustum::from_corners(ntl, ntr, nbr, nbl, ftl, ftr, fbr, fbl)
        };
    }

    /// Update the selected objects by using a ray segment with an origin at
    /// the specified pos in screen coordinates.
    fn set_selection_by_ray(&self, sim_world: &mut SimWorld, pos: UVec2, viewport_size: UVec2) {
        sim_world.highlighted_chunks.clear();
        sim_world.highlighted_objects.clear();

        let computed_camera = &sim_world.computed_cameras[sim_world.active_camera as usize];
        let camera_ray_segment = computed_camera.create_ray_segment(pos, viewport_size);

        if let Some(hit) = Self::get_interaction_hit(sim_world, &camera_ray_segment, |_| true) {
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

    #[inline]
    fn screen_to_ndc(p: Vec2, viewport_size: Vec2) -> Vec2 {
        let uv = p / viewport_size;
        let mut ndc = uv * 2.0 - Vec2::ONE;
        // Y grows down, so invert, because NDC grows up.
        ndc.y = -ndc.y;
        ndc
    }

    #[inline]
    fn unproject(ndc: Vec3, inv: &Mat4) -> Vec3 {
        let clip = Vec4::new(ndc.x, ndc.y, ndc.z, 1.0);
        let world = inv * clip;
        world.truncate() / world.w
    }

    fn get_interaction_hit(
        sim_world: &SimWorld,
        camera_ray_segment: &RaySegment,
        object_pred: impl Fn(&Object) -> bool,
    ) -> Option<InteractionHit> {
        if camera_ray_segment.is_degenerate() {
            return None;
        }

        let mut best_object_t = f32::INFINITY;
        let mut best_object_hit: Option<(Handle<Object>, Vec3)> = None;

        let mut object_candidates = Vec::new();
        sim_world
            .objects
            .static_bvh
            .objects_intersect_ray_segment(camera_ray_segment, &mut object_candidates);

        for handle in object_candidates {
            let Some(object) = sim_world.objects.get(handle) else {
                continue;
            };
            if !object_pred(object) {
                continue;
            }

            if let Some(hit) =
                object.ray_intersection(camera_ray_segment, RayIntersectionMode::Meshes)
                && hit.t < best_object_t
            {
                best_object_t = hit.t;
                best_object_hit = Some((handle, hit.world_position));
            }
        }

        let mut best_terrain_t = f32::INFINITY;
        let mut best_terrain_hit: Option<(IVec2, Vec3)> = None;

        let mut chunk_candidates = Vec::new();
        sim_world
            .terrain
            .quad_tree
            .ray_intersect_chunks(camera_ray_segment, &mut chunk_candidates);

        for chunk_coord in chunk_candidates {
            if let Some(hit) = sim_world
                .terrain
                ._chunk_intersect_ray_segment(chunk_coord, camera_ray_segment)
                && hit.t < best_terrain_t
            {
                best_terrain_t = hit.t;
                best_terrain_hit = Some((chunk_coord, hit.world_position));
            }
        }

        match (best_object_hit, best_terrain_hit) {
            (Some((object, world_position)), Some((chunk_coord, terrain_position))) => {
                if best_object_t <= best_terrain_t {
                    Some(InteractionHit::Object {
                        _world_position: world_position,
                        _distance: best_object_t,
                        object,
                    })
                } else {
                    Some(InteractionHit::Terrain {
                        _world_position: terrain_position,
                        _distance: best_terrain_t,
                        chunk_coord,
                    })
                }
            }
            (Some((object, world_position)), None) => Some(InteractionHit::Object {
                _world_position: world_position,
                _distance: best_object_t,
                object,
            }),
            (None, Some((chunk_coord, terrain_position))) => Some(InteractionHit::Terrain {
                _world_position: terrain_position,
                _distance: best_terrain_t,
                chunk_coord,
            }),
            (None, None) => None,
        }
    }
}
