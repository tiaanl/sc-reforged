use glam::Vec3;

use crate::engine::prelude::Frame;

use super::super::render::{GeometryBuffer, RenderWorld};

pub fn clear_render_targets(
    render_world: &RenderWorld,
    frame: &mut Frame,
    geometry_buffer: &GeometryBuffer,
) {
    geometry_buffer.clear(
        &mut frame.encoder,
        Vec3::from_slice(&render_world.camera_env.fog_color),
    );
}
