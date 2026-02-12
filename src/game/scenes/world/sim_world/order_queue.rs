use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use super::orders::Order;

/// Per-entity FIFO queue of pending orders waiting to become active.
#[derive(Component, Debug, Default)]
pub struct PendingOrders {
    /// Orders waiting to be promoted to the active [Order] component.
    pending: VecDeque<Order>,
}

impl PendingOrders {
    /// Append a new order to the queue.
    #[inline]
    pub fn enqueue(&mut self, order: Order) {
        self.pending.push_back(order);
    }

    /// Pop the next queued order from the front of the FIFO queue.
    #[inline]
    pub fn pop_next(&mut self) -> Option<Order> {
        self.pending.pop_front()
    }

    /// Return whether no pending orders are queued.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Number of pending orders currently queued.
    #[inline]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Iterate pending orders in FIFO order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Order> {
        self.pending.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::scenes::world::sim_world::orders::{ChangeStateTarget, OrderChangeState};

    #[test]
    fn enqueue_and_pop_preserve_fifo_order() {
        let mut queue = PendingOrders::default();
        queue.enqueue(Order::Idle);
        queue.enqueue(Order::ChangeState(OrderChangeState::new(
            ChangeStateTarget::Stand,
        )));
        queue.enqueue(Order::ChangeState(OrderChangeState::new(
            ChangeStateTarget::Prone,
        )));

        assert_eq!(queue.pop_next(), Some(Order::Idle));
        assert_eq!(
            queue.pop_next(),
            Some(Order::ChangeState(OrderChangeState::new(
                ChangeStateTarget::Stand
            )))
        );
        assert_eq!(
            queue.pop_next(),
            Some(Order::ChangeState(OrderChangeState::new(
                ChangeStateTarget::Prone
            )))
        );
        assert_eq!(queue.pop_next(), None);
    }

    #[test]
    fn empty_queue_reports_zero_and_none() {
        let mut queue = PendingOrders::default();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop_next(), None);
    }

    #[test]
    fn len_and_is_empty_track_enqueue_and_pop() {
        let mut queue = PendingOrders::default();
        queue.enqueue(Order::Idle);
        queue.enqueue(Order::ChangeState(OrderChangeState::new(
            ChangeStateTarget::Crouch,
        )));

        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop_next(), Some(Order::Idle));
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
        let _ = queue.pop_next();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
}
