use bevy_ecs::prelude::*;
use glam::{IVec2, UVec2, Vec4};

use crate::{
    engine::input::InputState,
    game::scenes::world::sim_world::{
        ComputedCamera, DynamicBvh, SimWorldState, Terrain, UiRect,
        ecs::{ActiveCamera, Viewport},
    },
};

#[derive(Event)]
pub struct Clicked {
    pos: UVec2,
}

#[derive(Clone, Copy, Event)]
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
    pub fn _min_max(&self) -> (UVec2, UVec2) {
        let n = self.normalize();
        (n.pos, n.pos + n.size.as_uvec2())
    }
}

#[derive(Default, Resource)]
pub struct WorldInteraction {
    selection_rect: Option<SelectionRect>,
    pub selected_entity: Option<Entity>,
}

const DRAG_THRESHOLD: u32 = 2;

pub fn input(
    mut commands: Commands,
    mut world_interaction: ResMut<WorldInteraction>,
    input_state: Res<InputState>,
) {
    use winit::event::MouseButton;

    if input_state.mouse_just_pressed(MouseButton::Left) {
        // Start dragging the selection rect.
        world_interaction.selection_rect = input_state.mouse_position().map(|pos| SelectionRect {
            pos,
            size: IVec2::ZERO,
        });
    } else if input_state.mouse_just_released(MouseButton::Left) {
        // If the selection rect is larger than the threshold, that confirms that we drew a selection rect and not just clicked.
        if let Some(rect) = world_interaction.selection_rect.take() {
            if rect.size.abs().min_element() > DRAG_THRESHOLD as i32 {
                commands.trigger(rect);
            } else {
                commands.trigger(Clicked { pos: rect.pos });
            }
        }
    } else if let Some(ref mut selection_rect) = world_interaction.selection_rect
        && let Some(mouse_position) = input_state.mouse_position()
    {
        selection_rect.size = mouse_position.as_ivec2() - selection_rect.pos.as_ivec2();
    }
}

pub fn update(world_interaction: Res<WorldInteraction>, mut state: ResMut<SimWorldState>) {
    if let Some(rect) = &world_interaction.selection_rect {
        let SelectionRect { pos, size } = rect.normalize();
        let size = size.as_uvec2();

        if size.x > DRAG_THRESHOLD && size.y > DRAG_THRESHOLD {
            const THICKNESS: u32 = 1;

            state.ui.ui_rects.push(UiRect {
                pos,
                size,
                color: Vec4::new(0.0, 0.0, 0.0, 0.5),
            });

            // Left
            state.ui.ui_rects.push(UiRect {
                pos,
                size: UVec2::new(THICKNESS, size.y),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5),
            });

            // Right
            state.ui.ui_rects.push(UiRect {
                pos: UVec2::new(pos.x + size.x - THICKNESS, pos.y),
                size: UVec2::new(THICKNESS, size.y),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5),
            });

            // Top
            state.ui.ui_rects.push(UiRect {
                pos: UVec2::new(pos.x + THICKNESS, pos.y),
                size: UVec2::new(size.x.max(THICKNESS * 2) - THICKNESS * 2, THICKNESS),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5),
            });

            // Bottom
            state.ui.ui_rects.push(UiRect {
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

#[allow(clippy::too_many_arguments)]
pub fn on_clicked(
    clicked: On<Clicked>,
    viewport: Res<Viewport>,
    camera: Single<&ComputedCamera, With<ActiveCamera>>,
    dynamic_bvh: Res<DynamicBvh>,
    terrain: Res<Terrain>,
    mut world_interaction: ResMut<WorldInteraction>,

    mut entity_cache: Local<Vec<Entity>>,
    mut terrain_hit_cache: Local<Vec<IVec2>>,
) {
    let ray = camera.create_ray_segment(clicked.pos, viewport.size);

    entity_cache.clear();
    dynamic_bvh._query_ray_segment(&ray, &mut entity_cache);

    if let Some(&entity) = entity_cache.first() {
        // TODO: Check that the entity is actually selectable.

        // The user click on a selectable entity, so select it.
        world_interaction.selected_entity = Some(entity);
        return;
    }

    /*
    // If nothing is selected, we're done for now.
    let Some(selected_entity) = world_interaction.selected_entity else {
        return;
    };

    // If the user didn't click on a selectable entity, check for a terrain hit.
    terrain_hit_cache.clear();
    terrain
        .quad_tree
        .ray_intersect_chunks(&ray, &mut terrain_hit_cache);

    let terrain_hits: Vec<_> = terrain_hit_cache
        .iter()
        .filter_map(|&chunk| terrain.chunk_intersect_ray_segment(chunk, &ray))
        .collect();

    if let Some(terrain_hit) = terrain_hits.iter().min_by(|&a, &b| a.t.total_cmp(&b.t)) {}
    */
}

/*
/// Update the selected objects by using a rectangle in screen coordinates.
fn set_selection_by_rect(
    rect: SelectionRect,
    computed_camera: &ComputedCamera,
    viewport_size: UVec2,
) {
    let (min, max) = rect.min_max();
    debug_assert!(min.x <= max.x);
    debug_assert!(min.y <= max.y);

    let _frustum = {
        const NDC_Z_NEAR: f32 = 0.0;
        const NDC_Z_FAR: f32 = 1.0;

        let viewport = viewport_size.as_vec2();

        let screen_tl = min.as_vec2();
        let screen_br = max.as_vec2();
        let screen_tr = Vec2::new(screen_br.x, screen_tl.y);
        let screen_bl = Vec2::new(screen_tl.x, screen_br.y);

        let ndc_tl = screen_to_ndc(screen_tl, viewport);
        let ndc_tr = screen_to_ndc(screen_tr, viewport);
        let ndc_br = screen_to_ndc(screen_br, viewport);
        let ndc_bl = screen_to_ndc(screen_bl, viewport);

        let inv = &computed_camera.view_proj.inv;

        let ntl = unproject(ndc_tl.extend(NDC_Z_NEAR), inv);
        let ntr = unproject(ndc_tr.extend(NDC_Z_NEAR), inv);
        let nbr = unproject(ndc_br.extend(NDC_Z_NEAR), inv);
        let nbl = unproject(ndc_bl.extend(NDC_Z_NEAR), inv);

        let ftl = unproject(ndc_tl.extend(NDC_Z_FAR), inv);
        let ftr = unproject(ndc_tr.extend(NDC_Z_FAR), inv);
        let fbr = unproject(ndc_br.extend(NDC_Z_FAR), inv);
        let fbl = unproject(ndc_bl.extend(NDC_Z_FAR), inv);

        Frustum::from_corners(ntl, ntr, nbr, nbl, ftl, ftr, fbr, fbl)
    };
}

/// The user clicked somewhere and wants to interact with an object.
fn interact_with(
    _state: &mut SimWorldState,
    _computed_camera: &ComputedCamera,
    _terrain: &Terrain,
    _pos: UVec2,
    _viewport_size: UVec2,
) {
    /*
    // Shoot a ray into the scene from the camera to see what we can hit.
    let camera_ray_segment = computed_camera.create_ray_segment(pos, viewport_size);

    if let Some(hit) = get_interaction_hit(objects, terrain, &camera_ray_segment) {
        if state.selected_objects.is_empty() {
            if let InteractionHit::Object {
                _world_position,
                _distance,
                object,
            } = hit
            {
                // Select the object that was clicked on.
                state.selected_objects.insert(object);
            }
        } else {
            // Interact with all selected objects.
            for &handle in state.selected_objects.iter() {
                if let Some(object) = objects.get_mut(handle) {
                    object.interact(&hit)
                }
            }
        }
    }
    */
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

fn _get_interaction_hit(
    _terrain: &Terrain,
    _camera_ray_segment: &RaySegment,
) -> Option<_InteractionHit> {
    None

    /*
    if camera_ray_segment.is_degenerate() {
        return None;
    }

    let mut best_object_t = f32::INFINITY;
    let mut best_object_hit: Option<(Handle<Object>, Vec3)> = None;

    let mut object_candidates = Vec::new();

    objects
        .static_bvh
        .objects_intersect_ray_segment(camera_ray_segment, &mut object_candidates);

    for (handle, object) in object_candidates
        .iter()
        .filter_map(|&handle| objects.get(handle).map(|object| (handle, object)))
    {
        if let Some(hit) = object.ray_intersection(camera_ray_segment, RayIntersectionMode::Meshes)
            && hit.t < best_object_t
        {
            best_object_t = hit.t;
            best_object_hit = Some((handle, hit.world_position));
        }
    }

    let mut best_terrain_t = f32::INFINITY;
    let mut best_terrain_hit: Option<(IVec2, Vec3)> = None;

    let mut chunk_candidates = Vec::new();
    terrain
        .quad_tree
        .ray_intersect_chunks(camera_ray_segment, &mut chunk_candidates);

    for chunk_coord in chunk_candidates {
        if let Some(hit) = terrain._chunk_intersect_ray_segment(chunk_coord, camera_ray_segment)
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
                    world_position: terrain_position,
                    _distance: best_terrain_t,
                    _chunk_coord: chunk_coord,
                })
            }
        }
        (Some((object, world_position)), None) => Some(InteractionHit::Object {
            _world_position: world_position,
            _distance: best_object_t,
            object,
        }),
        (None, Some((chunk_coord, terrain_position))) => Some(InteractionHit::Terrain {
            world_position: terrain_position,
            _distance: best_terrain_t,
            _chunk_coord: chunk_coord,
        }),
        (None, None) => None,
    }
    */
}
*/
