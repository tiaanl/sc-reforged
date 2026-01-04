use bevy_ecs::prelude::*;

use crate::{
    engine::transform::Transform,
    game::scenes::world::{sim_world::Order, systems::Time},
};

pub fn process_biped_orders(mut orders: Query<(&mut Transform, &mut Order)>, time: Res<Time>) {
    let delta_time = time.delta_time;

    for (mut transform, mut order) in orders.iter_mut() {
        // We do the match here instead of just calling order.update so that
        // each order's update can have a different signature.
        match *order {
            Order::Idle => {}
            Order::_Move(ref mut order_move) => {
                order_move.update(&mut transform, delta_time);
            }
        }
    }
}
