use std::{borrow::Cow, path::PathBuf};

use glam::{IVec2, UVec2};
use tracing::info;
use wgpu::util::DeviceExt;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::Camera,
        compositor::Compositor,
        config::{CampaignDef, TerrainMapping},
    },
};

use super::{height_map::HeightMap, water::Water};

pub struct Terrain {
    /// Height data for the terrain.
    height_map: HeightMap,

    /// Dictates the terrain resolution.
    pub max_view_distance: f32,

    /// Pipeline to render the terrain.
    pipeline: wgpu::RenderPipeline,

    /// Pipeline to render a wireframe over the terrain.
    wireframe_pipeline: wgpu::RenderPipeline,

    /// Pipeline that calculates LOD for each chunk and culls them in the camera frustum.
    process_chunks_pipeline: wgpu::ComputePipeline,

    /// Bind group layout with all the data required by the pipeline.
    process_chunks_bind_group_layout: wgpu::BindGroupLayout,

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
    chunk_draw_commands_buffer: wgpu::Buffer,

    /// Total amount of chunks for the entire terrain.
    total_chunks: u32,

    draw_wireframe: bool,
    draw_normals: bool,
    lod_level: usize,

    water: Water,

    normals_lookup: Vec<Vec3>,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct ChunkData {
    min: Vec3,
    _padding1: f32,
    max: Vec3,
    _padding2: f32,
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
        asset_loader: &AssetLoader,
        renderer: &Renderer,
        shaders: &mut Shaders,
        campaign_def: &CampaignDef,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let TerrainMapping {
            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,
            texture_map_base_name,
            water_level,
            ..
        } = {
            let path = PathBuf::from("textures")
                .join("terrain")
                .join(&campaign_def.base_name)
                .join("terrain_mapping.txt");
            info!("Loading terrain mapping: {}", path.display());
            asset_loader.load_config::<TerrainMapping>(&path)?
        };

        let water_level = water_level * altitude_map_height_base;

        let terrain_texture_bind_group = {
            let path = format!("trnhigh/{}.jpg", texture_map_base_name);
            info!("Loading high detail terrain texture: {path}");

            let handle = asset_loader.load_jpeg(path)?;
            let image = asset_loader
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
            let data = asset_loader.load_raw(path)?;
            let mut reader = std::io::Cursor::new(data);
            HeightMap::from_pcx(nominal_edge_size, altitude_map_height_base, &mut reader)
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

        let module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
        );

        let pipeline = {
            let layout = renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("terrain"),
                    bind_group_layouts: &[
                        renderer.texture_bind_group_layout(),
                        camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vertex_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[Vertex::vertex_buffer_layout()],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Cw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        ..Default::default()
                    },
                    depth_stencil: Some(
                        renderer
                            .depth_buffer
                            .depth_stencil_state(wgpu::CompareFunction::Less, true),
                    ),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: Compositor::ALBEDO_TEXTURE_FORMAT,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                            Some(wgpu::ColorTargetState {
                                format: Compositor::POSITION_TEXTURE_FORMAT,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                        ],
                    }),
                    multiview: None,
                    cache: None,
                })
        };

        let wireframe_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("terrain_wireframe", &module)
                .fragment_entry("fragment_main_wireframe")
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                })
                .bind_group_layout(renderer.texture_bind_group_layout())
                .bind_group_layout(camera_bind_group_layout)
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

                chunk_data.push(ChunkData {
                    min,
                    _padding1: 0.0,
                    max,
                    _padding2: 0.0,
                });
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

        let chunk_draw_commands_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_chunk_indirect"),
                    contents: bytemuck::cast_slice(&chunk_indirect_commands),
                    usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE,
                });

        let process_chunks_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("process_chunks"),
                    entries: &[
                        // u_chunk_data
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_draw_commands
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("process_chunks"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "process_chunks.wgsl"
                ))),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain_process_chunks"),
                bind_group_layouts: &[camera_bind_group_layout, &process_chunks_bind_group_layout],
                push_constant_ranges: &[],
            });

        let process_chunks_pipeline =
            renderer
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("process_chunks"),
                    layout: Some(&layout),
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        let water = Water::new(
            asset_loader,
            renderer,
            shaders,
            camera_bind_group_layout,
            Vec2::new(
                height_map.size.x as f32 * height_map.nominal_edge_size,
                height_map.size.y as f32 * height_map.nominal_edge_size,
            ),
            water_level,
        )?;

        let normals = Self::generate_normals_lookup_table();

        Ok(Self {
            height_map,

            max_view_distance: 13_300.0,

            pipeline,
            wireframe_pipeline,
            process_chunks_pipeline,
            process_chunks_bind_group_layout,

            terrain_texture_bind_group,

            draw_wireframe: false,
            draw_normals: false,

            vertices_buffer,
            chunk_indices_buffer,
            chunk_wireframe_indices_buffer,
            chunk_data_buffer,
            chunk_draw_commands_buffer,
            total_chunks,

            lod_level: 0,

            water,

            normals_lookup: normals,
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
        compositor: &Compositor,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let device = &frame.device;
        let queue = &frame.queue;
        let encoder = &mut frame.encoder;
        let surface = &frame.surface;
        let depth_texture = &frame.depth_buffer;

        self.process_chunks(device, queue, camera_bind_group);

        self.render_chunks(encoder, compositor, depth_texture, camera_bind_group);

        if self.draw_wireframe {
            self.render_wireframe(encoder, surface, camera_bind_group);
        }
    }

    pub fn render_water(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        self.water.render(frame, camera_bind_group);
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

        self.water.debug_panel(ui);
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

    /// Run the process_chunks compute shader to cull chunks not in the camera frustum and to set
    /// the LOD level.
    fn process_chunks(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        // Create the bind group
        let process_chunks_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("process_chunks"),
            layout: &self.process_chunks_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.chunk_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.chunk_draw_commands_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("process_chunks_encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("process_chunks"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.process_chunks_pipeline);
            compute_pass.set_bind_group(0, camera_bind_group, &[]);
            compute_pass.set_bind_group(1, &process_chunks_bind_group, &[]);
            compute_pass.dispatch_workgroups((self.total_chunks + 63) / 64, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    fn render_chunks(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        compositor: &Compositor,
        depth_buffer: &DepthBuffer,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terrain_chunks"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &compositor.albedo_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &compositor.position_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_buffer.texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(
            &self.chunk_draw_commands_buffer,
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

    fn render_wireframe(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terrain_chunks"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.wireframe_pipeline);
        render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_wireframe_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);

        let levels = [0..512, 512..640, 640..672, 672..680];
        for chunk_index in 0..self.total_chunks as i32 {
            render_pass.draw_indexed(
                levels[self.lod_level].clone(),
                chunk_index * (Self::VERTICES_PER_CHUNK * Self::VERTICES_PER_CHUNK) as i32,
                0..1,
            );
        }
    }

    fn generate_normals_lookup_table() -> Vec<Vec3> {
        let mut normals = Vec::with_capacity(1024);

        for pitch in 0..16 {
            let z = (pitch as f32).sin();
            for yaw in 0..64 {
                let x = (yaw as f32).cos();
                let y = (yaw as f32).sin();

                normals.push(Vec3::new(x, y, z));
            }
        }

        normals
    }
}
