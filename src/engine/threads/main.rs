//! The main thread is the one that is spawned when the application is started.
//! We don't start a new "main" thread, but we need some structures for message
//! passing.

use winit::event_loop::EventLoopProxy;

use crate::engine::scene::Scene;

/// Receiver for main thread events.
#[derive(Clone)]
pub struct MainThreadReceiver {
    proxy: EventLoopProxy<MainThreadEvent>,
}

impl MainThreadReceiver {
    pub fn new(proxy: EventLoopProxy<MainThreadEvent>) -> Self {
        Self { proxy }
    }

    /// Request that the current [Scene] be replaces by the given one.
    pub fn replace_scene(&self, new_scene: Box<dyn Scene>) {
        if let Err(err) = self
            .proxy
            .send_event(MainThreadEvent::ReplaceScene(new_scene))
        {
            tracing::error!("Failed to notify main thread of scene change. ({err})");
        }
    }
}

pub enum MainThreadEvent {
    /// Replace the current [Scene] with the given one.
    ReplaceScene(Box<dyn Scene>),
}
