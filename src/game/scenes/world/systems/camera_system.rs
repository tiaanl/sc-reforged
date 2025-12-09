use bevy_ecs::{
    query::{Changed, With},
    system::{Query, Res},
};
use glam::vec4;

use crate::{
    engine::prelude::Renderer,
    game::scenes::world::{
        plugins::day_night_cycle::DayNightCycle,
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

        let day_night_cycle = sim_world.world.resource::<DayNightCycle>();
        let snapshot = day_night_cycle.snapshot();

        let ambient_color = vec4(0.3, 0.3, 0.3, 1.0);

        target.sun_dir = snapshot.sun_dir.extend(0.0).to_array();
        target.sun_color = snapshot.sun_color.extend(1.0).to_array();
        target.ambient_color = ambient_color.to_array();
        target.fog_color = snapshot.fog_color.extend(1.0).to_array();
        target.fog_distance = snapshot.fog_distance;
        target.fog_near_fraction = snapshot.fog_near_fraction;
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
