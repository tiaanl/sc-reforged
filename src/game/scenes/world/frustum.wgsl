#define_import_path world::frustum

struct Frustum {
    planes: array<vec4<f32>, 6>,
}

fn extract_frustum_planes(clip: mat4x4<f32>) -> Frustum {
    let row1 = vec4<f32>(clip[0][0], clip[1][0], clip[2][0], clip[3][0]);
    let row2 = vec4<f32>(clip[0][1], clip[1][1], clip[2][1], clip[3][1]);
    let row3 = vec4<f32>(clip[0][2], clip[1][2], clip[2][2], clip[3][2]);
    let row4 = vec4<f32>(clip[0][3], clip[1][3], clip[2][3], clip[3][3]);

    let planes = array<vec4<f32>, 6>(
        normalize(row4 + row1), // left
        normalize(row4 - row1), // right
        normalize(row4 + row2), // bottom
        normalize(row4 - row2), // top
        normalize(row4 + row3), // near
        normalize(row4 - row3)  // far
    );

    return Frustum(planes);
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
