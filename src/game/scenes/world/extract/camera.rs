use bevy_ecs::prelude::*;

use crate::game::{
    render::world::{Camera, WorldRenderSnapshot},
    scenes::world::sim_world::{ComputedCamera, ecs::ActiveCamera},
};

pub fn extract_camera(
    mut snapshot: ResMut<WorldRenderSnapshot>,
    camera: Single<&ComputedCamera, With<ActiveCamera>>,
) {
    snapshot.camera = Camera {
        position: camera.position,
        forward: camera.forward,
        _near: camera.near,
        far: camera.far,
        proj_view: camera.view_proj.mat,
        frustum: camera.frustum.clone(),
    }
}
