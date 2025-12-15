pub enum EngineEvent {
    /// Request to exit the engine.
    Exit,
}

#[derive(Clone)]
pub struct EngineContext {
    event_loop_proxy: winit::event_loop::EventLoopProxy<EngineEvent>,
}

impl EngineContext {
    pub fn new(event_loop_proxy: winit::event_loop::EventLoopProxy<EngineEvent>) -> Self {
        Self { event_loop_proxy }
    }
}
