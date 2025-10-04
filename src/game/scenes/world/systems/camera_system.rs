use crate::{
    engine::input::InputState,
    game::{
        camera::Camera,
        scenes::world::{
            render_world::RenderWorld,
            sim_world::{ComputedCamera, SimWorld},
            systems::{ExtractContext, PreUpdateContext, PrepareContext, System},
        },
    },
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

pub struct CameraSystem<C>
where
    C: CameraController,
{
    /// The controller used to manipulate the camera.
    pub controller: C,
}

impl<C> CameraSystem<C>
where
    C: CameraController,
{
    pub fn new(controller: C) -> Self {
        Self { controller }
    }
}

impl<C> CameraSystem<C>
where
    C: CameraController,
{
    fn extract_camera(sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let source = &sim_world.computed_camera;
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

        let fog_distance = source.fog_distance.sample_sub_frame(time_of_day, true);
        let fog_near_fraction = source.fog_near_fraction.sample_sub_frame(time_of_day, true);
        let fog_color = source.fog_color.sample_sub_frame(time_of_day, true);

        target.sun_dir = sun_dir.extend(0.0).to_array();
        target.sun_color = sun_color.extend(1.0).to_array();
        target.fog_color = fog_color.extend(1.0).to_array();
        target.fog_distance = fog_distance;
        target.fog_near_fraction = fog_near_fraction;
    }
}

impl<C> System for CameraSystem<C>
where
    C: CameraController,
{
    fn pre_update(&mut self, context: &mut PreUpdateContext) {
        let camera = &mut context.sim_world.camera;
        self.controller
            .handle_input(camera, &context.input_state, context.time.delta_time);

        let view_proj = camera.calculate_view_projection();
        let frustum = view_proj.frustum();
        let position = camera.position;
        let forward = (camera.rotation * Camera::FORWARD).normalize();

        context.sim_world.computed_camera = ComputedCamera {
            view_proj,
            frustum,
            position,
            forward,
        };
    }

    fn extract(&mut self, context: &mut ExtractContext) {
        Self::extract_camera(context.sim_world, context.render_world);
        Self::extract_environment(context.sim_world, context.render_world);
    }

    fn prepare(&mut self, context: &mut PrepareContext) {
        context.renderer.queue.write_buffer(
            &context.render_world.camera_env_buffer,
            0,
            bytemuck::bytes_of(&context.render_world.camera_env),
        );
    }
}
