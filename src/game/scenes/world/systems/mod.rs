use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::scenes::world::{render_world::RenderWorld, sim_world::SimWorld},
};

pub mod camera_system;
pub mod day_night_cycle_system;
pub mod free_camera_controller;
pub mod top_down_camera_controller;

pub struct Time {
    pub delta_time: f32,
}

#[allow(unused_variables)]
pub trait System {
    /// Stage: Gather signals & schedule work for this frame.
    fn pre_update(&mut self, sim_world: &mut SimWorld, time: &Time, input_state: &InputState) {}

    /// Stage: Authoritative game state changes.
    fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {}

    /// Stage: Finalize sim results & housekeeping.
    fn post_update(&mut self, sim_world: &mut SimWorld) {}

    /// Stage: Copy read-only data needed for rendering into the [RenderWorld] (buffered).
    fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld) {}

    /// Stage: CPU work that produces GPU-ready data.
    fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer) {}

    /// Stage: Record command buffers & render passes.
    fn queue(&mut self, render_world: &RenderWorld, frame: &mut Frame) {}
}
