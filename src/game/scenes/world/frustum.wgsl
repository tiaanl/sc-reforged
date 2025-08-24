#define_import_path world::frustum

struct Frustum {
    planes: array<vec4<f32>, 6>,
}

fn is_point_in_frustum(frustum: Frustum, point: vec3<f32>) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        let plane = frustum.planes[i];
        if (dot(plane.xyz, point) + plane.w < 0.0) {
            return false; // Point is outside this plane
        }
    }
    return true; // Point is inside all planes
}

fn is_sphere_in_frustum(frustum: Frustum, center: vec3<f32>, radius: f32) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        let plane = frustum.planes[i];
        if (dot(plane.xyz, center) + plane.w < -radius) {
            return false; // Sphere is completely outside this plane
        }
    }
    return true; // Sphere is inside or intersecting all planes
}

fn is_aabb_in_frustum(frustum: Frustum, min: vec3<f32>, max: vec3<f32>) -> bool {
    for (var i = 0; i < 6; i = i + 1) {
        let plane = frustum.planes[i];
        let positive_vertex = vec3<f32>(
            select(min.x, max.x, plane.x >= 0.0),
            select(min.y, max.y, plane.y >= 0.0),
            select(min.z, max.z, plane.z >= 0.0),
        );
        if (dot(plane.xyz, positive_vertex) + plane.w < 0.0) {
            return false; // AABB is completely outside this plane
        }
    }
    return true; // AABB is inside or intersecting all planes
}
