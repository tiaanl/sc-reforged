use bevy_ecs::prelude::*;

use crate::{
    engine::storage::Handle,
    game::{
        AssetReader,
        model::Model,
        scenes::world::{
            animation::{generate_pose, pose::Pose},
            sim_world::{Sequencer, Sequences},
            systems::Time,
        },
    },
};

pub fn enqueue_next_sequences(
    mut sequencers: Query<&mut Sequencer>,
    sequences: Res<Sequences>,
    assets: Res<AssetReader>,
) {
    for mut sequencer in sequencers.iter_mut() {
        if let Some(sequence) = sequencer.next_sequence()
            && let Some(sequence_def) = sequences.sequence_def_by_name(&sequence)
        {
            sequencer.enqueue(&assets, sequence_def);
        }
    }
}

pub fn update_sequencers(mut sequencers: Query<&mut Sequencer>, time: Res<Time>) {
    for mut sequencer in sequencers.iter_mut() {
        sequencer.update(&time);
    }
}

pub fn update_entity_poses(
    sequencers: Query<(&Handle<Model>, &mut Pose, &Sequencer)>,
    assets: Res<AssetReader>,
) {
    for (&model_handle, mut pose, sequencer) in sequencers {
        if let Some((motion_handle, time)) = sequencer.get() {
            let Some(model) = assets.get_model(model_handle) else {
                continue;
            };

            let Some(motion) = assets.get_motion(motion_handle) else {
                continue;
            };

            *pose = generate_pose(&model.skeleton, motion, time, true);
        }
    }
}
