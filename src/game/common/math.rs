#![allow(dead_code)]

use glam::{Mat4, Vec3, Vec4};

/// Represents a view into the world by way of matrices.
#[derive(Clone, Debug, Default)]
pub struct ViewProjection {
    /// Combined projection * view matrix.
    pub mat: Mat4,
    /// Inverse of the projection * view matrix.
    pub inv: Mat4,
}

impl ViewProjection {
    pub fn from_projection_view(projection: Mat4, view: Mat4) -> Self {
        let mat = projection * view;
        let inv = mat.inverse();

        Self { mat, inv }
    }

    #[inline]
    pub fn unproject_ndc(&self, point: Vec3) -> Vec3 {
        self.inv.project_point3(point)
    }

    pub fn corners(&self) -> [Vec3; 8] {
        const NDC: &[(f32, f32)] = &[(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

        let mut result = [Vec3::ZERO; 8];
        for (i, &(x, y)) in NDC.iter().enumerate() {
            result[i] = self.unproject_ndc(Vec3::new(x, y, 0.0)); // near
            result[i + 4] = self.unproject_ndc(Vec3::new(x, y, 1.0)); // far
        }
        result
    }

    pub fn frustum(&self) -> Frustum {
        let r0 = self.mat.row(0);
        let r1 = self.mat.row(1);
        let r2 = self.mat.row(2);
        let r3 = self.mat.row(3);

        let left = Plane::from_row(r3 + r0);
        let right = Plane::from_row(r3 - r0);
        let bottom = Plane::from_row(r3 + r1);
        let top = Plane::from_row(r3 - r1);
        let near = Plane::from_row(r2); // wgpu (D3D/Metal, 0..1 Z)
        let far = Plane::from_row(r3 - r2);

        Frustum::from_planes(left, right, bottom, top, near, far)
    }
}

#[derive(Default)]
pub struct Frustum {
    /// [left, right, bottom, top, near, far]
    pub planes: [Plane; 6],
}

impl Frustum {
    pub fn from_planes(
        left: Plane,
        right: Plane,
        bottom: Plane,
        top: Plane,
        near: Plane,
        far: Plane,
    ) -> Self {
        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    /// Construct a frustum from 8 corner points.
    #[allow(clippy::too_many_arguments)]
    pub fn from_corners(
        near_top_left: Vec3,
        near_top_right: Vec3,
        near_bottom_right: Vec3,
        near_bottom_left: Vec3,
        far_top_left: Vec3,
        far_top_right: Vec3,
        far_bottom_right: Vec3,
        far_bottom_left: Vec3,
    ) -> Self {
        let left = Plane::from_points(near_top_left, far_top_left, far_bottom_left);
        let right = Plane::from_points(far_bottom_right, far_top_right, near_top_right);
        let bottom = Plane::from_points(near_bottom_left, far_bottom_left, far_bottom_right);
        let top = Plane::from_points(far_top_right, far_top_left, near_top_left);
        let near = Plane::from_points(near_bottom_left, near_bottom_right, near_top_right);
        let far = Plane::from_points(far_top_right, far_bottom_right, far_bottom_left);

        #[cfg(debug_assertions)]
        {
            let center = (near_top_left
                + near_top_right
                + near_bottom_right
                + near_bottom_left
                + far_top_left
                + far_top_right
                + far_bottom_right
                + far_bottom_left)
                / 8.0;
            for plane in [&left, &right, &top, &bottom, &near, &far] {
                let distance = plane.signed_distance(center);
                debug_assert!(distance >= 0.0, "outward frustum plane ({distance})");
            }
        }

        Frustum::from_planes(left, right, bottom, top, near, far)
    }

    /// Calculate the 8 corners of the frustum.
    pub fn corners(&self) -> Option<[Vec3; 8]> {
        let [left, right, bottom, top, near, far] = &self.planes;

        /// Calculate the intersection of 3 planes.
        fn intersect(a: &Plane, b: &Plane, c: &Plane) -> Option<Vec3> {
            let n1 = a.normal;
            let n2 = b.normal;
            let n3 = c.normal;

            let n2n3 = n2.cross(n3);
            let n3n1 = n3.cross(n1);
            let n1n2 = n1.cross(n2);

            let denom = n1.dot(n2n3);
            if denom.abs() < 1e-6 {
                // Planes are nearly parallel / degenerate
                return None;
            }

            let term1 = n2n3 * -a.distance;
            let term2 = n3n1 * -b.distance;
            let term3 = n1n2 * -c.distance;

            Some((term1 + term2 + term3) / denom)
        }

        Some([
            // near plane
            intersect(near, top, left)?,     // ntl
            intersect(near, top, right)?,    // ntr
            intersect(near, bottom, right)?, // nbr
            intersect(near, bottom, left)?,  // nbl
            // far plane
            intersect(far, top, left)?,     // ftl
            intersect(far, top, right)?,    // ftr
            intersect(far, bottom, right)?, // fbr
            intersect(far, bottom, left)?,  // fbl
        ])
    }

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
    pub fn point_at(&self, distance: f32) -> Vec3 {
        debug_assert!(self.direction.is_normalized());

        self.origin + self.direction * distance
    }

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

pub struct RaySegment {
    pub ray: Ray,
    pub distance: f32,
}

impl RaySegment {
    #[inline]
    pub fn is_degenerate(&self) -> bool {
        self.distance <= 0.0 || self.ray.direction.length_squared() == 0.0
    }

    #[inline]
    pub fn t_max(&self) -> f32 {
        let len = self.ray.direction.length();
        if len == 0.0 { 0.0 } else { self.distance / len }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    fn from_row(row: Vec4) -> Self {
        let normal = row.truncate();
        let length = normal.length();

        if length <= f32::EPSILON || !length.is_finite() {
            return Self {
                normal: Vec3::Z,
                distance: 0.0,
            };
        }

        let inv_length = 1.0 / length;
        Self {
            normal: normal * inv_length,
            distance: row.w * inv_length,
        }
    }

    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let ab = b - a;
        let ac = c - a;

        let normal = ab.cross(ac).normalize();

        let distance = -normal.dot(a);

        Self { normal, distance }
    }

    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn center(&self) -> Vec3 {
        self.min + (self.max - self.min)
    }

    pub fn fully_contains_sphere(&self, sphere: &BoundingSphere) -> bool {
        let center = sphere.center;
        let radius = sphere.radius;

        center.x - radius >= self.min.x
            && center.x + radius <= self.max.x
            && center.y - radius >= self.min.y
            && center.y + radius <= self.max.y
            && center.z - radius >= self.min.z
            && center.z + radius <= self.max.z
    }

    /// Return (t_enter, t_exit, enter_normal)
    pub fn intersect_ray_segment(&self, ray_segment: &RaySegment) -> Option<(f32, f32, Vec3)> {
        const EPSILON: f32 = 1e-8;

        if ray_segment.is_degenerate() {
            return None;
        }

        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;
        let mut enter_normal = Vec3::ZERO;

        for axis in 0..3 {
            let origin = ray_segment.ray.origin[axis];
            let direction = ray_segment.ray.direction[axis];
            let min_a = self.min[axis];
            let max_a = self.max[axis];

            if direction.abs() < EPSILON {
                // Parallel to slab; must be inside.
                if origin < min_a || origin > max_a {
                    return None;
                }
                continue;
            }

            let mut t1 = (min_a - origin) / direction;
            let mut t2 = (max_a - origin) / direction;
            let mut normal = Vec3::ZERO;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                normal[axis] = 1.0; // Entering from max-face.
            } else {
                normal[axis] = -1.0; // Entering from min-face.
            }

            if t1 > t_min {
                t_min = t1;
                enter_normal = normal;
            }
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }

        // Clip to segment range.
        let t_lo = 0.0;
        let t_hi = ray_segment.t_max();
        if t_max < t_lo || t_min > t_hi {
            return None;
        }

        let t_enter = t_min.max(t_lo);
        let t_exit = t_max.min(t_hi);

        Some((t_enter, t_exit, enter_normal))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub const ZERO: BoundingSphere = BoundingSphere {
        center: Vec3::ZERO,
        radius: 0.0,
    };

    #[inline]
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

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

    /// Returns (t, enter_normal).
    pub fn intersect_ray_segment(&self, ray_segment: &RaySegment) -> Option<(f32, Vec3)> {
        if ray_segment.is_degenerate() {
            return None;
        }

        let m = ray_segment.ray.origin - self.center;
        let a = ray_segment.ray.direction.length_squared();
        let b = 2.0 * m.dot(ray_segment.ray.direction);
        let c = m.length_squared() - self.radius * self.radius;

        // Origin inside sphere → treat t=0 as hit.
        if c <= 0.0 {
            let normal = (ray_segment.ray.origin - self.center).normalize_or_zero();
            return Some((0.0, normal));
        }

        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sqrt_disc = disc.sqrt();
        let inv_2a = 0.5 / a;

        let t1 = (-b - sqrt_disc) * inv_2a;
        let t2 = (-b + sqrt_disc) * inv_2a;

        let t_max = ray_segment.t_max();
        let t_hit = if (0.0..=t_max).contains(&t1) {
            t1
        } else if (0.0..=t_max).contains(&t2) {
            t2
        } else {
            return None;
        };

        let p = ray_segment.ray.origin + ray_segment.ray.direction * t_hit;
        let n = (p - self.center).normalize_or_zero();

        Some((t_hit, n))
    }
}

pub struct RayTriangleHit {
    /// Parameter along the ray: hit_point = origin + direction * t
    pub t: f32,
    /// World position of the intersection.
    pub world_position: Vec3,
    /// (w,u,v) where w = 1 - u - v
    pub barycentric: Vec3,
    /// Geometric triangle normal (right-hand cross of edges). Handedness of your world
    /// doesn't break the math; choose vertex winding to control this direction.
    pub normal: Vec3,
    /// True if the ray hits the side the normal points away from (normal·dir < 0).
    pub front_face: bool,
}

/// Ray/triangle intersection (Möller–Trumbore).
/// `cull_backfaces = true` rejects hits from the back side.
/// Returns `t`, hit position, barycentrics, and a geometric normal.
pub fn triangle_intersect_ray(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    ray: &Ray,
    cull_backfaces: bool,
) -> Option<RayTriangleHit> {
    // Edges of the triangle
    let edge_0_1 = v1 - v0;
    let edge_0_2 = v2 - v0;

    // Begin calculating determinant — also used to calculate U parameter.
    let perpendicular_to_direction_and_edge_0_2 = ray.direction.cross(edge_0_2);
    let determinant = edge_0_1.dot(perpendicular_to_direction_and_edge_0_2);

    // Epsilon tuned for f32; adjust if your scale is extreme.
    const EPS: f32 = 1e-7;

    if cull_backfaces {
        if determinant <= EPS {
            return None;
        }
    } else if determinant.abs() <= EPS {
        // Parallel or degenerate triangle.
        return None;
    }

    let inverse_determinant = 1.0 / determinant;

    // Calculate distance from vertex0 to ray origin
    let origin_to_vertex0 = ray.origin - v0;

    // Calculate U parameter and test bounds
    let u = origin_to_vertex0.dot(perpendicular_to_direction_and_edge_0_2) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    // Prepare to test V parameter
    let cross_origin_to_vertex0_and_edge_0_1 = origin_to_vertex0.cross(edge_0_1);

    // Calculate V parameter and test bounds
    let v = ray.direction.dot(cross_origin_to_vertex0_and_edge_0_1) * inverse_determinant;
    if v < 0.0 || (u + v) > 1.0 {
        return None;
    }

    // Calculate t to intersection point
    let t = edge_0_2.dot(cross_origin_to_vertex0_and_edge_0_1) * inverse_determinant;
    if t < 0.0 {
        return None; // Behind the ray origin
    }

    let world_position = ray.origin + ray.direction * t;

    // Geometric normal (right-hand cross). If your triangle winding follows your LH world,
    // you may want to swap the order to flip the normal.
    let normal = edge_0_1.cross(edge_0_2).normalize_or_zero();

    let front_face = normal.dot(ray.direction) < 0.0;
    let w = 1.0 - u - v;

    Some(RayTriangleHit {
        t,
        world_position,
        barycentric: Vec3::new(w, u, v),
        normal,
        front_face,
    })
}

/// Ray *segment*/triangle intersection. Accepts the hit only if `t ∈ [0, segment.distance]`.
#[inline]
pub fn triangle_intersect_ray_segment(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    ray_segment: &RaySegment,
    cull_backfaces: bool,
) -> Option<RayTriangleHit> {
    let hit = triangle_intersect_ray(v0, v1, v2, &ray_segment.ray, cull_backfaces)?;
    if hit.t <= ray_segment.distance {
        Some(hit)
    } else {
        None
    }
}
