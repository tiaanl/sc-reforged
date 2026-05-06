use bevy_ecs::prelude::*;

use crate::game::sim::{
    orders::{OrderRequest, OrdersController},
    systems::Time,
};

pub fn handle_order_requests(
    mut requests: MessageReader<OrderRequest>,
    mut orders_controllers: Query<&mut OrdersController>,
    time: Res<Time>,
) {
    for request in requests.read() {
        if let Ok(mut orders_controller) = orders_controllers.get_mut(request.entity) {
            tracing::info!("({}) Requesting order {request:?}", time.frame_index);
            orders_controller.enqueue_request(request.clone(), time.frame_index);
        } else {
            tracing::warn!("Entity has no OrdersController ({})", request.entity);
        }
    }
}

pub fn update_orders_controller(
    mut orders_controllers: Query<&mut OrdersController>,
    time: Res<Time>,
) {
    for mut orders_controller in orders_controllers.iter_mut() {
        orders_controller.execute(time.frame_index);
    }
}
