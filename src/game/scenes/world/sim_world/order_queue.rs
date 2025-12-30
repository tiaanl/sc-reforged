use std::collections::VecDeque;

use crate::{engine::egui_integration::UiExt, game::scenes::world::systems::Time};

use super::orders::Order;

/// A queue of orders that will be completed in FIFO order.
#[derive(Default)]
pub struct OrderQueue {
    /// List FIFO queue of pending orders that still need to be performed.
    pending: VecDeque<Order>,
    /// The current order being performed.
    current: Option<Order>,
}

impl OrderQueue {
    pub fn update(&mut self, _time: &Time) {
        // If there is no current order, but there are pending orders, grab
        // the next order from the queue.
        if self.current.is_none() && !self.pending.is_empty() {
            self.current = self.pending.pop_front();
        }
    }

    pub fn enqueue(&mut self, order: Order) {
        self.pending.push_back(order);
    }

    pub fn current(&self) -> Option<&Order> {
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
                    ui.h2("Current");
                    current.ui(ui);
                }

                if !self.pending.is_empty() {
                    ui.h2("Pending");
                    for order in self.pending.iter() {
                        order.ui(ui);
                    }
                }
            });
    }
}
