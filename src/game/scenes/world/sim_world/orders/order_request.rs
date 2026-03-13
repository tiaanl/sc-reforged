use bevy_ecs::prelude::*;

use super::order::Order;

#[derive(Clone, Debug, Message)]
pub struct OrderRequest {
    /// The [Entity] for which the order is requested.
    pub entity: Entity,
    /// The [Order].
    pub order: Order,
}
