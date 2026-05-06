use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use crate::game::sim::orders::{
    ActiveOrder, OrderRequest, order_request::OrderPriority,
};

#[derive(Component, Default)]
pub struct OrdersController {
    active_order: Option<ActiveOrderState>,
    pending_orders: VecDeque<PendingOrder>,
}

impl OrdersController {
    /// Enqueues an order request for later arbitration and activation.
    pub fn enqueue_request(&mut self, request: OrderRequest, frame: u64) {
        let effective_priority = request
            .priority_override
            .unwrap_or(request.order.descriptor().default_priority);

        self.pending_orders.push_front(PendingOrder {
            request,
            effective_priority,
            requested_at_frame: frame,
        });
    }

    /// Execute the current order.
    pub fn execute(&mut self, frame_index: u64) {
        // TODO: Even if there is an active order, peek any pending orders for one with a higher
        //       priority.

        if self.active_order.is_none() && !self.pending_orders.is_empty() {
            let pending = self.pending_orders.pop_front().unwrap();

            let runtime = pending.request.order.clone().into_active();

            self.active_order = Some(ActiveOrderState {
                runtime,
                started_at_frame: frame_index,
                effective_priority: pending.effective_priority,
                source_request: pending.request,
            });
        }

        if let Some(order) = self.active_order.as_mut() {
            tracing::info!("({}) Executing active order: {order:?}", frame_index);
            if let super::active_order::ExecuteOutcome::Complete = order.runtime.execute() {
                self.active_order = None;
            }
        }
    }
}

struct PendingOrder {
    pub request: OrderRequest,
    pub effective_priority: OrderPriority,
    pub requested_at_frame: u64,
}

#[derive(Debug)]
struct ActiveOrderState {
    pub runtime: ActiveOrder,
    pub started_at_frame: u64,
    pub effective_priority: OrderPriority,
    pub source_request: OrderRequest,
}
