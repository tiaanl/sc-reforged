use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::scenes::world::{render_world::RenderWorld, sim_world::SimWorld},
};

pub mod day_night_cycle_system;
pub mod free_camera_system;
pub mod top_down_camera_system;

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

// /// Stage: Gather signals & schedule work for this frame.
// pub trait PreUpdate {
//     fn pre_update(&mut self, sim_world: &mut SimWorld, time: &Time, input_state: &InputState);
// }

// pub trait AsPreUpdate {
//     fn as_pre_update(&mut self) -> Option<&mut dyn PreUpdate> {
//         None
//     }
// }

// impl<T> AsPreUpdate for T
// where
//     T: PreUpdate,
// {
//     fn as_pre_update(&mut self) -> Option<&mut dyn PreUpdate> {
//         Some(self)
//     }
// }

// #[inline]
// pub fn system_pre_update<S: AsPreUpdate>(
//     system: &mut S,
//     sim_world: &mut SimWorld,
//     time: &Time,
//     input_state: &InputState,
// ) {
//     if let Some(system) = S::as_pre_update(system) {
//         system.pre_update(sim_world, time, input_state)
//     }
// }

// /// Stage: Authoritative game state changes.
// pub trait Update {
//     fn update(&mut self, sim_world: &mut SimWorld, time: &Time);
// }

// #[inline]
// pub fn system_update(system: &mut impl Update, sim_world: &mut SimWorld, time: &Time) {
//     system.update(sim_world, time)
// }

// /// Stage: Finalize sim results & housekeeping.
// pub trait PostUpdate {
//     fn post_update(&mut self, sim_world: &mut SimWorld);
// }

// #[inline]
// pub fn system_post_update(system: &mut impl PostUpdate, sim_world: &mut SimWorld) {
//     system.post_update(sim_world)
// }

// /// Stage: Copy read-only data needed for rendering into the [RenderWorld] (buffered).
// pub trait Extract {
//     fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld);
// }

// #[inline]
// pub fn system_extract(
//     system: &mut impl Extract,
//     sim_world: &SimWorld,
//     render_world: &mut RenderWorld,
// ) {
//     system.extract(sim_world, render_world)
// }

// /// Stage: CPU work that produces GPU-ready data.
// pub trait Prepare {
//     fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer);
// }

// #[inline]
// pub fn system_prepare(
//     system: &mut impl Prepare,
//     render_world: &mut RenderWorld,
//     renderer: &Renderer,
// ) {
//     system.prepare(render_world, renderer);
// }

// /// Stage: Record command buffers & render passes.
// pub trait Queue {
//     fn queue(&mut self, render_world: &RenderWorld, frame: &mut Frame);
// }

// #[inline]
// pub fn system_queue(system: &mut impl Queue, render_world: &RenderWorld, frame: &mut Frame) {
//     system.queue(render_world, frame);
// }
