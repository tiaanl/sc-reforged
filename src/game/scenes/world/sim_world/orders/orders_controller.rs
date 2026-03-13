use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use super::order::Order;

pub struct OrderRequest {
    order: Order,
}

#[derive(Component, Default)]
pub struct OrdersController {
    pending_orders: VecDeque<OrderRequest>,
}

impl OrdersController {
    pub fn request_order(&mut self, request: OrderRequest) {
        self.pending_orders.push_front(request);
    }
}

fn update_orders_controller(mut orders_controllers: Query<&mut OrdersController>) {
    for mut orders_controller in orders_controllers.iter_mut() {
        //
    }
}
