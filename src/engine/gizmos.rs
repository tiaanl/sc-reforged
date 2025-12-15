#![allow(dead_code)]

use glam::{Mat4, Vec3, Vec4};

use crate::game::math::{BoundingBox, ViewProjection};

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct GizmoVertex {
    pub position: Vec3,
    _padding: f32,
    pub color: Vec4,
}

impl GizmoVertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self {
            position,
            _padding: 1.0,
            color,
        }
    }
}

pub fn create_axis(transform: Mat4, size: f32) -> Vec<GizmoVertex> {
    let zero = transform.project_point3(Vec3::ZERO);
    vec![
        GizmoVertex::new(zero, Vec4::new(1.0, 0.0, 0.0, 1.0)),
        GizmoVertex::new(
            transform.project_point3(Vec3::X * size),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        ),
        GizmoVertex::new(zero, Vec4::new(0.0, 1.0, 0.0, 1.0)),
        GizmoVertex::new(
            transform.project_point3(Vec3::Y * size),
            Vec4::new(0.0, 1.0, 0.0, 1.0),
        ),
        GizmoVertex::new(zero, Vec4::new(0.0, 0.0, 1.0, 1.0)),
        GizmoVertex::new(
            transform.project_point3(Vec3::Z * size),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        ),
    ]
}

pub fn create_iso_sphere(transform: Mat4, radius: f32, resolution: i32) -> Vec<GizmoVertex> {
    let mut vertices = Vec::new();
    let res = resolution.max(3);

    // Each axis defines the normal of the circle's plane.
    // For each axis, we need to pick two orthogonal vectors to define the circle.
    let axes = [
        (Vec3::Y, Vec3::Z, Vec4::new(1.0, 0.0, 0.0, 1.0)), // X: YZ plane (red)
        (Vec3::Z, Vec3::X, Vec4::new(0.0, 1.0, 0.0, 1.0)), // Y: ZX plane (green)
        (Vec3::X, Vec3::Y, Vec4::new(0.0, 0.5, 1.0, 1.0)), // Z: XY plane (blue)
    ];

    for (u, v, color) in axes {
        for i in 0..res {
            let theta0 = (i as f32) * std::f32::consts::TAU / (res as f32);
            let theta1 = ((i + 1) as f32) * std::f32::consts::TAU / (res as f32);

            let p0 = transform.transform_point3((u * theta0.cos() + v * theta0.sin()) * radius);
            let p1 = transform.transform_point3((u * theta1.cos() + v * theta1.sin()) * radius);

            vertices.push(GizmoVertex::new(p0, color));
            vertices.push(GizmoVertex::new(p1, color));
        }
    }

    vertices
}

pub fn create_view_projection(view_projection: &ViewProjection, color: Vec4) -> Vec<GizmoVertex> {
    const EDGES: &[(usize, usize)] = &[
        // near ring
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        // sides
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
        // far ring
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
    ];

    let mut result = Vec::with_capacity(EDGES.len() * 2);

    let v: [GizmoVertex; 8] = view_projection
        .corners()
        .map(|p| GizmoVertex::new(p, color));

    for &(from, to) in EDGES {
        result.push(v[from]);
        result.push(v[to]);
    }

    result
}

pub fn create_bounding_box(bounding_box: &BoundingBox, color: Vec4) -> Vec<GizmoVertex> {
    let min: Vec3 = bounding_box.min;
    let max: Vec3 = bounding_box.max;

    // 8 corners of the box
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];

    // Each pair defines a line segment (edge)
    const EDGES: &[(usize, usize)] = &[
        // bottom face
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        // top face
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        // vertical edges
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    let mut result = Vec::with_capacity(EDGES.len() * 2);

    for (a, b) in EDGES {
        result.push(GizmoVertex::new(corners[*a], color));
        result.push(GizmoVertex::new(corners[*b], color));
    }

    result
}
