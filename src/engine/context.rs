use winit::event_loop::EventLoopProxy;

use crate::engine::scene::{Scene, SceneLoader};

pub enum EngineEvent {
    /// Request to exit the engine.
    _Exit,
    /// Request a scene to be loaded.
    LoadScene(Box<dyn SceneLoader>),
    /// Request that the [Scene] be switched to the specified one.
    SwitchScene(Box<dyn Scene>),
}

#[derive(Clone)]
pub struct EngineContext {
    pub event_loop_proxy: EventLoopProxy<EngineEvent>,
}

impl EngineContext {
    pub fn new(event_loop_proxy: EventLoopProxy<EngineEvent>) -> Self {
        Self { event_loop_proxy }
    }

    pub fn switch_scene<S: SceneLoader>(&self, scene_loader: S) {
        let _ = self
            .event_loop_proxy
            .send_event(EngineEvent::LoadScene(Box::new(scene_loader)));
    }
}
