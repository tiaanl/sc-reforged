use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::{
    engine::{storage::Handle, transform::Transform},
    game::{
        AssetReader,
        model::Model,
        scenes::world::sim_world::{
            ecs::GizmoVertices,
            sequences::{MotionController, MotionSequencer, Pose, generate_pose},
        },
    },
};

use super::Time;

/// Advance all motion controllers for the current frame.
pub fn update_motion_controllers(
    mut motion_controllers: Query<(&mut MotionController, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut motion_controller, mut transform) in motion_controllers.iter_mut() {
        motion_controller.update(time.delta_time);

        // Once the motion has been calculated, adjust the transform of the
        // entity by the `root_motion` from the [MotionController].
        let adjust = transform.rotation * motion_controller.root_motion;
        transform.translation += adjust;
    }
}

/// Build a full pose for each animated entity from the currently active motion.
pub fn update_poses(
    mut poses: Query<(&MotionController, &Handle<Model>, &mut Pose)>,
    assets: Res<AssetReader>,
    motion_sequencer: Res<MotionSequencer>,
) {
    for (motion_controller, model_handle, mut pose) in poses.iter_mut() {
        let Some(model) = assets.get_model(*model_handle) else {
            continue;
        };
        let skeleton = &model.skeleton;

        let Some(active) = motion_controller.active.as_ref() else {
            if pose.bones.len() != skeleton.bones.len() {
                *pose = skeleton.to_pose();
            }
            continue;
        };

        let sample_time = if active.scaled_ticks_per_frame <= 0 {
            0.0
        } else {
            active.current_time_ticks.max(0) as f32 / active.scaled_ticks_per_frame as f32
        };

        let root_translation_override =
            motion_sequencer.default_cog_position(motion_controller.transition_check_state());

        *pose = generate_pose(
            skeleton,
            active.motion_info.motion.as_ref(),
            sample_time,
            active.motion_info.looping,
            root_translation_override,
        );
    }
}

pub fn _debug_draw_root_motion(
    query: Query<(&Transform, &MotionController)>,
    mut gizmos: ResMut<GizmoVertices>,
) {
    const COLOR: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);

    for (transform, motion_controller) in query.iter() {
        let origin = transform.translation;
        let dir = transform.rotation * motion_controller.root_motion;

        gizmos.draw_line(origin, origin + dir * 100.0, COLOR);
    }
}
