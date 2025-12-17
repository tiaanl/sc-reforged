use std::sync::Arc;

use winit::event_loop::EventLoopProxy;

use crate::engine::scene::{LoadContext, Scene, SceneLoader};

pub enum EngineEvent {
    /// Request to exit the engine.
    _Exit,
    /// Request that the [Scene] be switched to the specified one.
    SwitchScene(Box<dyn Scene>),
}

#[derive(Clone)]
pub struct EngineContext {
    event_loop_proxy: EventLoopProxy<EngineEvent>,
    load_context: Arc<LoadContext>,
}

impl EngineContext {
    pub fn new(event_loop_proxy: EventLoopProxy<EngineEvent>, load_context: LoadContext) -> Self {
        Self {
            event_loop_proxy,
            load_context: Arc::new(load_context),
        }
    }

    pub fn switch_scene<S: SceneLoader>(&self, scene_loader: S) {
        let engine_context = self.clone();
        let load_context = Arc::clone(&self.load_context);
        let event_loop_proxy = self.event_loop_proxy.clone();

        std::thread::spawn(move || {
            // let load_context = load_context;
            let scene = scene_loader
                .load(engine_context, load_context.as_ref())
                .unwrap();
            let _ = event_loop_proxy.send_event(EngineEvent::SwitchScene(scene));
        });
    }
}
