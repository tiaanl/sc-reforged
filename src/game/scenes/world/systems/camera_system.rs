use glam::{Quat, Vec3};

use crate::{
    engine::input::InputState,
    game::{
        camera::Camera,
        scenes::world::{
            render_world::RenderWorld,
            sim_world::{ComputedCamera, SimWorld},
            systems::{System, Time},
        },
    },
};

#[allow(unused_variables)]
pub trait CameraController {
    /// Gather input intent by the user.
    fn handle_input(&mut self, input_state: &InputState) {}

    /// Update the target camera with the gathered user intent.
    fn update(&mut self, camera: &mut Camera, delta_time: f32) {}
}

pub struct CameraSystem<C>
where
    C: CameraController,
{
    /// Index of the camera to control.
    camera_index: usize,
    /// World position of the camera.
    pub position: Vec3,
    /// Rotation of the camera.
    pub rotation: Quat,
    /// The controller used to manipulate the camera.
    pub controller: C,
}

impl<C> CameraSystem<C>
where
    C: CameraController,
{
    pub fn new(camera_index: usize, controller: C) -> Self {
        Self {
            camera_index,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            controller,
        }
    }
}

impl<C> System for CameraSystem<C>
where
    C: CameraController,
{
    fn pre_update(&mut self, _sim_world: &SimWorld, input_state: &InputState) {
        self.controller.handle_input(input_state);
    }

    fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        let source = &mut sim_world.cameras[self.camera_index];

        self.controller.update(source, time.delta_time);

        let view_proj = source.calculate_view_projection();
        let frustum = view_proj.frustum();
        let position = source.position;
        let forward = (source.rotation * Camera::FORWARD).normalize();

        sim_world.computed_cameras[self.camera_index] = ComputedCamera {
            view_proj,
            frustum,
            position,
            forward,
        };
    }

    fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let source = &sim_world.computed_cameras[self.camera_index];

        let target = &mut render_world.cameras[self.camera_index];
        target.proj_view = source.view_proj.mat.to_cols_array_2d();
        target.frustum = source
            .frustum
            .planes
            .map(|plane| plane.normal.extend(plane.distance).to_array());
        target.position = source.position.extend(1.0).to_array();
        target.forward = source.forward.extend(0.0).to_array();
    }

    fn prepare(
        &mut self,
        render_world: &mut RenderWorld,
        renderer: &crate::engine::prelude::Renderer,
    ) {
        let offset = (std::mem::size_of::<Camera>() * self.camera_index) as wgpu::BufferAddress;
        let data = bytemuck::bytes_of(&render_world.cameras[self.camera_index]);
        renderer
            .queue
            .write_buffer(&render_world.camera_buffer, offset, data);
    }
}
