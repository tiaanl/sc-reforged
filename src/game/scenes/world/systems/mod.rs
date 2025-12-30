use bevy_ecs::prelude::*;
use glam::{UVec2, Vec2};

use crate::{
    engine::{
        input::InputState,
        renderer::{Frame, Renderer},
    },
    game::{
        config::Campaign,
        scenes::world::{
            render::{RenderStore, RenderWorld, WorldRenderer},
            sim_world::{Objects, SimWorld},
            systems::{
                free_camera_controller::FreeCameraController,
                top_down_camera_controller::TopDownCameraController,
            },
        },
    },
};

mod camera_system;
mod clear_render_targets;
pub mod cull_system;
pub mod day_night_cycle_system;
mod free_camera_controller;
mod gizmo_system;
pub mod object_system;
mod top_down_camera_controller;
pub mod world_interaction;

pub use world_interaction::InteractionHit;

#[derive(Default, Resource)]
pub struct Time {
    /// Time elapsed since the last frame was rendered.
    pub delta_time: f32,
    /// Time in seconds since the simulation started.
    pub sim_time: f32,
}

/// Shared resources between rendering in the systems and the [RenderWorld].
pub struct Systems {
    /// Cache the sim time to pass to the [CameraEnvironment].
    sim_time: f32,

    pub camera_system: camera_system::CameraSystem,

    pub world_renderer: WorldRenderer,

    gizmo_system: gizmo_system::GizmoSystem,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
        sim_world: &SimWorld,
        campaign: &Campaign,
    ) -> Self {
        Self {
            sim_time: 0.0,

            camera_system: camera_system::CameraSystem::new(
                {
                    let camera_from = campaign.view_initial.from.extend(2500.0);
                    let camera_to = campaign.view_initial.to.extend(0.0);

                    let dir = (camera_to - camera_from).normalize();

                    let flat = Vec2::new(dir.x, dir.y);
                    let yaw = (-dir.x).atan2(dir.y).to_degrees();
                    let pitch = dir.z.atan2(flat.length()).to_degrees();

                    TopDownCameraController::new(
                        camera_from,
                        yaw.to_degrees(),
                        pitch.to_degrees(),
                        4_000.0,
                        100.0,
                    )
                },
                FreeCameraController::new(1000.0, 0.2),
            ),
            world_renderer: WorldRenderer::new(renderer, surface_format, render_store, sim_world),
            gizmo_system: gizmo_system::GizmoSystem::new(renderer, surface_format, render_store),
        }
    }

    pub fn input(
        &mut self,
        sim_world: &mut SimWorld,
        time: &Time,
        input_state: &InputState,
        _viewport_size: UVec2,
    ) {
        {
            let mut res = sim_world.ecs.resource_mut::<InputState>();
            *res = input_state.clone();
        }

        // TODO: Not nice that we have to pass in a `viewport_size` here, but don't know where else
        //       to put it for now.

        {
            // TODO: This should really be part of a system somewhere.
            let mut state = sim_world.state_mut();
            state.ui.ui_rects.clear();
        }

        self.camera_system.input(sim_world, time, input_state);

        // TODO: This should be the first step in the update system, but that
        //       would mean all systems should record input state and then
        //       process it in `update` as well, which is not done right now.
        self.camera_system.compute_cameras(sim_world);
    }

    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        self.sim_time = time.sim_time;

        sim_world.update_schedule.run(&mut sim_world.ecs);
    }

    pub fn extract(
        &mut self,
        renderer: &Renderer,
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        render_world.camera_env.sim_time = self.sim_time;
        self.camera_system.extract(sim_world, render_world);

        // Make sure all models are prepared to be rendered.
        let mut objects = sim_world.ecs.resource_mut::<Objects>();
        objects.prepare_models(renderer, render_store);

        self.world_renderer
            .extract(sim_world, render_store, render_world, viewport_size);

        self.gizmo_system.extract(sim_world, render_world);
    }

    pub fn prepare(
        &mut self,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
        surface_size: UVec2,
    ) {
        // Make sure the geometry buffer is the correct size.
        if surface_size != render_store.geometry_buffer.size {
            render_store
                .geometry_buffer
                .resize(&renderer.device, surface_size);
        }

        self.camera_system.prepare(render_world, renderer);
        self.world_renderer.prepare(renderer, render_world);
        self.gizmo_system.prepare(render_world, renderer);
    }

    pub fn queue(
        &mut self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
    ) {
        clear_render_targets::clear_render_targets(
            render_world,
            frame,
            &render_store.geometry_buffer,
        );
        self.world_renderer.queue(
            render_store,
            render_world,
            frame,
            &render_store.geometry_buffer,
        );

        render_store
            .compositor
            .render(frame, &render_store.geometry_buffer);

        self.gizmo_system.queue(render_world, frame);
    }
}
