use bevy_ecs::prelude::*;
use glam::Mat4;

use crate::game::scenes::world::{
    render::RenderUiRect,
    sim_world::{
        SimWorldState,
        ecs::{Snapshots, Viewport},
    },
};

pub fn extract_ui_snapshot(
    mut snapshots: ResMut<Snapshots>,
    state: Res<SimWorldState>,
    viewport: Res<Viewport>,
) {
    let snapshot = &mut snapshots.ui_render_snapshot;

    snapshot.view_proj = Mat4::orthographic_rh(
        0.0,
        viewport.size.x as f32,
        viewport.size.y as f32,
        0.0,
        -1.0,
        1.0,
    );

    // Copy the requested [UiRect]s to the temp buffer. Trying to avoid allocations.
    snapshot.ui_rects.clear();
    snapshot
        .ui_rects
        .extend(state.ui.ui_rects.iter().map(|rect| {
            let min = rect.pos.as_vec2();
            let max = min + rect.size.as_vec2();
            RenderUiRect {
                min: [min.x.min(max.x), min.y.min(max.y)],
                max: [min.x.max(max.x), min.y.max(max.y)],
                color: rect.color.to_array(),
            }
        }));
}
