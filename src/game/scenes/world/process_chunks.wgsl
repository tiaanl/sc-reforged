#import world::camera
#import world::frustum

struct ChunkInstance {
    center: vec3<f32>,
    radius: f32,

    chunk: vec2<i32>,
    min_elevation: f32,
    max_elevation: f32,

    flags: u32,
}

struct DrawArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_terrain_data: terrain::TerrainData;
@group(1) @binding(1) var<storage, read> u_chunk_instances: array<ChunkInstance>;
@group(1) @binding(2) var<storage, read_write> u_terrain_draw_args: array<DrawArgs>;
@group(1) @binding(3) var<storage, read_write> u_water_draw_args: array<DrawArgs>;

const LEVELS: array<vec2<u32>, 4> = array<vec2<u32>, 4>(
    vec2<u32>(0u, 384u),
    vec2<u32>(384u, 96u),
    vec2<u32>(480u, 24u),
    vec2<u32>(504u, 6u),
);

fn draw_terrain_chunk(chunk_index: u32, level: u32) {
    let lod_data = LEVELS[level];

    u_terrain_draw_args[chunk_index] = DrawArgs(
        lod_data.y,     // index_count
        1,              // instance_count,
        lod_data.x,     // first_index,
        0,              // vertex_base
        chunk_index,    // first_instance - Use the first instance as the chunk index.
    );
}

fn draw_water_chunk(chunk_index: u32) {
    // Water chunks are always rendered at full LOD because of the waves and vertex based blending
    // algorithm.
    let lod_data = LEVELS[0u];

    u_water_draw_args[chunk_index] = DrawArgs(
        lod_data.y,     // index_count
        1,              // instance_count,
        lod_data.x,     // first_index,
        0,              // vertex_base
        chunk_index,    // first_instance - Use the first instance as the chunk index.
    );
}

fn hide_terrain_chunk(chunk_index: u32) {
    u_terrain_draw_args[chunk_index] = DrawArgs(0, 0, 0, 0, 0);
}

fn hide_water_chunk(chunk_index: u32) {
    u_water_draw_args[chunk_index] = DrawArgs(0, 0, 0, 0, 0);
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let chunk_index = id.x;

    if (chunk_index >= arrayLength(&u_chunk_instances)) {
        return;
    }

    let f = frustum::Frustum(u_camera.frustum);

    let chunk_instance: ChunkInstance = u_chunk_instances[chunk_index];

    let visible = frustum::is_sphere_in_frustum(f, chunk_instance.center, chunk_instance.radius);
    if visible {
        draw_terrain_chunk(chunk_index, 0u);
    } else {
        hide_terrain_chunk(chunk_index);
    }

    // If the water level is below the minimum elevation of the chunk, then it will never be
    // visible.
    if u_terrain_data.water_level < chunk_instance.min_elevation {
        hide_water_chunk(chunk_index);
    } else {
        let water_center = vec3<f32>(chunk_instance.center.xy, u_terrain_data.water_level);
        let half_chunk_size = u_terrain_data.nominal_edge_size * f32(terrain::CELLS_PER_CHUNK) * 0.5;
        let radius = half_chunk_size * sqrt(2.0);

        if frustum::is_sphere_in_frustum(f, water_center, radius) {
            draw_water_chunk(chunk_index);
        } else {
            hide_water_chunk(chunk_index);
        }
    }
}
