#import world::camera::Camera;
#import terrain::{CELLS_PER_CHUNK, ChunkInstance, get_node};

@group(0) @binding(0) var<uniform> u_camera: Camera;

@group(1) @binding(0) var<storage, read> u_height_map: array<vec4<f32>>;
@group(1) @binding(1) var<storage, read> u_chunk_instances: array<ChunkInstance>;
@group(1) @binding(2) var<uniform> u_terrain_data: terrain::TerrainData;
@group(1) @binding(3) var t_terrain_texture: texture_2d<f32>;
@group(1) @binding(4) var t_water_texture: texture_2d<f32>;
@group(1) @binding(5) var s_sampler: sampler;

fn get_node_world_position(node: terrain::Node, lod: u32) -> vec3<f32> {
    return vec3<f32>(
        f32(node.x) * u_terrain_data.nominal_edge_size,
        f32(node.y) * u_terrain_data.nominal_edge_size,
        u_height_map[node.index].w,
    );
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex_wireframe(
    @builtin(instance_index) chunk_index: u32,
    @builtin(vertex_index) node_index: u32,
) -> VertexOutput {
    let node = get_node(u_terrain_data, chunk_index, node_index);
    let world_position = get_node_world_position(node, 0);

    let clip_position = u_camera.mat_proj_view * vec4<f32>(world_position, 1.0);

    return VertexOutput(clip_position, vec4<f32>(0.0, 1.0, 0.0, 1.0));
}

@fragment
fn fragment_wireframe(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vertex.color;
}
