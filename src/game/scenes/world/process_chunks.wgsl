// TODO: Use this from imports.
struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
    frustum: array<vec4<f32>, 6>,
}

struct ChunkData {
    min: vec3<f32>,
    _padding1: f32,
    max: vec3<f32>,
    _padding2: f32,
};

struct DrawArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

@group(0) @binding(0) var<uniform> u_camera: Camera;

@group(1) @binding(0) var<storage, read> u_chunk_data: array<ChunkData>;
@group(1) @binding(1) var<storage, read_write> u_draw_commands: array<DrawArgs>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let chunk_index = id.x;

    if (chunk_index >= arrayLength(&u_chunk_data)) {
        return;
    }

    let chunk = u_chunk_data[chunk_index];
    var visible = true;

    // AABB-Frustum culling
    for (var i = 0u; i < 6u; i++) {
        let plane = u_camera.frustum[i];

        // Calculate the "positive vertex" of the AABB for this plane
        let positive_vertex = vec3<f32>(
            select(chunk.min.x, chunk.max.x, plane.x > 0.0),
            select(chunk.min.y, chunk.max.y, plane.y > 0.0),
            select(chunk.min.z, chunk.max.z, plane.z > 0.0),
        );

        // If the positive vertex is outside the plane, the AABB is culled
        if dot(plane.xyz, positive_vertex) + plane.w < 0.0 {
            visible = false;
            break;
        }
    }

    // Populate indirect buffer if visible
    if (visible) {
        let lod_index_start = u32(0);
        let lod_index_count = u32(384);
        let base_vertex = i32(chunk_index * u32(81));
        u_draw_commands[chunk_index] = DrawArgs(
            lod_index_count, // index_count
            1, // instance_count,
            lod_index_start, // first_index,
            base_vertex,
            0, // first_instance
        );
    } else {
        u_draw_commands[chunk_index] = DrawArgs(0, 0, 0, 0, 0);
    }
}
