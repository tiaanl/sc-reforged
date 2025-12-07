use bevy_ecs::{
    query::{Changed, With},
    system::{Query, Res},
};
use glam::vec4;

use crate::{
    engine::prelude::Renderer,
    game::scenes::world::{
        render::RenderWorld,
        sim_world::{
            ActiveCamera, Camera, CameraController, ComputedCamera, InputStateResource, SimWorld,
            Time,
        },
    },
};

pub struct CameraSystem;

impl CameraSystem {
    fn extract_camera(sim_world: &mut SimWorld, render_world: &mut RenderWorld) {
        let source = {
            let mut query = sim_world
                .world
                .query_filtered::<&ComputedCamera, With<ActiveCamera>>();
            query.single(&sim_world.world).unwrap()
        };

        let target = &mut render_world.camera_env;

        target.proj_view = source.view_proj.mat.to_cols_array_2d();
        target.frustum = source
            .frustum
            .planes
            .map(|plane| plane.normal.extend(plane.distance).to_array());
        target.position = source.position.extend(1.0).to_array();
        target.forward = source.forward.extend(0.0).to_array();
    }

    fn extract_environment(sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let target = &mut render_world.camera_env;

        let time_of_day = sim_world.time_of_day;
        let source = &sim_world.day_night_cycle;

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
}

impl CameraSystem {
    pub fn extract(&mut self, sim_world: &mut SimWorld, render_world: &mut RenderWorld) {
        Self::extract_camera(sim_world, render_world);
        Self::extract_environment(sim_world, render_world);
    }

    pub fn prepare(&mut self, render_world: &RenderWorld, renderer: &Renderer) {
        renderer.queue.write_buffer(
            &render_world.camera_env_buffer,
            0,
            bytemuck::bytes_of(&render_world.camera_env),
        );
    }
}

pub fn camera_controller_input(
    mut query: Query<(&mut CameraController, &mut Camera), With<ActiveCamera>>,
    input_state: Res<InputStateResource>,
    time: Res<Time>,
) {
    for (mut controller, mut camera) in query.iter_mut() {
        match controller.as_mut() {
            CameraController::TopDown(controller) => {
                controller.handle_input(camera.as_mut(), &input_state.0, time.delta_time)
            }

            CameraController::Free(controller) => {
                controller.handle_input(camera.as_mut(), &input_state.0, time.delta_time)
            }
        }
    }
}

pub fn compute_cameras(mut query: Query<(&Camera, &mut ComputedCamera), Changed<Camera>>) {
    for (camera, mut computed_camera) in query.iter_mut() {
        *computed_camera = camera.compute();
    }
}
