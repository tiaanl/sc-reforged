use bevy_ecs::prelude::*;
use glam::Vec4;

use crate::{
    engine::transform::Transform,
    game::scenes::world::sim_world::ecs::{BoundingBoxComponent, GizmoVertices},
};

pub fn draw_model_bounding_boxes(
    models: Query<(&Transform, &BoundingBoxComponent)>,
    mut gizmo_vertices: ResMut<GizmoVertices>,
) {
    let color = Vec4::new(1.0, 0.0, 0.0, 1.0);

    for (transform, bounding_box) in models.iter() {
        let actual = bounding_box.0.transformed(transform.to_mat4());
        gizmo_vertices.draw_bounding_box(&actual, color);
    }
}
