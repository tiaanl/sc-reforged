use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::scenes::world::{render_world::RenderWorld, sim_world::SimWorld},
};

pub mod free_camera_system;
pub mod top_down_camera_system;

pub struct Time {
    pub delta_time: f32,
}

pub trait PreUpdate {
    fn pre_update(&mut self, sim_world: &mut SimWorld, time: &Time, input_state: &InputState);
}

#[inline]
pub fn system_pre_update(
    system: &mut impl PreUpdate,
    sim_world: &mut SimWorld,
    time: &Time,
    input_state: &InputState,
) {
    system.pre_update(sim_world, time, input_state)
}

pub trait Update {
    fn update(&mut self, sim_world: &mut SimWorld, time: &Time);
}

#[inline]
pub fn system_update(system: &mut impl Update, sim_world: &mut SimWorld, time: &Time) {
    system.update(sim_world, time)
}

pub trait PostUpdate {
    fn post_update(&mut self, sim_world: &mut SimWorld);
}

#[inline]
pub fn system_post_update(system: &mut impl PostUpdate, sim_world: &mut SimWorld) {
    system.post_update(sim_world)
}

pub trait Extract {
    fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld);
}

#[inline]
pub fn system_extract(
    system: &mut impl Extract,
    sim_world: &SimWorld,
    render_world: &mut RenderWorld,
) {
    system.extract(sim_world, render_world)
}

pub trait Prepare {
    fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer);
}

#[inline]
pub fn system_prepare(
    system: &mut impl Prepare,
    render_world: &mut RenderWorld,
    renderer: &Renderer,
) {
    system.prepare(render_world, renderer);
}

pub trait Queue {
    fn queue(&mut self, render_world: &RenderWorld, frame: &mut Frame);
}

#[inline]
pub fn system_queue(system: &mut impl Queue, render_world: &RenderWorld, frame: &mut Frame) {
    system.queue(render_world, frame);
}
