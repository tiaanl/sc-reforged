use bevy_ecs::prelude::*;
use glam::vec4;

use crate::{
    engine::renderer::Renderer,
    game::scenes::world::{
        render::RenderWorld,
        sim_world::{Camera, ComputedCamera, DayNightCycle, SimWorld, ecs::ActiveCamera},
    },
};

fn extract_camera(camera: &ComputedCamera, render_world: &mut RenderWorld) {
    let target = &mut render_world.camera_env;

    target.proj_view = camera.view_proj.mat.to_cols_array_2d();
    target.frustum = camera
        .frustum
        .planes
        .map(|plane| plane.normal.extend(plane.distance).to_array());
    target.position = camera.position.extend(1.0).to_array();
    target.forward = camera.forward.extend(0.0).to_array();
}

fn extract_environment(sim_world: &SimWorld, render_world: &mut RenderWorld) {
    let state = sim_world.state();

    let target = &mut render_world.camera_env;

    let time_of_day = state.time_of_day;
    let source = &sim_world.ecs.resource::<DayNightCycle>();

    let sun_dir = source.sun_dir.sample_sub_frame(time_of_day, true);
    let sun_color = source.sun_color.sample_sub_frame(time_of_day, true);

    let ambient_color = vec4(0.3, 0.3, 0.3, 1.0);

    let fog_distance = source.fog_distance.sample_sub_frame(time_of_day, true);
    let fog_near_fraction = source.fog_near_fraction.sample_sub_frame(time_of_day, true);
    let fog_color = source.fog_color.sample_sub_frame(time_of_day, true);

    target.sun_dir = sun_dir.extend(0.0).to_array();
    target.sun_color = sun_color.extend(1.0).to_array();
    target.ambient_color = ambient_color.to_array();
    target.fog_color = fog_color.extend(1.0).to_array();
    target.fog_distance = fog_distance;
    target.fog_near_fraction = fog_near_fraction;
}

pub fn compute_cameras(mut cameras: Query<(&Camera, &mut ComputedCamera)>) {
    for (camera, mut computed_camera) in cameras.iter_mut() {
        *computed_camera = camera.compute();
    }
}

pub fn extract(sim_world: &mut SimWorld, render_world: &mut RenderWorld) {
    for camera in sim_world
        .ecs
        .query_filtered::<&ComputedCamera, With<ActiveCamera>>()
        .query(&sim_world.ecs)
    {
        extract_camera(camera, render_world);
    }

    extract_environment(sim_world, render_world);
}

pub fn prepare(render_world: &RenderWorld, renderer: &Renderer) {
    renderer.queue.write_buffer(
        &render_world.camera_env_buffer,
        0,
        bytemuck::bytes_of(&render_world.camera_env),
    );
}
