use std::collections::VecDeque;

use super::orders::Order;

/// A queue of orders that will be completed in FIFO order.
#[derive(Debug, Default)]
pub struct OrderQueue {
    orders: VecDeque<Order>,
    current: Option<Order>,
}

impl OrderQueue {
    pub fn enqueue(&mut self, order: Order) {
        self.orders.push_back(order);
    }

    pub fn _current(&self) -> Option<&Order> {
        self.current.as_ref()
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .inner_margin(4)
            .corner_radius(ui.visuals().window_corner_radius)
            .stroke(egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .fill(ui.visuals().extreme_bg_color)
            .show(ui, |ui| {
                if let Some(current) = &self.current {
                    current.ui(ui);
                }

                for order in self.orders.iter() {
                    order.ui(ui);
                }
            });
    }
}
