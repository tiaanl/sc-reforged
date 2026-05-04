use glam::Vec3;

use crate::engine::renderer::RenderContext;

use super::super::render::GeometryBuffer;

pub fn clear_render_targets(
    render_context: &mut RenderContext,
    geometry_buffer: &GeometryBuffer,
    fog_color: Vec3,
) {
    geometry_buffer.clear(&mut render_context.encoder, fog_color);
}
