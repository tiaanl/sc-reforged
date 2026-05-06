use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{
    engine::{input::InputState, transform::Transform},
    game::{
        math::BoundingBox,
        sim::{
            DynamicBvh, DynamicBvhHandle, StaticBvh, StaticBvhHandle, ecs, extract,
            free_camera_controller, top_down_camera_controller,
        },
    },
};

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

#[derive(Resource)]
pub struct SimulationControl {
    /// Whether systems in [ecs::UpdateSet::Update] should run this frame.
    pub run_update: bool,
}

impl Default for SimulationControl {
    fn default() -> Self {
        Self { run_update: true }
    }
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

/// Build the per-frame simulation update [Schedule]. Run once per tick against
/// the simulation [World]; relies on `Time` having been advanced beforehand.
pub fn build_update_schedule() -> Schedule {
    use ecs::UpdateSet::*;

    let mut schedule = Schedule::default();

    schedule.configure_sets((Start, Input, Update, End).chain());
    schedule.configure_sets(Update.run_if(should_run_simulation_update));

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
            world_interaction::update,
            (
                orders::handle_order_requests,
                orders::update_orders_controller,
            )
                .chain(),
            sequences::update_motion_controllers,
            sequences::update_poses,
            rebuild_static_bvh.run_if(|q: Query<(), Added<ecs::BoundingBoxComponent>>| {
                q.iter().count() > 0
            }),
            update_dynamic_bvh,
            sequences::_debug_draw_root_motion,
        )
            .in_set(Update)
            .chain(),
    );

    // End
    schedule.add_systems(reset_input_state.in_set(End));

    schedule
}

/// Build the extract [Schedule]. Run once per render frame to populate
/// `WorldRenderSnapshot` from the simulation `World`.
pub fn build_extract_schedule() -> Schedule {
    extract::create_extract_schedule()
}

fn reset_input_state(mut input: ResMut<InputState>) {
    input.reset_per_frame();
}

fn should_run_simulation_update(control: Res<SimulationControl>) -> bool {
    control.run_update
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
