use bevy_ecs::prelude::*;
use glam::{Mat4, Vec2};

use crate::game::scenes::world::{
    extract::{RenderSnapshot, UiRect},
    sim_world::{SimWorldState, ecs::Viewport},
};

pub fn extract_ui_snapshot(
    mut snapshot: ResMut<RenderSnapshot>,
    state: Res<SimWorldState>,
    viewport: Res<Viewport>,
) {
    let snapshot = &mut snapshot.ui;

    snapshot.proj_view = Mat4::orthographic_rh(
        0.0,
        viewport.size.x as f32,
        viewport.size.y as f32,
        0.0,
        -1.0,
        1.0,
    );

    snapshot.ui_rects.clear();
    snapshot
        .ui_rects
        .extend(state.ui.ui_rects.iter().map(|rect| {
            let min = rect.pos.as_vec2();
            let max = min + rect.size.as_vec2();
            UiRect {
                min: Vec2::new(min.x.min(max.x), min.y.min(max.y)),
                max: Vec2::new(min.x.max(max.x), min.y.max(max.y)),
                color: rect.color,
            }
        }));
}
