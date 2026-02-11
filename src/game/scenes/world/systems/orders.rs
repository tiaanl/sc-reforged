use bevy_ecs::prelude::*;

use crate::{
    engine::transform::Transform,
    game::scenes::world::{
        sim_world::{Order, OrderMessage, Sequencer},
        systems::Time,
    },
};

pub fn issue_new_orders(
    mut orders: Query<&mut Order>,
    mut new_orders_reader: MessageReader<OrderMessage>,
) {
    for message in new_orders_reader.read() {
        match message {
            OrderMessage::New {
                entity,
                order: new_order,
            } => {
                if let Ok(mut order) = orders.get_mut(*entity) {
                    tracing::info!("Issuing new order {new_order:?} to {entity}");
                    *order = new_order.clone();
                }
            }
        }
    }
}

pub fn process_biped_orders(
    mut orders: Query<(Entity, &mut Transform, &mut Order, &mut Sequencer)>,
    mut new_orders: MessageWriter<OrderMessage>,
    time: Res<Time>,
) {
    let delta_time = time.delta_time;

    for (entity, mut transform, mut order, mut sequencer) in orders.iter_mut() {
        // We do the match here instead of just calling order.update so that
        // each order's update can have a different signature.
        let done = match *order {
            Order::Idle => {
                // Idle is never done?
                false
            }
            Order::_Move(ref mut order_move) => {
                order_move.update(&mut transform, delta_time, &mut sequencer)
            }
        };

        if done {
            new_orders.write(OrderMessage::New {
                entity,
                order: Order::Idle,
            });
        }
    }
}
