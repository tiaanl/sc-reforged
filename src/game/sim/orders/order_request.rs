use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::game::sim::orders::{
    ActiveOrder,
    active_order::{move_to::MoveToOrder, move_to_attack::MoveToAttackOrder},
};

#[derive(Debug)]
pub struct OrderDescriptor {
    pub class: OrderClass,
    pub default_priority: OrderPriority,
    pub cancel_policy: OrderCancelPolicy,
}

pub const MOVE_TO_DESCRIPTOR: OrderDescriptor = OrderDescriptor {
    class: OrderClass::Unknown,
    default_priority: OrderPriority::Normal,
    cancel_policy: OrderCancelPolicy::Never,
};

pub const MOVE_TO_ATTACK_DESCRIPTOR: OrderDescriptor = OrderDescriptor {
    class: OrderClass::Unknown,
    default_priority: OrderPriority::Normal,
    cancel_policy: OrderCancelPolicy::Never,
};

#[derive(Clone, Copy, Debug)]
pub enum OrderClass {
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub enum OrderPriority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Copy, Debug)]
pub enum OrderCancelPolicy {
    Never,
}

#[derive(Clone, Debug)]
pub enum RequestedOrder {
    MoveTo { location: Vec3 },
    MoveToAttack { entity: Entity },
}

impl RequestedOrder {
    /// Returns the descriptor for this requested order.
    pub fn descriptor(&self) -> &'static OrderDescriptor {
        match self {
            RequestedOrder::MoveTo { .. } => &MOVE_TO_DESCRIPTOR,
            RequestedOrder::MoveToAttack { .. } => &MOVE_TO_ATTACK_DESCRIPTOR,
        }
    }

    /// Instantiates a runtime order from the request payload.
    pub fn into_active(self) -> ActiveOrder {
        match self {
            RequestedOrder::MoveTo { location } => ActiveOrder::MoveTo(MoveToOrder { location }),
            RequestedOrder::MoveToAttack { entity } => {
                ActiveOrder::MoveToAttack(MoveToAttackOrder { entity })
            }
        }
    }
}

#[derive(Clone, Debug, Message)]
pub struct OrderRequest {
    /// The [Entity] for which the order is requested.
    pub entity: Entity,
    /// The [Order].
    pub order: RequestedOrder,
    /// If set will override the default priority of the order.
    pub priority_override: Option<OrderPriority>,
}
