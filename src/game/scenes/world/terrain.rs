use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use glam::{IVec2, UVec2};
use naga_oil::compose::ShaderDefValue;
use tracing::info;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::Camera,
        config::{CampaignDef, TerrainMapping},
    },
};

use super::height_map::HeightMap;

struct ChunkMesh {
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,
    wireframe_indices_buffer: wgpu::Buffer,
}

impl ChunkMesh {
    fn new(renderer: &Renderer) -> Self {
        let mut vertices = Vec::with_capacity(
            Terrain::VERTICES_PER_CHUNK as usize * Terrain::VERTICES_PER_CHUNK as usize,
        );
        for y in 0..Terrain::VERTICES_PER_CHUNK {
            for x in 0..Terrain::VERTICES_PER_CHUNK {
                vertices.push(TerrainVertex {
                    index: UVec2::new(x, y),
                });
            }
        }

        let vertices_buffer = renderer.create_vertex_buffer("chunk_vertices", &vertices);

        // level 0 = 0..384
        // level 1 = 0..96
        // level 2 = 0..24
        // level 3 = 0..6
        let mut indices = Vec::<u32>::with_capacity(384_usize + 96_usize + 24_usize + 6_usize);

        // level 0 = 0..512
        // level 1 = 0..128
        // level 2 = 0..32
        // level 3 = 0..8
        let mut wireframe_indices =
            Vec::<u32>::with_capacity(512_usize + 128_usize + 32_usize + 8_usize);

        for level in 0..=Terrain::LOD_MAX {
            let cell_count = Terrain::CELLS_PER_CHUNK >> level;
            let scale = 1 << level;

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = (y * Terrain::VERTICES_PER_CHUNK + x) * scale;
                    let i1 = (y * Terrain::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i2 = ((y + 1) * Terrain::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i3 = ((y + 1) * Terrain::VERTICES_PER_CHUNK + x) * scale;

                    indices.extend_from_slice(&[i0, i1, i2, i2, i3, i0]);
                    wireframe_indices.extend_from_slice(&[i0, i1, i1, i2, i2, i3, i3, i0]);
                }
            }

            println!("len: {}", wireframe_indices.len());
        }

        let indices_buffer = renderer.create_index_buffer("chunk_indices", &indices);
        let wireframe_indices_buffer =
            renderer.create_index_buffer("chunk_wireframe_indices", &wireframe_indices);

        Self {
            vertices_buffer,
            indices_buffer,
            wireframe_indices_buffer,
        }
    }
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct TerrainData {
    size: UVec2,
    nominal_edge_size: f32,
    water_level: f32,

    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,

    _padding: f32,
}

pub struct Terrain {
    /// Height data for the terrain.
    height_map: HeightMap,

    /// Dictates the terrain resolution.
    pub max_view_distance: f32,

    /// Pipeline to render the terrain.
    terrain_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render the water.
    water_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render a wireframe over the terrain.
    wireframe_pipeline: wgpu::RenderPipeline,

    /// Pipeline that calculates LOD for each chunk and culls them in the camera frustum.
    process_chunks_pipeline: wgpu::ComputePipeline,

    /// Bind group layout with all the data required by the pipeline.
    process_chunks_bind_group_layout: wgpu::BindGroupLayout,

    /// The texture used to render over the entire terrain.
    terrain_texture_bind_group: wgpu::BindGroup,

    /// The water texture.
    water_texture_bind_group: wgpu::BindGroup,

    /// The mesh we use to render chunks.
    chunk_mesh: ChunkMesh,

    /// Holds data for the terrain used by the GPU.
    terrain_data_uniform: UniformBuffer<TerrainData>,

    height_map_buffer: StorageBuffer<f32>,

    draw_wireframe: bool,
    draw_normals: bool,
    lod_level: usize,

    normals_lookup: Vec<Vec3>,
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
struct TerrainVertex {
    index: UVec2,
}

impl BufferLayout for TerrainVertex {
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>] {
        const BUFFERS: &[wgpu::VertexBufferLayout] = &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Uint32x2,
            ],
        }];

        BUFFERS
    }
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

            water_trans_depth,
            water_trans_high,
            water_trans_low,
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

        // let UVec2 {
        //     x: chunks_x,
        //     y: chunks_y,
        // } = height_map.size / Terrain::CELLS_PER_CHUNK;
        // let total_chunks = chunks_x * chunks_y;

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map.size.x, height_map.size.y,
        );

        let height_map_buffer = {
            let mut nodes =
                Vec::with_capacity(height_map.size.y as usize * height_map.size.x as usize);

            for y in 0..(height_map.size.y + 1) {
                for x in 0..(height_map.size.x + 1) {
                    let position = height_map.position_for_vertex(IVec2::new(x as i32, y as i32));
                    nodes.push(position.z);
                }
            }

            StorageBuffer::new(
                renderer,
                "height_map",
                wgpu::ShaderStages::VERTEX,
                true,
                nodes,
            )
        };

        let terrain_data_uniform = UniformBuffer::with_data(
            renderer,
            "terrain_data",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            TerrainData {
                size: height_map.size,
                nominal_edge_size,
                water_level,

                water_trans_depth,
                water_trans_low: water_trans_low as f32 / 512.0,
                water_trans_high: water_trans_high as f32 / 512.0,

                _padding: 0.0,
            },
        );

        let module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
            HashMap::default(),
        );

        let terrain_pipeline = renderer
            .build_render_pipeline::<TerrainVertex>("terrain", &module)
            .with_vertex_entry("vertex_main")
            .with_fragment_entry("fragment_main")
            .binding(camera_bind_group_layout)
            .binding(&height_map_buffer.bind_group_layout)
            .binding(&terrain_data_uniform.bind_group_layout)
            .binding(renderer.texture_bind_group_layout())
            .push_constant(wgpu::ShaderStages::VERTEX, 0..8)
            .with_depth_compare(wgpu::CompareFunction::LessEqual)
            .build();

        let water_pipeline = renderer
            .build_render_pipeline::<TerrainVertex>("water", &module)
            .with_vertex_entry("water_vertex_main")
            .with_fragment_entry("water_fragment_main")
            .binding(camera_bind_group_layout)
            .binding(&height_map_buffer.bind_group_layout)
            .binding(&terrain_data_uniform.bind_group_layout)
            .binding(renderer.texture_bind_group_layout())
            .push_constant(wgpu::ShaderStages::VERTEX, 0..8)
            .with_depth_compare(wgpu::CompareFunction::LessEqual)
            // .with_depth_writes(false)
            .with_blend(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            })
            .build();

        let module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
            HashMap::from([("WIREFRAME".to_string(), ShaderDefValue::Bool(true))]),
        );

        let wireframe_pipeline = renderer
            .build_render_pipeline::<TerrainVertex>("terrain_wireframe", &module)
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            })
            .with_vertex_entry("wireframe_vertex_main")
            .with_fragment_entry("wireframe_fragment_main")
            .binding(camera_bind_group_layout)
            .binding(&height_map_buffer.bind_group_layout)
            .binding(&terrain_data_uniform.bind_group_layout)
            .push_constant(wgpu::ShaderStages::VERTEX, 0..8)
            .build();

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

        let normals_lookup = Self::generate_normals_lookup_table();

        let chunk_mesh = ChunkMesh::new(renderer);

        let water_texture_bind_group = {
            let image = asset_loader.load_bmp(
                PathBuf::from("textures")
                    .join("image_processor")
                    .join("water2.bmp"),
            )?;
            let image = asset_loader
                .asset_store()
                .get(image)
                .expect("just loaded it successfully.");
            let water_texture = renderer.create_texture_view("water", &image.data);

            let sampler = renderer.create_sampler(
                "water",
                wgpu::AddressMode::Repeat,
                wgpu::FilterMode::Linear,
                wgpu::FilterMode::Linear,
            );

            renderer.create_texture_bind_group("water", &water_texture, &sampler)
        };

        Ok(Self {
            height_map,
            max_view_distance: 13_300.0,
            terrain_pipeline,
            water_pipeline,
            wireframe_pipeline,
            process_chunks_pipeline,
            process_chunks_bind_group_layout,
            terrain_texture_bind_group,
            water_texture_bind_group,
            chunk_mesh,
            terrain_data_uniform,
            height_map_buffer,
            draw_wireframe: false,
            draw_normals: false,
            lod_level: 0,
            normals_lookup,
        })
    }

    pub fn update(&mut self, _camera: &Camera) {}

    pub fn render(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        // self.process_chunks(&frame.device, &frame.queue, camera_bind_group);
        self.render_terrain(frame, camera_bind_group);
    }

    pub fn render_gizmos(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        if self.draw_wireframe {
            self.render_wireframe(frame, camera_bind_group);
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
    /// Run the process_chunks compute shader to cull chunks not in the camera frustum and to set
    /// the LOD level.
    fn process_chunks(
        &self,
        _device: &RenderDevice,
        _queue: &RenderQueue,
        _camera_bind_group: &wgpu::BindGroup,
    ) {
        /*
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
        */
    }

    fn render_terrain(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        // LOD ranges.
        let range = [0..384, 384..480, 480..504, 504..510];

        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("terrain_chunks"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame.surface,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &frame.depth_buffer.texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
            render_pass.set_index_buffer(
                self.chunk_mesh.indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.height_map_buffer.bind_group, &[]);
            render_pass.set_bind_group(2, &self.terrain_data_uniform.bind_group, &[]);
            render_pass.set_bind_group(3, &self.terrain_texture_bind_group, &[]);

            for y in 0..self.height_map.size.y / Terrain::CELLS_PER_CHUNK {
                for x in 0..self.height_map.size.x / Terrain::CELLS_PER_CHUNK {
                    render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, &x.to_ne_bytes());
                    render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 4, &y.to_ne_bytes());
                    render_pass.draw_indexed(range[self.lod_level].clone(), 0, 0..1);
                }
            }
        }
    }

    pub fn render_water(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        // LOD ranges.
        let range = [0..384, 384..480, 480..504, 504..510];

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.water_pipeline);

        render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_mesh.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.height_map_buffer.bind_group, &[]);
        render_pass.set_bind_group(2, &self.terrain_data_uniform.bind_group, &[]);
        render_pass.set_bind_group(3, &self.water_texture_bind_group, &[]);

        for y in 0..self.height_map.size.y / Terrain::CELLS_PER_CHUNK {
            for x in 0..self.height_map.size.x / Terrain::CELLS_PER_CHUNK {
                render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, &x.to_ne_bytes());
                render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 4, &y.to_ne_bytes());
                render_pass.draw_indexed(range[3].clone(), 0, 0..1);
            }
        }
    }

    fn render_wireframe(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        let levels = [0..512, 512..640, 640..672, 672..680];

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_chunks"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
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
        render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_mesh.wireframe_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.height_map_buffer.bind_group, &[]);
        render_pass.set_bind_group(2, &self.terrain_data_uniform.bind_group, &[]);

        for y in 0..self.height_map.size.y / Terrain::CELLS_PER_CHUNK {
            for x in 0..self.height_map.size.x / Terrain::CELLS_PER_CHUNK {
                render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, &x.to_ne_bytes());
                render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 4, &y.to_ne_bytes());
                // Render level 0.
                render_pass.draw_indexed(levels[self.lod_level].clone(), 0, 0..1);
            }
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
