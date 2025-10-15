use glam::vec2;

use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::{
        config::Campaign,
        scenes::world::{
            render::{RenderStore, RenderWorld},
            sim_world::SimWorld,
            systems::top_down_camera_controller::TopDownCameraController,
        },
    },
};

pub use cull_system::DebugQuadTreeOptions;

mod camera_system;
mod clear_render_targets;
mod cull_system;
mod day_night_cycle_system;
mod free_camera_controller;
mod gizmo_system;
mod model_render_system;
mod objects_system;
mod terrain_system;
mod top_down_camera_controller;

pub struct Time {
    pub delta_time: f32,
}

/// Shared resources between rendering in the systems and the [RenderWorld].
pub struct Systems {
    camera_system: camera_system::CameraSystem<TopDownCameraController>,
    pub culling: cull_system::CullSystem,
    terrain_system: terrain_system::TerrainSystem,
    pub objects_system: objects_system::ObjectsSystem,
    gizmo_system: gizmo_system::GizmoSystem,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        render_store: &RenderStore,
        sim_world: &SimWorld,
        campaign: &Campaign,
    ) -> Self {
        Self {
            camera_system: camera_system::CameraSystem::new({
                let camera_from = campaign.view_initial.from.extend(2500.0);
                let camera_to = campaign.view_initial.to.extend(0.0);

                let dir = (camera_to - camera_from).normalize();

                let flat = vec2(dir.x, dir.y);
                let yaw = (-dir.x).atan2(dir.y).to_degrees();
                let pitch = dir.z.atan2(flat.length()).to_degrees();

                TopDownCameraController::new(
                    camera_from,
                    yaw.to_degrees(),
                    pitch.to_degrees(),
                    10_000.0,
                    100.0,
                )
            }),
            culling: cull_system::CullSystem::default(),
            terrain_system: terrain_system::TerrainSystem::new(renderer, render_store, sim_world),
            objects_system: objects_system::ObjectsSystem::new(renderer, render_store),
            gizmo_system: gizmo_system::GizmoSystem::new(renderer, render_store),
        }
    }

    pub fn input(&mut self, sim_world: &mut SimWorld, time: &Time, input_state: &InputState) {
        self.camera_system.input(sim_world, time, input_state);
    }

    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        self.culling.calculate_visible_chunks(sim_world);
        day_night_cycle_system::increment_time_of_day(sim_world, time);
        self.objects_system.render_gizmos(sim_world);
    }

    pub fn extract(
        &mut self,
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
    ) {
        self.camera_system.extract(sim_world, render_world);
        self.terrain_system.extract(sim_world, render_world);
        self.gizmo_system.extract(sim_world, render_world);

        // Make sure all models are prepared to be rendered.
        sim_world.objects.prepare_models(render_store);

        self.objects_system.extract(sim_world, render_store);
    }

    pub fn prepare(
        &mut self,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
        _render_store: &mut RenderStore,
    ) {
        self.camera_system.prepare(render_world, renderer);
        self.terrain_system.prepare(render_world, renderer);
        self.objects_system.prepare(render_world, renderer);
        self.gizmo_system.prepare(render_world, renderer);
    }

    pub fn queue(
        &mut self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        depth_buffer: &wgpu::TextureView,
    ) {
        clear_render_targets::clear_render_targets(render_world, frame, depth_buffer);
        self.terrain_system.queue(render_world, frame, depth_buffer);
        self.objects_system
            .queue(render_store, render_world, frame, depth_buffer);
        self.gizmo_system.queue(render_world, frame);
    }
}
