use std::path::PathBuf;

use glam::{IVec2, UVec2};
use tracing::info;
use wgpu::util::DeviceExt;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::{BoundingBox, Camera},
        config::{CampaignDef, TerrainMapping},
    },
};

use super::height_map::HeightMap;

pub struct Chunk {
    distance_from_camera: f32,
    resolution: u32,
    visible: bool,
    bounding_box: BoundingBox,
    meshes: [GpuChunkMesh; 4],
}

fn generate_chunk(height_map: &HeightMap, offset: IVec2, resolution: u32) -> ChunkMesh {
    let step = Terrain::CELLS_PER_CHUNK >> resolution;

    let cells = Terrain::CELLS_PER_CHUNK / step;
    let mut vertices = Vec::with_capacity(cells as usize * cells as usize);

    for y in (offset.y..=offset.y + Terrain::CELLS_PER_CHUNK as i32).step_by(step as usize) {
        for x in (offset.x..=offset.x + Terrain::CELLS_PER_CHUNK as i32).step_by(step as usize) {
            vertices.push(Vertex {
                position: height_map.position_for_vertex(IVec2 {
                    x: x as i32,
                    y: y as i32,
                }),
                normal: Vec3::Z,
                tex_coord: Vec2::new(
                    x as f32 / height_map.size.x as f32,
                    y as f32 / height_map.size.y as f32,
                ),
            });
        }
    }

    let mut indices = Vec::with_capacity(cells as usize * cells as usize * 3);
    let mut wireframe_indices = Vec::with_capacity(cells as usize * cells as usize * 8);
    for y in 0..cells {
        for x in 0..cells {
            let f0 = y * (cells + 1) + x;
            let f1 = f0 + 1;
            let f3 = f1 + cells;
            let f2 = f3 + 1;

            // 2 tringles for the face.
            indices.extend_from_slice(&[
                f0, f1, f2, // 0
                f2, f3, f0, // 1
            ]);

            // 4 lines for the wireframe.
            wireframe_indices.extend_from_slice(&[
                f0, f1, // 0
                f1, f2, // 1
                f2, f3, // 2
                f3, f0, // 3
            ]);
        }
    }

    ChunkMesh {
        vertices,
        indices,
        wireframe_indices,
    }
}

impl Chunk {
    pub fn new(height_map: &HeightMap, chunk_offset: IVec2, renderer: &Renderer) -> Chunk {
        let chunk_size = Terrain::CELLS_PER_CHUNK as i32;
        let min = chunk_offset * chunk_size;
        let max = min + IVec2::new(chunk_size + 1, chunk_size + 1);

        Self {
            distance_from_camera: f32::MAX,
            resolution: Terrain::LOD_MAX,
            visible: true,
            bounding_box: BoundingBox {
                min: height_map.position_for_vertex(IVec2 {
                    x: min.x as i32,
                    y: min.y as i32,
                }),
                max: height_map.position_for_vertex(IVec2 {
                    x: max.x as i32,
                    y: max.y as i32,
                }),
            },
            meshes: [0, 1, 2, 3]
                .map(|res| generate_chunk(&height_map, min, res).into_gpu(renderer)),
        }
    }

    pub fn mesh_at(&self, resolution: u32) -> &GpuChunkMesh {
        assert!(resolution <= Terrain::LOD_MAX);
        &self.meshes[resolution as usize]
    }
}

pub struct ChunkMesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    wireframe_indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn into_gpu(self, renderer: &Renderer) -> GpuChunkMesh {
        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_chunk_vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_count = self.indices.len() as u32;
        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_chunk_index_buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let wireframe_index_count = self.wireframe_indices.len() as u32;
        let wireframe_index_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_wireframe_index_buffer"),
                    contents: bytemuck::cast_slice(&self.wireframe_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        GpuChunkMesh {
            vertex_buffer,
            index_buffer,
            index_count,
            wireframe_index_buffer,
            wireframe_index_count,
        }
    }
}

pub struct GpuChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub wireframe_index_buffer: wgpu::Buffer,
    pub wireframe_index_count: u32,
}

struct StrataChunk {
    gpu_mesh: GpuIndexedMesh,
}

pub struct Terrain {
    /// Height data for the terrain.
    height_map: HeightMap,

    /// Dictates the terrain resolution.
    pub max_view_distance: f32,

    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,

    terrain_texture_bind_group: wgpu::BindGroup,

    draw_wireframe: bool,
    draw_normals: bool,

    /// A single buffer holding all the vertices for the terrain at the highest LOD level. Vertices
    /// are grouped by chunk, so that index (n * chunk_index_count) == chunk n.
    vertices_buffer: wgpu::Buffer,

    /// Holds indices for a chunk at all LOD levels. [level 0 indices, level 1 indices, ...].
    chunk_indices_buffer: wgpu::Buffer,

    /// The same indices as [chunk_indices_buffer], except for wireframes.
    chunk_wireframe_indices_buffer: wgpu::Buffer,

    /// Total amount of chunks for the entire terrain.
    total_chunks: u32,

    lod_level: usize,
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

        // Generate vertices for each chunk in sequence. [chunk 0, chunk 1, chunk 2, ...]
        // 81 vertices per chunk.
        let vertices_buffer = {
            let mut vertices = vec![];
            for y in 0..chunks_y {
                for x in 0..chunks_x {
                    vertices.extend(Self::generate_chunk_vertices(
                        &height_map,
                        IVec2::new(
                            (Terrain::CELLS_PER_CHUNK * x) as i32,
                            (Terrain::CELLS_PER_CHUNK * y) as i32,
                        ),
                    ));
                }
            }

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        };

        // Generate indices for each LOD level.
        // level 0 = 0..384
        // level 1 = 0..96
        // level 2 = 0..24
        // level 3 = 0..6
        let chunk_indices_buffer = {
            let mut indices = Vec::<u32>::default();

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
                    }
                }
            }

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                })
        };

        let chunk_wireframe_indices_buffer = {
            let mut indices = Vec::<u32>::default();

            for level in 0..=Self::LOD_MAX {
                let cell_count = Self::CELLS_PER_CHUNK >> level;
                let scale = 1 << level;

                for y in 0..cell_count {
                    for x in 0..cell_count {
                        let i0 = (y * Self::VERTICES_PER_CHUNK + x) * scale;
                        let i1 = (y * Self::VERTICES_PER_CHUNK + (x + 1)) * scale;
                        let i2 = ((y + 1) * Self::VERTICES_PER_CHUNK + (x + 1)) * scale;
                        let i3 = ((y + 1) * Self::VERTICES_PER_CHUNK + x) * scale;

                        indices.extend_from_slice(&[i0, i1, i1, i2, i2, i3, i3, i0]);
                    }
                }
                println!("size: {}", indices.len());
            }

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_writeframe_indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                })
        };

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
            total_chunks: chunks_x * chunks_y,

            lod_level: 0,
        })
    }

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

            let levels = [0..384, 384..480, 480..504, 504..510];
            for chunk_index in 0..self.total_chunks as i32 {
                render_pass.draw_indexed(
                    levels[self.lod_level].clone(),
                    chunk_index * (Self::VERTICES_PER_CHUNK * Self::VERTICES_PER_CHUNK) as i32,
                    0..1,
                );
            }
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

        for i in 0..4 {
            if ui
                .add(egui::widgets::RadioButton::new(
                    self.lod_level == i,
                    format!("level {i}"),
                ))
                .clicked()
            {
                self.lod_level = i;
            }
        }

        // egui::Grid::new("terrain_data").show(ui, |ui| {
        //     ui.label("terrain mapping size");
        //     ui.label(format!("{} x {}", self.map_dx, self.map_dy));
        //     ui.end_row();

        //     ui.label("nominal edge size");
        //     ui.label(format!("{}", self.nominal_edge_size));
        //     ui.end_row();

        //     ui.label("altitude map height base");
        //     ui.label(format!("{}", self.altitude_map_height_base));
        // });
    }
}
