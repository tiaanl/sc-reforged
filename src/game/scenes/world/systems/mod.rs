use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{
    engine::{
        renderer::{Frame, Renderer},
        shader_cache::ShaderCache,
        transform::Transform,
    },
    game::{
        AssetReader,
        math::BoundingBox,
        scenes::world::{
            extract,
            render::{RenderBindings, RenderLayouts, RenderPipeline, RenderTargets, WorldRenderer},
            sim_world::{
                DynamicBvh, DynamicBvhHandle, StaticBvh, StaticBvhHandle, ecs,
                free_camera_controller, top_down_camera_controller,
            },
        },
    },
};

use super::extract::*;

mod camera;
mod changed;
mod clear_render_targets;
mod debug;
mod gizmos;
mod orders;
mod sequences;
pub mod world_interaction;

#[derive(Resource)]
pub struct Time {
    /// The instant the scene started running. Constant the entire time the
    /// scene is running.
    pub scene_start: std::time::Instant,
    /// Time elapsed since the last frame was rendered.
    pub delta_time: f32,
    /// Time in seconds since the simulation started.
    pub sim_time: f32,
    /// The number of the current frame. This will loop round to 0 when it runs
    /// out of numbers.
    pub frame_index: u64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            scene_start: std::time::Instant::now(),
            delta_time: 0.0,
            sim_time: 0.0,
            frame_index: 0,
        }
    }
}

impl Time {
    pub fn next_frame(&mut self, delta_time: f32) {
        self.delta_time = delta_time;
        self.sim_time = (std::time::Instant::now() - self.scene_start).as_secs_f32();
        self.frame_index = self.frame_index.wrapping_add(1);
    }
}

/// Shared resources between rendering in the systems and the [RenderWorld].
pub struct Systems {
    /// Cache the sim time to pass to the [CameraEnvironment].
    sim_time: f32,

    pub update_schedule: Schedule,
    pub extract_schedule: Schedule,

    // pub camera_system: camera_system::CameraSystem,
    pub world_renderer: WorldRenderer,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        render_targets: &RenderTargets,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        sim_world: &mut World,
    ) -> Self {
        // Update schedule.
        let update_schedule = {
            use ecs::UpdateSet::*;

            let mut schedule = Schedule::default();

            schedule.configure_sets((Start, Input, Update).chain());

            // Start
            schedule.add_systems(gizmos::clear_gizmo_vertices.in_set(Start));

            // Input
            schedule.add_systems(
                (
                    (
                        top_down_camera_controller::input,
                        free_camera_controller::input,
                    ),
                    camera::update_far_distance.run_if(changed::time_of_day_changed),
                    camera::compute_cameras,
                    world_interaction::input,
                )
                    .in_set(Input)
                    .chain(),
            );

            // Update
            schedule.add_systems(
                (
                    orders::issue_new_orders,
                    orders::process_biped_orders,
                    world_interaction::update,
                    rebuild_static_bvh.run_if(|q: Query<(), Added<ecs::BoundingBoxComponent>>| {
                        q.iter().count() > 0
                    }),
                    update_dynamic_bvh,
                    sequences::enqueue_next_sequences,
                    sequences::update_sequencers,
                    // debug::draw_model_bounding_boxes,
                )
                    .in_set(Update)
                    .chain(),
            );

            schedule
        };

        let extract_schedule = extract::create_extract_schedule();

        let assets = sim_world.resource::<AssetReader>();

        Self {
            sim_time: 0.0,

            update_schedule,
            extract_schedule,

            world_renderer: WorldRenderer::new(
                assets,
                renderer,
                render_targets,
                layouts,
                shader_cache,
                sim_world,
            ),
        }
    }

    pub fn update(&mut self, sim_world: &mut World) {
        // TODO: What does self.sim_time actually do?
        self.sim_time = sim_world
            .resource::<Time>()
            .scene_start
            .elapsed()
            .as_secs_f32();

        self.update_schedule.run(sim_world);
    }

    pub fn extract(&mut self, sim_world: &mut World) {
        self.extract_schedule.run(sim_world);
    }

    pub fn prepare(
        &mut self,
        assets: &AssetReader,
        bindings: &mut RenderBindings,
        renderer: &Renderer,
        render_snapshot: &RenderSnapshot,
    ) {
        self.world_renderer
            .prepare(assets, renderer, bindings, render_snapshot);
    }

    pub fn queue(
        &mut self,
        render_targets: &RenderTargets,
        bindings: &RenderBindings,
        snapshot: &RenderSnapshot,
        frame: &mut Frame,
    ) {
        clear_render_targets::clear_render_targets(
            frame,
            &render_targets.geometry_buffer,
            snapshot.environment.fog_color,
        );
        self.world_renderer
            .queue(bindings, frame, &render_targets.geometry_buffer, snapshot);
    }
}

fn rebuild_static_bvh(
    objects: Query<(Entity, &Transform, &ecs::BoundingBoxComponent), With<StaticBvhHandle>>,
    mut static_bvh: ResMut<StaticBvh>,
    mut bounding_box_scratch: Local<Vec<(Entity, BoundingBox)>>,
) {
    bounding_box_scratch.clear();

    objects
        .iter()
        .for_each(|(entity, transform, bounding_box)| {
            let bounding_box = bounding_box.0.transformed(transform.to_mat4());
            bounding_box_scratch.push((entity, bounding_box))
        });

    if bounding_box_scratch.is_empty() {
        tracing::warn!("Empty static bvh bounding boxes!");
    } else {
        tracing::info!(
            "Rebuilding static BVH with {} objects",
            bounding_box_scratch.len()
        );
        static_bvh.rebuild(&bounding_box_scratch)
    }
}

fn update_dynamic_bvh(
    objects: Query<(&DynamicBvhHandle, &Transform, &ecs::BoundingBoxComponent), Changed<Transform>>,
    mut dynamic_bvh: ResMut<DynamicBvh>,
) {
    let bvh = dynamic_bvh.as_mut();

    for (&handle, transform, bounding_box) in objects.iter() {
        let new_bounding_box = bounding_box.0.transformed(transform.to_mat4());
        bvh.update(handle, new_bounding_box, Vec3::ZERO);
    }
}
