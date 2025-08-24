#import world::camera
#import world::frustum

struct ChunkInstance {
    center: vec3<f32>,
    radius: f32,

    chunk: vec2<i32>,
    min_elevation: f32,
    max_elevation: f32,

    lod_index: u32,
    flags: u32,
}

struct DrawArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

struct LodRange {
    first_index: u32,
    index_count: u32,
}

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var<uniform> u_terrain_data: terrain::TerrainData;
@group(2) @binding(1) var<storage, read_write> u_chunk_instances: array<ChunkInstance>;
@group(2) @binding(2) var<storage, read_write> u_terrain_draw_args: array<DrawArgs>;
@group(2) @binding(3) var<storage, read_write> u_water_draw_args: array<DrawArgs>;
@group(2) @binding(4) var<storage, read_write> u_wireframe_draw_args: array<DrawArgs>;


const MAX_LOD_LEVELS: u32 = 4u;
const ZERO_DRAW_ARGS: DrawArgs = DrawArgs(0u, 0u, 0u, 0, 0u);

const TERRAIN_LODS: array<LodRange, 4> = array<LodRange, 4>(
    LodRange(0u, 384u),
    LodRange(384u, 96u),
    LodRange(480u, 24u),
    LodRange(504u, 6u),
);

const WIREFRAME_LODS: array<LodRange, 4> = array<LodRange, 4>(
    LodRange(0u, 512u),
    LodRange(512u, 128u),
    LodRange(640u, 32u),
    LodRange(672u, 8u),
);

fn make_draw_args(range: LodRange, chunk_index: u32) -> DrawArgs {
    return DrawArgs(range.index_count, 1u, range.first_index, 0, chunk_index);
}

fn compute_lod(center_world: vec3<f32>) -> u32 {
    let far = max(u_environment.fog_params.y, 1e-6);
    let inv_step = f32(MAX_LOD_LEVELS) / far;
    let forward = camera::camera_forward(u_camera);

    let forward_distance = max(0.0, dot(center_world - u_camera.position, forward));
    let t = forward_distance * inv_step;
    return u32(clamp(i32(floor(t)), 0, i32(MAX_LOD_LEVELS - 1u)));
}

fn write_terrain_and_wireframe(chunk_index: u32, lod: u32) {
    let terrain_lod = TERRAIN_LODS[lod];
    let wireframe_lod = WIREFRAME_LODS[lod];

    u_terrain_draw_args[chunk_index]   = make_draw_args(terrain_lod, chunk_index);
    u_wireframe_draw_args[chunk_index] = make_draw_args(wireframe_lod, chunk_index);
}

fn hide_terrain_and_wireframe(chunk_index: u32) {
    u_terrain_draw_args[chunk_index]   = ZERO_DRAW_ARGS;
    u_wireframe_draw_args[chunk_index] = ZERO_DRAW_ARGS;
}

fn write_water(chunk_index: u32, lod: u32) {
    let terrain_lod = TERRAIN_LODS[lod];

    u_water_draw_args[chunk_index] = make_draw_args(terrain_lod, chunk_index);
}

fn hide_water(chunk_index: u32) {
    u_water_draw_args[chunk_index] = ZERO_DRAW_ARGS;
}

// ---------- Main ----------
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let chunk_index = gid.x;

    if (chunk_index >= arrayLength(&u_chunk_instances)) {
        return;
    }

    let world_frustum = frustum::Frustum(u_camera.frustum);
    let chunk: ChunkInstance = u_chunk_instances[chunk_index];

    // Compute and store LOD
    let lod = compute_lod(chunk.center);
    u_chunk_instances[chunk_index].lod_index = lod;

    // Terrain visibility
    let terrain_visible = frustum::is_sphere_in_frustum(
        world_frustum,
        chunk.center,
        chunk.radius,
    );
    if (terrain_visible) {
        write_terrain_and_wireframe(chunk_index, lod);
    } else {
        hide_terrain_and_wireframe(chunk_index);
    }

    // Water visibility (skip if never visible due to height)
    if (u_terrain_data.water_level < chunk.min_elevation) {
        hide_water(chunk_index);
        return;
    }

    let half_chunk = u_terrain_data.nominal_edge_size * f32(terrain::CELLS_PER_CHUNK) * 0.5;
    let water_center = vec3<f32>(chunk.center.xy, u_terrain_data.water_level);
    let water_radius = half_chunk * sqrt(2.0);

    if (frustum::is_sphere_in_frustum(world_frustum, water_center, water_radius)) {
        write_water(chunk_index, lod);
    } else {
        hide_water(chunk_index);
    }
}
