use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::{ComputedCamera, ecs::ActiveCamera};

use super::RenderSnapshot;

pub fn extract_camera(
    mut snapshot: ResMut<RenderSnapshot>,
    camera: Single<&ComputedCamera, With<ActiveCamera>>,
) {
    snapshot.camera = super::render_snapshot::Camera {
        position: camera.position,
        forward: camera.forward,
        _near: camera.near,
        far: camera.far,
        proj_view: camera.view_proj.mat,
        frustum: camera.frustum.clone(),
    }
}
