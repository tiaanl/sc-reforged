#![allow(dead_code)]

use glam::{Mat4, Vec3, Vec4};

#[derive(Default)]
pub struct Matrices {
    /// Combined projection * view matrix.
    pub proj_view: Mat4,
    /// Inverse of the projection * view matrix.
    pub proj_view_inverse: Mat4,
}

impl Matrices {
    pub fn from_projection_view(projection: Mat4, view: Mat4) -> Self {
        let proj_view = projection * view;
        let proj_view_inverse = proj_view.inverse();

        Self {
            proj_view,
            proj_view_inverse,
        }
    }

    #[inline]
    pub fn unproject_ndc(&self, point: Vec3) -> Vec3 {
        debug_assert!(point.x >= -1.0 && point.x <= 1.0);
        debug_assert!(point.y >= -1.0 && point.y <= 1.0);
        debug_assert!(point.z >= -1.0 && point.z <= 1.0);
        self.proj_view_inverse.project_point3(point)
    }

    pub fn corners(&self) -> [Vec3; 8] {
        const NDC: &[(f32, f32)] = &[(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

        let mut result = [Vec3::ZERO; 8];
        for (i, &(x, y)) in NDC.iter().enumerate() {
            result[i] = self.unproject_ndc(Vec3::new(x, y, 0.0)); // near
            result[i + 4] = self.unproject_ndc(Vec3::new(x, y, 1.0)); // far
        }
        result
    }

    pub fn frustum(&self) -> Frustum {
        let r0 = self.proj_view.row(0);
        let r1 = self.proj_view.row(1);
        let r2 = self.proj_view.row(2);
        let r3 = self.proj_view.row(3);

        let left = Plane::from_row(r3 + r0);
        let right = Plane::from_row(r3 - r0);
        let bottom = Plane::from_row(r3 + r1);
        let top = Plane::from_row(r3 - r1);
        let near = Plane::from_row(r2); // wgpu (D3D/Metal, 0..1 Z)
        let far = Plane::from_row(r3 - r2);

        Frustum {
            planes: [left, right, bottom, top, near, far],
        }
    }
}

pub struct Frustum {
    pub planes: [Plane; 6],
}

impl From<Mat4> for Frustum {
    fn from(view_proj: Mat4) -> Self {
        let r0 = view_proj.row(0);
        let r1 = view_proj.row(1);
        let r2 = view_proj.row(2);
        let r3 = view_proj.row(3);

        let left = Plane::from_row(r3 + r0);
        let right = Plane::from_row(r3 - r0);
        let bottom = Plane::from_row(r3 + r1);
        let top = Plane::from_row(r3 - r1);
        let near = Plane::from_row(r2); // wgpu (D3D/Metal, 0..1 Z)
        let far = Plane::from_row(r3 - r2);

        Frustum {
            planes: [left, right, bottom, top, near, far],
        }
    }
}

impl Frustum {
    pub fn intersects_bounding_box(&self, b: &BoundingBox) -> bool {
        const EPS: f32 = 1e-5;
        for pl in &self.planes {
            let mask = pl.normal.cmplt(Vec3::ZERO);
            let p = Vec3::select(mask, b.min, b.max);
            if pl.signed_distance(p) < -EPS {
                return false;
            }
        }
        true
    }

    pub fn intersects_bounding_sphere(&self, bounding_sphere: &BoundingSphere) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.signed_distance(bounding_sphere.center) >= -bounding_sphere.radius)
    }
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn intersect_plane(&self, plane: &Plane) -> Option<Vec3> {
        let denom = self.direction.dot(plane.normal);

        // Check if the ray is parallel to the plane
        if denom.abs() < 1e-6 {
            return None;
        }

        let t = ((plane.normal * plane.distance) - self.origin).dot(plane.normal) / denom;

        // Check if the intersection is behind the ray's origin
        if t < 0.0 {
            return None;
        }

        // Compute the intersection point
        Some(self.origin + t * self.direction)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    fn from_row(row: Vec4) -> Self {
        let normal = row.truncate();
        let length = normal.length();

        if length <= f32::EPSILON || !length.is_infinite() {
            return Self {
                normal: Vec3::Z,
                distance: 0.0,
            };
        }

        let inv_length = 1.0 / length;
        Self {
            normal: normal / inv_length,
            distance: row.w / inv_length,
        }
    }

    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Debug, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn center(&self) -> Vec3 {
        self.min + (self.max - self.min)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn from_positions_ritter<I>(positions: I) -> Self
    where
        I: IntoIterator<Item = Vec3>,
    {
        let positions: Vec<Vec3> = positions.into_iter().collect();

        let positions_count = positions.len();

        if positions_count == 1 {
            return Self {
                center: positions[0],
                radius: 0.0,
            };
        }

        let p0 = *positions.iter().min_by(|a, b| a.x.total_cmp(&b.x)).unwrap();

        let p1 = *positions
            .iter()
            .max_by(|a, b| {
                let aa = (**a - p0).length_squared();
                let bb = (**b - p0).length_squared();
                aa.total_cmp(&bb)
            })
            .unwrap();

        let p2 = *positions
            .iter()
            .max_by(|a, b| {
                let aa = (**a - p1).length_squared();
                let bb = (**b - p1).length_squared();
                aa.total_cmp(&bb)
            })
            .unwrap();

        let mut center = (p1 + p2) * 0.5;
        let mut radius = (p2 - p1).length() * 0.5;

        for position in positions.iter() {
            let delta = position - center;
            let distance = delta.length();
            if distance > radius {
                let new_radius = 0.5 * (radius + distance);
                if distance > 0.0 {
                    center += delta * ((new_radius - radius) / distance);
                }
                radius = new_radius;
            }
        }

        Self { center, radius }
    }

    /// Minimal sphere that encloses self and other.
    pub fn union(&self, other: &BoundingSphere) -> BoundingSphere {
        let delta = other.center - self.center;
        let d = delta.length();

        // One contains the other, or coincident centers.
        if d <= (other.radius.max(0.0) - self.radius.max(0.0)).abs() {
            return if self.radius.max(0.0) >= other.radius.max(0.0) {
                *self
            } else {
                *other
            };
        }

        let new_radius = 0.5 * (d + self.radius.max(0.0) + other.radius.max(0.0));

        let t = if d > 0.0 {
            (new_radius - self.radius.max(0.0)) / d
        } else {
            0.0
        };
        let new_center = self.center + delta * t;

        BoundingSphere {
            center: new_center,
            radius: new_radius,
        }
    }

    pub fn expand_to_include(&mut self, other: &BoundingSphere) {
        *self = self.union(other);
    }
}
