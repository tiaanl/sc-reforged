use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::orders::{Order, OrderRequest};

use super::order::ExecuteOutcome;

#[derive(Component, Default)]
pub struct OrdersController {
    active_order: Option<OrderRequest>,
    pending_orders: VecDeque<OrderRequest>,
}

impl OrdersController {
    pub fn request_order(&mut self, request: OrderRequest) {
        self.pending_orders.push_front(request);
    }

    /// Execute the current order.
    pub fn execute(&mut self, frame_index: u64) {
        // TODO: Even if there is an active order, peek any pending orders for one with a higher
        //       priority.

        if self.active_order.is_none() && !self.pending_orders.is_empty() {
            self.active_order = self.pending_orders.pop_front();
        }

        if let Some(OrderRequest { ref mut order, .. }) = self.active_order {
            tracing::info!("({}) Executing active order: {order:?}", frame_index);
            match order.execute() {
                ExecuteOutcome::Executing => {}
                ExecuteOutcome::Complete => self.active_order = None,
            }
        }
    }
}
