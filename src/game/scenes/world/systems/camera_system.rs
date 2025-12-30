use glam::vec4;
use strum::IntoEnumIterator;

use crate::{
    engine::{input::InputState, renderer::Renderer},
    game::scenes::world::{
        render::RenderWorld,
        sim_world::{ActiveCamera, Camera, DayNightCycle, SimWorld},
        systems::Time,
    },
};

use super::{
    free_camera_controller::FreeCameraController,
    top_down_camera_controller::TopDownCameraController,
};

pub trait CameraController {
    /// Gather input intent by the user.
    fn handle_input(
        &mut self,
        target_camera: &mut Camera,
        input_state: &InputState,
        delta_time: f32,
    );
}

pub struct CameraSystem {
    /// Control the game camera.
    pub game_controller: TopDownCameraController,
    /// Control the debug camera.
    pub debug_controller: FreeCameraController,
}

impl CameraSystem {
    pub fn new(
        game_controller: TopDownCameraController,
        debug_controller: FreeCameraController,
    ) -> Self {
        Self {
            game_controller,
            debug_controller,
        }
    }
}

impl CameraSystem {
    fn extract_camera(sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let state = sim_world.state();

        let source = &state.computed_cameras[state.active_camera as usize];
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
}

impl CameraSystem {
    pub fn input(&mut self, sim_world: &mut SimWorld, time: &Time, input_state: &InputState) {
        let day_night_cycle = sim_world.ecs.resource::<DayNightCycle>();
        let time_of_day = sim_world.state().time_of_day;
        let far = day_night_cycle
            .fog_distance
            .sample_sub_frame(time_of_day, true);

        let state = &mut sim_world.state_mut();
        let c = state.active_camera;
        let camera = &mut state.cameras[c as usize];
        camera.far = far;

        match c {
            ActiveCamera::Game => {
                self.game_controller
                    .handle_input(camera, input_state, time.delta_time);
            }

            ActiveCamera::Debug => {
                self.debug_controller
                    .handle_input(camera, input_state, time.delta_time);
            }
        }
    }

    #[inline]
    pub fn compute_cameras(&mut self, sim_world: &mut SimWorld) {
        let mut state = sim_world.state_mut();
        for c in ActiveCamera::iter() {
            state.computed_cameras[c as usize] = state.cameras[c as usize].compute();
        }
    }

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
