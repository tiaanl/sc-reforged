use glam::Vec3;

use crate::engine::renderer::Frame;

use super::super::render::GeometryBuffer;

pub fn clear_render_targets(frame: &mut Frame, geometry_buffer: &GeometryBuffer, fog_color: Vec3) {
    geometry_buffer.clear(&mut frame.encoder, fog_color);
}
