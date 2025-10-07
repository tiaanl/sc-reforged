use ahash::HashMap;

use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::scenes::world::{render_world::RenderWorld, sim_world::SimWorld},
};

pub mod camera_system;
pub mod clear_render_targets;
pub mod cull_system;
pub mod day_night_cycle_system;
pub mod free_camera_controller;
pub mod gizmo_system;
pub mod terrain_system;
pub mod top_down_camera_controller;

pub struct Time {
    pub delta_time: f32,
}

pub struct NewSystemContext<'a> {
    pub renderer: &'a Renderer,
    pub render_store: &'a mut RenderStore,
    pub sim_world: &'a SimWorld,
}

pub struct PreUpdateContext<'a> {
    pub sim_world: &'a mut SimWorld,
    pub time: &'a Time,
    pub input_state: &'a InputState,
}

pub struct UpdateContext<'a> {
    pub sim_world: &'a mut SimWorld,
    pub time: &'a Time,
}

pub struct PostUpdateContext<'a> {
    pub sim_world: &'a mut SimWorld,
}

pub struct ExtractContext<'a> {
    pub sim_world: &'a mut SimWorld,
    pub render_world: &'a mut RenderWorld,
}

pub struct PrepareContext<'a> {
    pub render_world: &'a mut RenderWorld,
    pub renderer: &'a Renderer,
    pub render_store: &'a mut RenderStore,
}

pub struct QueueContext<'a> {
    pub render_world: &'a RenderWorld,
    pub frame: &'a mut Frame,
    pub render_store: &'a RenderStore,
}

#[allow(unused_variables)]
pub trait System {
    /// Stage: Gather signals & schedule work for this frame.
    fn pre_update(&mut self, context: &mut PreUpdateContext) {}

    /// Stage: Authoritative game state changes.
    fn update(&mut self, context: &mut UpdateContext) {}

    /// Stage: Finalize sim results & housekeeping.
    fn post_update(&mut self, context: &mut PostUpdateContext) {}

    /// Stage: Copy read-only data needed for rendering into the [RenderWorld] (buffered).
    fn extract(&mut self, context: &mut ExtractContext) {}

    /// Stage: CPU work that produces GPU-ready data.
    fn prepare(&mut self, context: &mut PrepareContext) {}

    /// Stage: Record command buffers & render passes.
    fn queue(&mut self, context: &mut QueueContext) {}
}

#[derive(Default)]
pub struct RenderStore {
    bind_group_layouts: HashMap<&'static str, wgpu::BindGroupLayout>,
    bind_groups: HashMap<&'static str, wgpu::BindGroup>,
}

macro_rules! impl_render_store_item {
    ($get_name:ident, $insert_name:ident, $var:ident, $ty:ty) => {
        impl RenderStore {
            #[inline]
            pub fn $get_name(&self, id: &'static str) -> Option<$ty> {
                self.$var.get(id).cloned()
            }

            pub fn $insert_name(&mut self, id: &'static str, value: $ty) {
                if self.$var.insert(id, value).is_some() {
                    tracing::warn!("Replacing item: {id}");
                }
            }
        }
    };
}

impl_render_store_item!(
    get_bind_group_layout,
    store_bind_group_layout,
    bind_group_layouts,
    wgpu::BindGroupLayout
);
impl_render_store_item!(
    get_bind_group,
    store_bind_group,
    bind_groups,
    wgpu::BindGroup
);
