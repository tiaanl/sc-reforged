use glam::Mat4;

use crate::{
    engine::storage::Handle,
    game::renderer::{RenderAnimation, render_models::RenderModel},
};

/// Holds animation data for a [RenderInstance].
#[derive(Clone, Copy)]
pub struct RenderInstanceAnimation {
    /// Handle to the animation to play.
    pub handle: Handle<RenderAnimation>,
    /// The time to calculate the animation frame.
    pub time: f32,
}

impl RenderInstanceAnimation {
    pub fn from_animation(animation: Handle<RenderAnimation>) -> Self {
        Self {
            handle: animation,
            time: 0.0,
        }
    }
}

/// Represents a model being rendered in the world.
pub struct RenderInstance {
    /// The model to render.
    pub render_model: Handle<RenderModel>,
    /// An ID stored to write into the pick system.
    pub entity_id: u32,
    /// The transform to render the model at.
    pub transform: Mat4,
    /// An optional animation to play for the model.
    pub animation: Option<RenderInstanceAnimation>,
}

impl RenderInstance {
    pub fn new(render_model: Handle<RenderModel>, entity_id: u32, transform: Mat4) -> Self {
        Self {
            render_model,
            entity_id,
            transform,
            animation: None,
        }
    }
}
