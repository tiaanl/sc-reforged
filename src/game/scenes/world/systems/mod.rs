use bevy_ecs::prelude::*;
use glam::UVec2;

use crate::{
    engine::{
        renderer::{Frame, Renderer},
        transform::Transform,
    },
    game::scenes::world::{
        render::{
            GizmoRenderPipeline, GizmoRenderSnapshot, RenderBox, RenderStore, RenderWorld,
            WorldRenderer,
        },
        sim_world::{
            SimWorld,
            ecs::{self, BoundingBoxComponent},
            free_camera_controller, top_down_camera_controller,
        },
    },
};

use super::extract::*;

pub mod camera_system;
mod clear_render_targets;
pub mod day_night_cycle_system;
mod orders;
pub mod world_interaction;

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

    pub update_schedule: Schedule,
    pub extract_schedule: Schedule,

    terrain_extract: TerrainExtract,
    model_extract: ModelsExtract,
    gizmo_extract: GizmoExtract,

    // pub camera_system: camera_system::CameraSystem,
    pub world_renderer: WorldRenderer,

    pub gizmo_render_snapshot: GizmoRenderSnapshot,
    gizmo_render_pipeline: GizmoRenderPipeline,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
        sim_world: &mut SimWorld,
    ) -> Self {
        // Update schedule.
        let update_schedule = {
            use ecs::UpdateSet::*;

            let mut schedule = Schedule::default();

            schedule.configure_sets((Input, Update).chain());

            schedule.add_systems(
                (
                    (
                        top_down_camera_controller::input,
                        free_camera_controller::input,
                    ),
                    camera_system::compute_cameras,
                    world_interaction::input,
                )
                    .in_set(Input)
                    .chain(),
            );

            schedule.add_systems(
                (
                    day_night_cycle_system::increment_time_of_day,
                    orders::process_biped_orders,
                    world_interaction::update,
                )
                    .in_set(Update)
                    .chain(),
            );

            schedule
        };

        let extract_schedule = {
            let mut schedule = Schedule::default();

            fn clear_snapshots(mut snapshots: ResMut<ecs::Snapshots>) {
                snapshots.clear();
            }

            schedule.add_systems(
                (
                    clear_snapshots,
                    |bounding_boxes: Query<(&Transform, &BoundingBoxComponent)>,
                     mut snapshots: ResMut<ecs::Snapshots>| {
                        for (transform, bounding_box_component) in bounding_boxes.iter() {
                            snapshots.box_render_snapshot.boxes.push(RenderBox {
                                transform: transform.clone(),
                                min: bounding_box_component.0.min,
                                max: bounding_box_component.0.max,
                            });
                        }
                    },
                )
                    .chain(),
            );

            schedule
        };

        Self {
            sim_time: 0.0,

            update_schedule,
            extract_schedule,

            terrain_extract: TerrainExtract::new(sim_world),
            model_extract: ModelsExtract::new(sim_world),
            gizmo_extract: GizmoExtract::new(sim_world),

            world_renderer: WorldRenderer::new(renderer, surface_format, render_store, sim_world),

            gizmo_render_snapshot: GizmoRenderSnapshot::default(),
            gizmo_render_pipeline: GizmoRenderPipeline::new(renderer, surface_format, render_store),
        }
    }

    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        self.sim_time = time.sim_time;

        self.update_schedule.run(&mut sim_world.ecs);
    }

    pub fn extract(
        &mut self,
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        self.extract_schedule.run(&mut sim_world.ecs);

        self.terrain_extract
            .extract(sim_world, &mut self.world_renderer.terrain_render_snapshot);

        self.model_extract
            .extract(sim_world, &mut self.world_renderer.model_render_snapshot);

        self.gizmo_extract
            .extract(sim_world, &mut self.gizmo_render_snapshot);

        render_world.camera_env.sim_time = self.sim_time;

        camera_system::extract(sim_world, render_world);

        self.world_renderer
            .extract(sim_world, render_store, render_world, viewport_size);
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

        camera_system::prepare(render_world, renderer);
        self.world_renderer
            .prepare(renderer, render_store, render_world);
        self.gizmo_render_pipeline
            .prepare(render_world, renderer, &self.gizmo_render_snapshot);
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

        self.gizmo_render_pipeline
            .queue(render_world, &self.gizmo_render_snapshot, frame);
    }
}
