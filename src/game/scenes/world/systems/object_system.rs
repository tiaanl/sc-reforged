use crate::game::scenes::world::{
    sim_world::{ObjectData, OrderKind, OrderQueue, SimWorld},
    systems::Time,
};

pub struct ObjectSystem;

impl ObjectSystem {
    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        for (_, object) in sim_world.objects.objects.iter_mut() {
            match &mut object.data {
                ObjectData::Scenery { .. } => {}
                ObjectData::Biped {
                    order_queue,
                    sequencer,
                    ..
                } => {
                    order_queue.update(time);
                    sequencer.update(time);
                }
                ObjectData::SingleModel { .. } => {}
            }
        }
    }

    fn update_order_queue(order_queue: &mut OrderQueue) {
        let Some(order) = order_queue.current() else {
            return;
        };

        match order.kind {
            OrderKind::Move { world_position } => todo!(),
        }
    }
}
