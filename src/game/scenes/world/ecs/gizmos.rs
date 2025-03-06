use bevy_ecs::prelude as ecs;
use glam::{Vec3, Vec4};

use crate::engine::gizmos::GizmoVertex;

use super::Transform;

pub fn setup_ecs(world: &mut ecs::World, render_schedule: &mut ecs::Schedule) {
    world.init_resource::<GizmosBatch>();
    render_schedule.add_systems(gather_entity_gizmos);
}

#[derive(ecs::Component, Default)]
pub struct EntityGizmos {
    vertices: Vec<GizmoVertex>,
}

impl EntityGizmos {
    pub fn axis(&mut self, position: Vec3, size: f32) {
        self.vertices
            .push(GizmoVertex::new(position, Vec4::new(1.0, 0.0, 0.0, 1.0)));
        self.vertices.push(GizmoVertex::new(
            position + Vec3::X * size,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        ));

        self.vertices
            .push(GizmoVertex::new(position, Vec4::new(0.0, 1.0, 0.0, 1.0)));
        self.vertices.push(GizmoVertex::new(
            position + Vec3::Y * size,
            Vec4::new(0.0, 1.0, 0.0, 1.0),
        ));

        self.vertices
            .push(GizmoVertex::new(position, Vec4::new(0.0, 0.0, 1.0, 1.0)));
        self.vertices.push(GizmoVertex::new(
            position + Vec3::Z * size,
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        ));
    }
}

#[derive(Default, ecs::Resource)]
pub struct GizmosBatch {
    vertices: Vec<GizmoVertex>,
}

impl GizmosBatch {
    pub fn take(&mut self) -> Vec<GizmoVertex> {
        let mut vertices = Vec::default();
        std::mem::swap(&mut self.vertices, &mut vertices);
        vertices
    }
}

fn gather_entity_gizmos(
    mut gizmos_q: ecs::Query<(&Transform, &mut EntityGizmos)>,
    mut gizmos_batch: ecs::ResMut<GizmosBatch>,
) {
    for (transform, mut gizmos) in gizmos_q
        .iter_mut()
        .filter(|(_, gizmos)| !gizmos.vertices.is_empty())
    {
        let mat = transform.to_mat4();
        gizmos_batch
            .vertices
            .extend(gizmos.vertices.iter().map(|v| {
                let position = mat.project_point3(v.position);
                GizmoVertex::new(position, v.color)
            }));
        gizmos.vertices.clear();
    }
}
