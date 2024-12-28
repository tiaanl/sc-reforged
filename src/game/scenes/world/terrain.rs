use std::path::PathBuf;

use glam::{IVec2, UVec2};
use tracing::info;
use wgpu::util::DeviceExt;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::Camera,
        config::{CampaignDef, TerrainMapping},
    },
};

use super::height_map::HeightMap;

pub struct Terrain {
    /// Height data for the terrain.
    height_map: HeightMap,

    /// Dictates the terrain resolution.
    pub max_view_distance: f32,

    /// Pipeline to render the terrain.
    pipeline: wgpu::RenderPipeline,

    /// Pipeline to render a wireframe over the terrain.
    wireframe_pipeline: wgpu::RenderPipeline,

    /// The texture used to render over the entire terrain.
    terrain_texture_bind_group: wgpu::BindGroup,

    /// A single buffer holding all the vertices for the terrain at the highest LOD level. Vertices
    /// are grouped by chunk, so that index (n * chunk_index_count) == chunk n.
    vertices_buffer: wgpu::Buffer,

    /// Holds indices for a chunk at all LOD levels. [level 0 indices, level 1 indices, ...].
    chunk_indices_buffer: wgpu::Buffer,

    /// The same indices as [chunk_indices_buffer], except for wireframes.
    chunk_wireframe_indices_buffer: wgpu::Buffer,

    /// Holds data for each chunk.
    chunk_data_buffer: wgpu::Buffer,

    /// Buffer holding indirect draw data for each chunk that needs rendering per frame.
    chunk_indirect_draw_calls_buffer: wgpu::Buffer,

    /// Total amount of chunks for the entire terrain.
    total_chunks: u32,

    draw_wireframe: bool,
    draw_normals: bool,
    lod_level: usize,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct ChunkData {
    min: Vec3,
    max: Vec3,
}

/// Fields copied from [wgpu::util::DrawIndexedIndirectArgs], but wgpu doesn't support bytemuck.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::NoUninit)]
struct ChunkDrawCall {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

impl Terrain {
    pub const LOD_MAX: u32 = 3;

    /// The amount of cells in a chunk. The example has 4 cells.
    ///
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    pub const CELLS_PER_CHUNK: u32 = 1 << Self::LOD_MAX;

    /// The amount of vertices in the chunk. Always 1 more than the amount of cells.
    pub const VERTICES_PER_CHUNK: u32 = Self::CELLS_PER_CHUNK + 1;

    pub fn new(
        assets: &AssetLoader,
        renderer: &Renderer,
        shaders: &mut Shaders,
        campaign_def: &CampaignDef,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        fog_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let TerrainMapping {
            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,
            texture_map_base_name,
            ..
        } = {
            let path = PathBuf::from("textures")
                .join("terrain")
                .join(&campaign_def.base_name)
                .join("terrain_mapping.txt");
            info!("Loading terrain mapping: {}", path.display());
            assets.load_config::<TerrainMapping>(&path)?
        };

        let terrain_texture_bind_group = {
            let path = format!("trnhigh/{}.jpg", texture_map_base_name);
            info!("Loading high detail terrain texture: {path}");

            let handle = assets.load_jpeg(path)?;
            let image = assets
                .asset_store()
                .get(handle)
                .expect("Just loaded successfully.");
            let texture_view = renderer.create_texture_view("terrain_texture", &image.data);

            renderer.create_texture_bind_group(
                "terrain_texture_bind_group",
                &texture_view,
                &renderer.create_sampler(
                    "terrain_sampler",
                    wgpu::AddressMode::ClampToEdge,
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Linear,
                ),
            )
        };

        let height_map = {
            use super::height_map::HeightMap;

            let path = format!("maps/{}.pcx", campaign_def.base_name); // TODO: Get the name of the map from the [CampaignDef].
            info!("Loading terrain height map: {path}");
            let data = assets.load_raw(path)?;
            let mut reader = std::io::Cursor::new(data);
            HeightMap::from_reader(nominal_edge_size, altitude_map_height_base, &mut reader)
                .map_err(|_| AssetError::Custom("Could not load height map data.".to_string()))?
        };

        let UVec2 {
            x: chunks_x,
            y: chunks_y,
        } = height_map.size / Terrain::CELLS_PER_CHUNK;
        let total_chunks = chunks_x * chunks_y;

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map.size.x, height_map.size.y,
        );

        let shader_module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
        );

        let pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("terrain", &shader_module)
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                })
                .bind_group_layout(renderer.texture_bind_group_layout())
                .bind_group_layout(camera_bind_group_layout)
                .bind_group_layout(fog_bind_group_layout),
        );

        let wireframe_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("terrain_wireframe", &shader_module)
                .vertex_entry("vertex_main")
                .fragment_entry("fragment_main_wireframe")
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                })
                .bind_group_layout(renderer.texture_bind_group_layout())
                .bind_group_layout(camera_bind_group_layout)
                .bind_group_layout(fog_bind_group_layout)
                .disable_depth_buffer(),
        );

        // let mut chunks_data = Vec::default();

        // Generate vertices for each chunk in sequence. [chunk 0, chunk 1, chunk 2, ...]
        // 81 vertices per chunk.

        // chunk sizes: 8 >> 4 >> 2 >> 1
        // 9 * 9 + 5 * 5 + 3 * 3 + 2 * 2 == 81 + 25 + 9 + 4 == 119
        let mut vertices = Vec::with_capacity(119 * total_chunks as usize);
        let mut chunk_data = Vec::with_capacity(total_chunks as usize);

        let size = Vec2::new(height_map.size.x as f32, height_map.size.y as f32);

        for y in 0..chunks_y {
            for x in 0..chunks_x {
                let offset = IVec2::new(
                    (Terrain::CELLS_PER_CHUNK * x) as i32,
                    (Terrain::CELLS_PER_CHUNK * y) as i32,
                );

                let mut min = Vec3::MAX;
                let mut max = Vec3::MIN;
                for y in offset.y..=offset.y + Self::CELLS_PER_CHUNK as i32 {
                    for x in offset.x..=offset.x + Self::CELLS_PER_CHUNK as i32 {
                        let position = height_map.position_for_vertex(IVec2::new(x, y));
                        vertices.push(Vertex {
                            position,
                            normal: Vec3::Z, // TODO: Get the normal from the height map.
                            tex_coord: Vec2::new(x as f32 / size.x, y as f32 / size.y),
                        });
                        min = min.min(position);
                        max = max.max(position);
                    }
                }

                chunk_data.push(ChunkData { min, max });
            }
        }

        let vertices_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let chunk_data_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_data"),
                    contents: bytemuck::cast_slice(&chunk_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        // Generate indices for each LOD level.

        // level 0 = 0..384
        // level 1 = 0..96
        // level 2 = 0..24
        // level 3 = 0..6
        let mut indices = Vec::<u32>::default();

        // level 0 = 0..512
        // level 1 = 0..128
        // level 2 = 0..32
        // level 3 = 0..8
        let mut wireframe_indices = Vec::<u32>::default();

        for level in 0..=Self::LOD_MAX {
            let cell_count = Self::CELLS_PER_CHUNK >> level;
            let scale = 1 << level;

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = (y * Self::VERTICES_PER_CHUNK + x) * scale;
                    let i1 = (y * Self::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i2 = ((y + 1) * Self::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i3 = ((y + 1) * Self::VERTICES_PER_CHUNK + x) * scale;

                    indices.extend_from_slice(&[i0, i1, i2, i2, i3, i0]);
                    wireframe_indices.extend_from_slice(&[i0, i1, i1, i2, i2, i3, i3, i0]);
                }
            }
        }

        let chunk_indices_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let chunk_wireframe_indices_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_writeframe_indices"),
                    contents: bytemuck::cast_slice(&wireframe_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        // Generate a command for each chunk.
        let chunk_indirect_commands = (0..total_chunks)
            .map(|index| ChunkDrawCall {
                first_index: 0,
                index_count: 384,
                base_vertex: index as i32 * 81,
                first_instance: 0,
                instance_count: 1,
            })
            .collect::<Vec<_>>();

        let chunk_indirect_draw_calls_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_indirect"),
                    contents: bytemuck::cast_slice(&chunk_indirect_commands),
                    usage: wgpu::BufferUsages::INDIRECT,
                });

        Ok(Self {
            height_map,

            max_view_distance: 13_300.0,

            pipeline,
            wireframe_pipeline,

            terrain_texture_bind_group,

            draw_wireframe: false,
            draw_normals: false,

            vertices_buffer,
            chunk_indices_buffer,
            chunk_wireframe_indices_buffer,
            chunk_data_buffer,
            chunk_indirect_draw_calls_buffer,
            total_chunks,

            lod_level: 0,
        })
    }

    pub fn update(&mut self, _camera: &Camera) {
        /*
        // Check if each terrain chunk is inside the cameras view frustum.
        let matrices = camera.calculate_matrices();
        let frustum = Frustum::from_matrices(&matrices);

        self.chunks.iter_mut().for_each(|chunk| {
            chunk.visible = frustum.contains_bounding_box(&chunk.bounding_box);
        });

        // Go through each terrain chunk and calculate its distance from the camera.
        for chunk in self.chunks.iter_mut().filter(|chunk| chunk.visible) {
            let distance = chunk.bounding_box.center().distance(camera.position);
            chunk.distance_from_camera = distance;

            let res = distance / (self.max_view_distance / Terrain::LOD_MAX as f32);
            let res = Terrain::LOD_MAX - (res as u32).min(Terrain::LOD_MAX);

            chunk.resolution = res;
        }
        */
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        fog_bind_group: &wgpu::BindGroup,
    ) {
        {
            let mut render_pass = frame.begin_basic_render_pass("terrain_render_pass", true);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            render_pass.set_index_buffer(
                self.chunk_indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, fog_bind_group, &[]);

            render_pass.multi_draw_indexed_indirect(
                &self.chunk_indirect_draw_calls_buffer,
                0,
                self.total_chunks,
            );

            /*
            let levels = [0..384, 384..480, 480..504, 504..510];
            for chunk_index in 0..self.total_chunks as i32 {
                render_pass.draw_indexed(
                    levels[self.lod_level].clone(),
                    chunk_index * (Self::VERTICES_PER_CHUNK * Self::VERTICES_PER_CHUNK) as i32,
                    0..1,
                );
            }
            */
        }

        if self.draw_wireframe {
            let mut render_pass =
                frame.begin_basic_render_pass("terrain_wireframe_render_pass", false);

            render_pass.set_pipeline(&self.wireframe_pipeline);
            render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            render_pass.set_index_buffer(
                self.chunk_wireframe_indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, fog_bind_group, &[]);

            let levels = [0..512, 512..640, 640..672, 672..680];
            for chunk_index in 0..self.total_chunks as i32 {
                render_pass.draw_indexed(
                    levels[self.lod_level].clone(),
                    chunk_index * (Self::VERTICES_PER_CHUNK * Self::VERTICES_PER_CHUNK) as i32,
                    0..1,
                );
            }
        }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        use egui::widgets::DragValue;
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");

        ui.add(DragValue::new(&mut self.max_view_distance).speed(10.0));

        for level in 0..=Self::LOD_MAX as usize {
            if ui
                .add(egui::widgets::RadioButton::new(
                    self.lod_level == level,
                    format!("level {level}"),
                ))
                .clicked()
            {
                self.lod_level = level;
            }
        }
    }
}

impl Terrain {
    /// Generate a list of vertices for a chunk at [offset].
    fn generate_chunk_vertices(height_map: &HeightMap, offset: IVec2) -> Vec<Vertex> {
        // chunk sizes: 8 >> 4 >> 2 >> 1
        // 9 * 9 + 5 * 5 + 3 * 3 + 2 * 2 == 81 + 25 + 9 + 4 == 119
        let mut vertices = Vec::with_capacity((0..=Self::LOD_MAX).fold(0, |c, i| {
            let v = (1 << i) + 1;
            c + v * v
        }));

        let size = Vec2::new(height_map.size.x as f32, height_map.size.y as f32);

        for y in offset.y..=offset.y + Self::CELLS_PER_CHUNK as i32 {
            for x in offset.x..=offset.x + Self::CELLS_PER_CHUNK as i32 {
                let position = height_map.position_for_vertex(IVec2::new(x, y));
                vertices.push(Vertex {
                    position,
                    normal: Vec3::Z, // TODO: Get the normal from the height map.
                    tex_coord: Vec2::new(x as f32 / size.x, y as f32 / size.y),
                });
            }
        }

        vertices
    }
}
