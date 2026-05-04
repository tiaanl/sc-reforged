use bevy_ecs::prelude::*;
use glam::Mat4;

use crate::game::{
    render::world::WorldRenderSnapshot,
    scenes::world::sim_world::{SimWorldState, ecs::Viewport},
};

pub fn extract_ui_snapshot(
    mut snapshot: ResMut<WorldRenderSnapshot>,
    _state: Res<SimWorldState>,
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
}
