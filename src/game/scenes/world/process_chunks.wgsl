#import world::camera
#import world::frustum
#import world::terrain

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

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_terrain_data: terrain::TerrainData;
@group(1) @binding(1) var<storage, read> u_chunk_data: array<ChunkData>;
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

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let chunk_index = id.x;

    if (chunk_index >= arrayLength(&u_chunk_data)) {
        return;
    }

    let f = frustum::extract_frustum_planes(u_camera.mat_projection * u_camera.mat_view);

    let chunk = u_chunk_data[chunk_index];

    if frustum::is_aabb_in_frustum(f, chunk.min, chunk.max) {
        draw_terrain_chunk(chunk_index, 0u);
    } else {
        hide_terrain_chunk(chunk_index);
    }

    // If the water level is below the min of the chunk, then we don't even have to check the
    // frustum.
    if u_terrain_data.water_level < chunk.min.z {
        hide_water_chunk(chunk_index);
    } else {
        let water_min = vec3<f32>(chunk.min.xy, u_terrain_data.water_level);
        let water_max = vec3<f32>(chunk.max.xy, u_terrain_data.water_level);
        if frustum::is_aabb_in_frustum(f, water_min, water_max) {
            draw_water_chunk(chunk_index);
        } else {
            hide_water_chunk(chunk_index);
        }
    }
}
