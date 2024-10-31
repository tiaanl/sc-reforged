use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{vec2, vec3, Vec2, Vec3};
use tracing::info;
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{
    engine::{
        assets::{AssetError, Assets, Image},
        gizmos::GizmoVertex,
        renderer::{GpuTexture, Renderer},
    },
    game::config::{ConfigFile, TerrainMapping},
};

pub struct Terrain {
    height_map_width: u32,
    height_map_height: u32,

    altitude_map_height_base: f32,
    map_dx: f32,
    map_dy: f32,
    nominal_edge_size: f32,

    vertex_buffer: wgpu::Buffer,

    index_buffer: wgpu::Buffer,
    index_count: u32,

    wireframe_index_buffer: wgpu::Buffer,
    wireframe_index_count: u32,

    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,

    terrain_texture_bind_group: wgpu::BindGroup,
    terrain_texture: GpuTexture,

    draw_wireframe: bool,
    draw_normals: bool,

    vertices: Vec<Vertex>,
    normals_table: Vec<Vec3>,
}

fn load_texture_map(data: &[u8]) -> Image {
    /*
    typedef struct _PcxHeader
    {
        BYTE	Identifier;        /* PCX Id Number (Always 0x0A) */
        BYTE	Version;           /* Version Number */
        BYTE	Encoding;          /* Encoding Format */
        BYTE	BitsPerPixel;      /* Bits per Pixel */
        WORD	XStart;            /* Left of image */
        WORD	YStart;            /* Top of Image */
        WORD	XEnd;              /* Right of Image */
        WORD	YEnd;              /* Bottom of image */
        WORD	HorzRes;           /* Horizontal Resolution */
        WORD	VertRes;           /* Vertical Resolution */
        BYTE	Palette[48];       /* 16-Color EGA Palette */
        BYTE	Reserved1;         /* Reserved (Always 0) */
        BYTE	NumBitPlanes;      /* Number of Bit Planes */
        WORD	BytesPerLine;      /* Bytes per Scan-line */
        WORD	PaletteType;       /* Palette Type */
        WORD	HorzScreenSize;    /* Horizontal Screen Size */
        WORD	VertScreenSize;    /* Vertical Screen Size */
        BYTE	Reserved2[54];     /* Reserved (Always 0) */
    } PCXHEAD;
    */

    let mut data = std::io::Cursor::new(data);

    let _header = data.read_u8().unwrap();
    let _version = data.read_u8().unwrap();
    let encoding_method = data.read_u8().unwrap();
    let _bits = data.read_u8().unwrap();
    let min_x = data.read_u16::<LittleEndian>().unwrap();
    let min_y = data.read_u16::<LittleEndian>().unwrap();
    let max_x = data.read_u16::<LittleEndian>().unwrap();
    let max_y = data.read_u16::<LittleEndian>().unwrap();
    let _dpi_x = data.read_u16::<LittleEndian>().unwrap();
    let _dpi_y = data.read_u16::<LittleEndian>().unwrap();
    data.seek(SeekFrom::Current(48)).unwrap(); // Skip the EGA palette.
    let _ = data.read_u8().unwrap(); // Reserved.
    let _color_planes = data.read_u8().unwrap();
    let _color_plane_bytes = data.read_u16::<LittleEndian>().unwrap();
    let _palette_mode = data.read_u16::<LittleEndian>().unwrap();
    let _source_width = data.read_u16::<LittleEndian>().unwrap();
    let _source_height = data.read_u16::<LittleEndian>().unwrap();
    data.seek(SeekFrom::Current(54)).unwrap();

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;

    let data = if encoding_method == 0 {
        let mut decoded = vec![0_u8; width as usize * height as usize];
        data.read_exact(decoded.as_mut()).unwrap();
        decoded
    } else if encoding_method == 1 {
        let mut decoded = Vec::with_capacity(width as usize * height as usize);
        while let Ok(byte) = data.read_u8() {
            if (byte & 0xC0) != 0xC0 {
                decoded.push(byte);
            } else {
                let count = (byte & 0x3F) as usize;
                let data_byte = data.read_u8().unwrap();
                decoded.extend(std::iter::repeat(data_byte).take(count));
            }
        }
        decoded
    } else {
        panic!("Invalid PCX encoding");
    };

    Image {
        data,
        width: width as u32,
        height: height as u32,
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
}

impl Terrain {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let terrain_name = "kola"; // TODO: Replace with campaign name.

        let TerrainMapping {
            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,
            texture_map_base_name,
            ..
        } = {
            let terrain_mapping_path =
                format!("textures/terrain/{}/terrain_mapping.txt", terrain_name);
            info!("Loading terrain mapping: {}", terrain_mapping_path);
            let data = assets.load_config_file(terrain_mapping_path)?;
            TerrainMapping::from(ConfigFile::new(&data))
        };

        let terrain_texture = {
            use crate::engine::assets::Image;

            let path = format!("trnhigh/{}.jpg", texture_map_base_name);
            info!("Loading high detail terrain texture: {path}");

            let Image {
                data,
                width,
                height,
            } = assets.load_jpeg(path)?;

            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            let texture = renderer.device.create_texture(
                &(wgpu::TextureDescriptor {
                    label: Some("terrain_trnhigh_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }),
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            renderer.queue.write_texture(
                wgpu::ImageCopyTextureBase {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::default(),
                    aspect: wgpu::TextureAspect::All,
                },
                data.as_ref(),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                size,
            );

            GpuTexture {
                texture,
                view,
                sampler,
            }
        };

        let terrain_texture_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("terrain_texture_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let terrain_texture_bind_group =
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("terrain_texture_bind_group"),
                    layout: &terrain_texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&terrain_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&terrain_texture.sampler),
                        },
                    ],
                });

        let height_map = {
            let path = format!("maps/{}.pcx", terrain_name); // TODO: Get the name of the map from the [CampaignDef].
            info!("Loading terrain height map: {path}");
            let data = assets.load_raw(path)?;
            load_texture_map(&data)
        };

        let normals_lookup = {
            let path = format!("textures/terrain/{terrain_name}/{terrain_name}_vn.dat");
            info!("Loading normals lookup data from: {path}");
            let mut r = std::io::Cursor::new(assets.load_raw(path)?);
            (0..(height_map.width as usize * height_map.height as usize))
                .into_iter()
                .map(|_| r.read_u16::<LittleEndian>().unwrap())
                .collect::<Vec<_>>()
        };

        let normals_table = {
            let mut normals = vec![];
            for angle_group in 0..16 {
                let y = (angle_group as f32 * 0.09817477).sin();
                for angle_step in 0..64 {
                    let x = (angle_step as f32 * 0.09817477).cos();
                    let z = (angle_step as f32 * 0.09817477).sin();
                    normals.push(vec3(x, y, z).normalize());
                }
            }
            normals
        };

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map.width, height_map.height,
        );

        let heights = (0..=255)
            .map(|h| ((255 - h) as f32) * altitude_map_height_base)
            .collect::<Vec<_>>();

        macro_rules! index {
            ($x:expr,$y:expr) => {{
                (($y as u32) * height_map.height + ($x as u32)) as u32
            }};
        }

        let value = normals_lookup[index!(height_map.width / 2, height_map.height / 2) as usize];
        info!("value: {}", value);

        let (vertices, indices, wireframe_indices) = {
            let mut vertices =
                Vec::with_capacity(height_map.width as usize * height_map.height as usize);
            let mut indices = Vec::with_capacity(
                (height_map.width as usize - 1) * (height_map.height as usize - 1) * 6,
            );
            let mut wireframe_indices =
                Vec::with_capacity((height_map.width as usize) * (height_map.height as usize) * 4);

            for y in 0..height_map.height {
                for x in 0..height_map.width {
                    let altitude = heights[height_map.data[index!(x, y) as usize] as usize];
                    let normal = normals_table[normals_lookup[index!(x, y) as usize] as usize];

                    vertices.push(Vertex {
                        position: vec3(
                            (x as f32) * nominal_edge_size,
                            altitude as f32,
                            (y as f32) * nominal_edge_size,
                        ),
                        normal,
                        tex_coord: vec2(
                            x as f32 / (height_map.width - 1) as f32,
                            y as f32 / (height_map.height - 1) as f32,
                        ),
                    });
                }
            }

            for y in 0..(height_map.height - 1) {
                for x in 0..(height_map.width - 1) {
                    indices.push(index!(x, y));
                    indices.push(index!(x + 1, y));
                    indices.push(index!(x, y + 1));

                    indices.push(index!(x + 1, y));
                    indices.push(index!(x + 1, y + 1));
                    indices.push(index!(x, y + 1));

                    wireframe_indices.push(index!(x, y));
                    wireframe_indices.push(index!(x + 1, y));

                    wireframe_indices.push(index!(x, y));
                    wireframe_indices.push(index!(x, y + 1));
                }

                wireframe_indices.push(index!(height_map.width - 1, y));
                wireframe_indices.push(index!(height_map.width - 1, y + 1));
            }
            for x in 0..(height_map.width - 1) {
                wireframe_indices.push(index!(x, height_map.height - 1));
                wireframe_indices.push(index!(x + 1, height_map.height - 1));
            }

            (vertices, indices, wireframe_indices)
        };

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_vertex_buffer"),
                contents: bytemuck::cast_slice(vertices.as_ref()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let wireframe_index_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_wireframe_index_buffer"),
                    contents: bytemuck::cast_slice(&wireframe_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let shader_module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("terrain_shadow_module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "world.wgsl"
                ))),
            });

        let pipeline = {
            let pipeline_layout =
                renderer
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("terrain_pipeline_layout"),
                        bind_group_layouts: &[
                            camera_bind_group_layout,
                            &terrain_texture_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vertex_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                0 => Float32x3,
                                1 => Float32x3,
                                2 => Float32x2,
                            ],
                        }],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: renderer.depth_stencil_state(wgpu::CompareFunction::Less),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: "fragment_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface_config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                })
        };

        let wireframe_pipeline = {
            let writeframe_pipeline_layout =
                renderer
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("terrain_writeframe_pipeline_layout"),
                        bind_group_layouts: &[camera_bind_group_layout],
                        push_constant_ranges: &[],
                    });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_wireframe_render_pipeline"),
                    layout: Some(&writeframe_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vertex_main_wireframe",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                0 => Float32x3,
                                1 => Float32x3,
                                2 => Float32x2,
                            ],
                        }],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        ..Default::default()
                    },
                    depth_stencil: renderer.depth_stencil_state(wgpu::CompareFunction::Always),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: "fragment_main_wireframe",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface_config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                })
        };

        Ok(Self {
            height_map_width: height_map.width,
            height_map_height: height_map.height,

            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,

            vertex_buffer,

            index_buffer,
            index_count: indices.len() as u32,

            wireframe_index_buffer,
            wireframe_index_count: wireframe_indices.len() as u32,

            pipeline,
            wireframe_pipeline,

            terrain_texture_bind_group,
            terrain_texture,

            draw_wireframe: false,
            draw_normals: false,

            vertices,
            normals_table,
        })
    }

    /// Return the max bounds of the terrain. The min value of the bound is
    /// [`Vec3::ZERO`].
    pub fn _bounds(&self) -> Vec3 {
        vec3(
            self.map_dx * self.nominal_edge_size,
            self.altitude_map_height_base * 255.0,
            self.map_dy * self.nominal_edge_size,
        )
    }

    pub fn render_normals(&self) -> Vec<GizmoVertex> {
        const LENGTH: f32 = 100.0;

        let mut vertices = vec![];

        if !self.draw_normals {
            return vertices;
        }

        let width = self.height_map_width as usize;
        let height = self.height_map_height as usize;

        let color = [0.0, 1.0, 1.0, 1.0];

        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;

                let x = self.vertices[index].position.x;
                let y = self.vertices[index].position.y;
                let z = self.vertices[index].position.z;
                vertices.push(GizmoVertex {
                    position: [x, y, z, 1.0],
                    color,
                });

                let x = x + self.vertices[index].normal.x * LENGTH;
                let y = y + self.vertices[index].normal.y * LENGTH;
                let z = z + self.vertices[index].normal.z * LENGTH;
                vertices.push(GizmoVertex {
                    position: [x, y, z, 1.0],
                    color,
                });
            }
        }

        vertices
    }

    pub fn render_normals_lookup(&self) -> Vec<GizmoVertex> {
        const SIZE: f32 = 100.0;

        let mut vertices = vec![];
        for (i, v) in self.normals_table.iter().enumerate() {
            let color = if i == 48 {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            vertices.push(GizmoVertex {
                position: [0.0, 0.0, 0.0, 1.0],
                color,
            });
            vertices.push(GizmoVertex {
                position: [v.x * SIZE, v.y * SIZE, v.z * SIZE, 1.0],
                color,
            });
        }
        vertices
    }

    pub fn update(&mut self, _delta_time: f32) {}

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");
        ui.checkbox(&mut self.draw_normals, "Draw normals");

        egui::Grid::new("terrain_data").show(ui, |ui| {
            ui.label("height map size");
            ui.label(format!(
                "{} x {}",
                self.height_map_width, self.height_map_height
            ));
            ui.end_row();

            ui.label("terrain mapping size");
            ui.label(format!("{} x {}", self.map_dx, self.map_dy));
            ui.end_row();

            ui.label("nominal edge size");
            ui.label(format!("{}", self.nominal_edge_size));
            ui.end_row();

            ui.label("altitude map height base");
            ui.label(format!("{}", self.altitude_map_height_base));
        });
    }

    pub fn render(
        &self,
        renderer: &crate::engine::renderer::Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 0.4,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: renderer
                    .render_pass_depth_stencil_attachment(wgpu::LoadOp::Clear(1.0)),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_texture_bind_group, &[]);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        if self.draw_wireframe {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: renderer
                    .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.wireframe_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.wireframe_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.draw_indexed(0..self.wireframe_index_count, 0, 0..1);
        }
    }
}
