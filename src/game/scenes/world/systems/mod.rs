use glam::{UVec2, Vec2};

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
pub use objects_system::RenderWrapper;

mod camera_system;
mod clear_render_targets;
mod cull_system;
mod day_night_cycle_system;
mod free_camera_controller;
mod gizmo_system;
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
            }),
            culling: cull_system::CullSystem::default(),
            terrain_system: terrain_system::TerrainSystem::new(renderer, render_store, sim_world),
            objects_system: objects_system::ObjectsSystem::new(renderer, render_store),
            gizmo_system: gizmo_system::GizmoSystem::new(renderer, render_store),
        }
    }

    pub fn input(
        &mut self,
        sim_world: &mut SimWorld,
        time: &Time,
        input_state: &InputState,
        viewport_size: UVec2,
    ) {
        // TODO: Not nice that we have to pass in a `viewport_size` here, but don't know where else
        //       to put it for now.

        self.camera_system.input(sim_world, time, input_state);

        if let Some(mouse_position) = input_state.mouse_position() {
            let _camera_ray_segment = sim_world
                .computed_camera
                .create_ray_segment(mouse_position.as_uvec2(), viewport_size);
        }
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
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
    ) {
        // Make sure the geometry buffer is the correct size.
        if renderer.surface.size() != render_store.geometry_buffer.size {
            render_store
                .geometry_buffer
                .resize(&renderer.device, renderer.surface.size());
        }

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
    ) {
        clear_render_targets::clear_render_targets(
            render_world,
            frame,
            &render_store.geometry_buffer,
        );
        self.terrain_system
            .queue(render_world, frame, &render_store.geometry_buffer);
        self.objects_system.queue(
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
