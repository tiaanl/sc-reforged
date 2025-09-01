#define_import_path math

const EPSILON: f32 = 1e-4;

fn normalize_safe(v: vec3<f32>) -> vec3<f32> {
    let len2 = dot(v, v);
    if (len2 > EPSILON) { return v / sqrt(len2); }
    return v;
}

fn quat_to_mat3(q: vec4<f32>) -> mat3x3<f32> {
    let x2 = q.x + q.x;
    let y2 = q.y + q.y;
    let z2 = q.z + q.z;

    let xx2 = q.x * x2;
    let yy2 = q.y * y2;
    let zz2 = q.z * z2;
    let xy2 = q.x * y2;
    let xz2 = q.x * z2;
    let yz2 = q.y * z2;
    let wx2 = q.w * x2;
    let wy2 = q.w * y2;
    let wz2 = q.w * z2;

    return mat3x3<f32>(
        vec3<f32>(1.0 - (yy2 + zz2), xy2 + wz2,        xz2 - wy2),
        vec3<f32>(xy2 - wz2,        1.0 - (xx2 + zz2), yz2 + wx2),
        vec3<f32>(xz2 + wy2,        yz2 - wx2,        1.0 - (xx2 + yy2)),
    );
}

fn transform_from_position_and_rotation(position: vec3<f32>, rotation: vec4<f32>) -> mat4x4<f32> {
    let rot = quat_to_mat3(rotation);
    return mat4x4<f32>(
        vec4<f32>(rot[0], 0.0),
        vec4<f32>(rot[1], 0.0),
        vec4<f32>(rot[2], 0.0),
        vec4<f32>(position, 1.0),
    );
}

fn position_in_frustum(view_projection: mat4x4<f32>, position: vec3<f32>) -> bool {
    let projected = view_projection * vec4<f32>(position, 1.0);

    // Must be in front of the camera.
    if (projected.w <= 0.0) {
        return false;
    }

    let ndc = projected.xyz / projected.w;

    let inside_xy =
        all(ndc.xy >= vec2<f32>(-1.0 - EPSILON)) &&
        all(ndc.xy <= vec2<f32>(1.0 + EPSILON));
    if !inside_xy {
        return false;
    }

    let inside_z = (ndc.z >= 0.0 - EPSILON) && (ndc.z <= 1.0 + EPSILON);
    if !inside_z {
        return false;
    }

    return true;
}

