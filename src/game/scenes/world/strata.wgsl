#import world::camera
#import world::environment
#import world::terrain
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var<storage> u_height_map: array<vec4<f32>>;
@group(2) @binding(1) var<uniform> u_terrain_data: terrain::TerrainData;
@group(2) @binding(2) var t_texture: texture_2d<f32>;
@group(2) @binding(3) var s_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) vertex_index: vec2<u32>,
    @location(4) chunk_pos: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_position: vec3<f32>,
}

fn get_height_map_index(chunk_pos: vec2<u32>, vertex_pos: vec2<u32>) -> u32 {
    let x = chunk_pos.x * terrain::CELLS_PER_CHUNK + vertex_pos.x;
    let y = chunk_pos.y * terrain::CELLS_PER_CHUNK + vertex_pos.y;
    let index = y * (u_terrain_data.size.x + 1) + x;

    return index;
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let bottom = -2000.0;

    let height_map_index = get_height_map_index(vertex.chunk_pos, vertex.vertex_index);
    let height = u_height_map[height_map_index].w;

    let chunk_start = vec2<f32>(
        f32(vertex.chunk_pos.x * terrain::CELLS_PER_CHUNK) * u_terrain_data.nominal_edge_size,
        f32(vertex.chunk_pos.y * terrain::CELLS_PER_CHUNK) * u_terrain_data.nominal_edge_size,
    );

    let world_position = vec3<f32>(
        chunk_start.x + vertex.position.x * u_terrain_data.nominal_edge_size,
        chunk_start.y + vertex.position.y * u_terrain_data.nominal_edge_size,
        bottom + vertex.position.z * (height - bottom),
    );
    let clip_position = u_camera.mat_projection * u_camera.mat_view * vec4(world_position, 1.0);

    let tex_coord = vec2<f32>(
        vertex.tex_coord.x,
        vertex.tex_coord.y * (height - bottom) / (u_terrain_data.nominal_edge_size * f32(terrain::CELLS_PER_CHUNK)),
    );

    return VertexOutput(
        clip_position,
        vertex.normal,
        tex_coord,
        world_position,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> geometry_buffers::OpaqueGeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    return geometry_buffers::to_opaque_geometry_buffer(
        diffuse,
        vertex.world_position,
        0xFFFFFFFF,
    );
}
