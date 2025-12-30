use bevy_ecs::prelude::*;

use crate::game::scenes::world::{
    sim_world::{ObjectData, Objects, OrderKind, OrderQueue},
    systems::Time,
};

pub fn update(mut objects: ResMut<Objects>, time: Res<Time>) {
    for (_, object) in objects.objects.iter_mut() {
        match &mut object.data {
            ObjectData::Scenery { .. } => {}
            ObjectData::Biped {
                order_queue,
                sequencer,
                ..
            } => {
                order_queue.update(&time);
                sequencer.update(&time);
            }
            ObjectData::SingleModel { .. } => {}
        }
    }
}

fn _update_order_queue(order_queue: &mut OrderQueue) {
    let Some(order) = order_queue._current() else {
        return;
    };

    match order.kind {
        OrderKind::Move {
            world_position: _world_position,
        } => todo!(),
    }
}
