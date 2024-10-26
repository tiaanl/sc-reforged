use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use glam::{vec3, Vec3};
use tracing::info;
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{
    engine::{
        assets::{AssetError, Assets},
        renderer::Renderer,
    },
    game::config::{ConfigFile, TerrainMapping},
};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coord: [f32; 2],
}

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

    draw_wireframe: bool,
}

fn load_texture_map(data: &[u8]) -> (u32, u32, Vec<u8>) {
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

    let decoded = if encoding_method == 0 {
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

    (width as u32, height as u32, decoded)
}

impl Terrain {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let terrain_name = "training"; // TODO: Replace with campaign name.

        let TerrainMapping {
            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,
            ..
        } = {
            let terrain_mapping_path =
                format!("textures/terrain/{}/terrain_mapping.txt", terrain_name);

            info!("Loading terrain mapping: {}", terrain_mapping_path);

            let data = assets.load_config_file(terrain_mapping_path)?;

            TerrainMapping::from(ConfigFile::new(&data))
        };

        let (height_map_width, height_map_height, _height_map) = {
            let path = format!("maps/{}.pcx", terrain_name); // TODO: Get the name of the map from the [CampaignDef].
            let data = assets.load_raw(path)?;
            load_texture_map(&data)
        };

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map_width, height_map_height,
        );

        let heights = (0..=255)
            .map(|h| (h as f32) * altitude_map_height_base)
            .collect::<Vec<_>>();

        let (vertices, indices, wireframe_indices) = if true {
            let mut vertices =
                Vec::with_capacity(height_map_width as usize * height_map_height as usize);
            let mut indices: Vec<u16> = Vec::with_capacity(
                (height_map_width as usize - 1) * (height_map_height as usize - 1) * 6,
            );
            let mut wireframe_indices: Vec<u16> = Vec::with_capacity(
                (height_map_width as usize - 1) * (height_map_height as usize - 1) * 8,
            );

            for y in 0..height_map_width {
                for x in 0..height_map_height {
                    let height = _height_map[(y * height_map_width + x) as usize] as usize;
                    vertices.push([
                        (x as f32) * nominal_edge_size,
                        heights[height],
                        (y as f32) * nominal_edge_size,
                        0.0,
                        0.0,
                        0.0,
                        1.0,
                        0.0,
                    ]);
                }
            }

            for y in 0..height_map_width - 1 {
                for x in 0..height_map_height - 1 {
                    // Top-left, bottom-left, bottom-right, top-right vertices of the quad
                    let bottom_left = (y * height_map_width + x) as u16;
                    let top_left = ((y + 1) * height_map_width + x) as u16;
                    let top_right = ((y + 1) * height_map_width + (x + 1)) as u16;
                    let bottom_right = (y * height_map_width + (x + 1)) as u16;

                    // First triangle (top-left, bottom-left, bottom-right)
                    indices.push(top_left);
                    indices.push(bottom_left);
                    indices.push(bottom_right);

                    // Second triangle (top-left, bottom-right, top-right)
                    indices.push(top_left);
                    indices.push(bottom_right);
                    indices.push(top_right);

                    // Horizontal edges
                    wireframe_indices.push(top_left);
                    wireframe_indices.push(top_right);

                    // Vertical edge (right of the quad)
                    wireframe_indices.push(top_right);
                    wireframe_indices.push(bottom_right);
                }
            }

            for x in 0..height_map_width - 1 {
                let bottom_left = x as u16;
                let bottom_right = (x + 1) as u16;
                wireframe_indices.push(bottom_left);
                wireframe_indices.push(bottom_right);
            }

            // Add the last column's vertical edges
            for y in 0..height_map_height - 1 {
                let bottom_left = (y * height_map_width) as u16;
                let top_left = ((y + 1) * height_map_width) as u16;
                wireframe_indices.push(bottom_left);
                wireframe_indices.push(top_left);
            }

            (
                vertices,
                indices.iter().map(|i| *i as u16).collect(),
                wireframe_indices,
            )
        } else {
            // vx, vy, vz, nx, ny, nz, u, v
            let vertices = vec![
                // Back face
                [-0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 0.0], // 0
                [0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 0.0],  // 1
                [0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 1.0],   // 2
                [-0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 1.0],  // 3
                // Front face
                [-0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0], // 4
                [0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0],  // 5
                [0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0],   // 6
                [-0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0],  // 7
                // Left face
                [-0.5, -0.5, -0.5, -1.0, 0.0, 0.0, 0.0, 0.0], // 8
                [-0.5, 0.5, -0.5, -1.0, 0.0, 0.0, 1.0, 0.0],  // 9
                [-0.5, 0.5, 0.5, -1.0, 0.0, 0.0, 1.0, 1.0],   // 10
                [-0.5, -0.5, 0.5, -1.0, 0.0, 0.0, 0.0, 1.0],  // 11
                // Right face
                [0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 0.0, 0.0], // 12
                [0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 0.0],  // 13
                [0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 1.0, 1.0],   // 14
                [0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0],  // 15
                // Top face
                [-0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0], // 16
                [0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 1.0, 0.0],  // 17
                [0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0, 1.0],   // 18
                [-0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0],  // 19
                // Bottom face
                [-0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.0, 0.0], // 20
                [0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 1.0, 0.0],  // 21
                [0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 1.0, 1.0],   // 22
                [-0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 0.0, 1.0],  // 23
            ];

            let indices: &[u16] = &[
                0, 1, 2, 2, 3, 0, // Back face
                4, 5, 6, 6, 7, 4, // Front face
                8, 9, 10, 10, 11, 8, // Left face
                12, 13, 14, 14, 15, 12, // Right face
                16, 17, 18, 18, 19, 16, // Top face
                20, 21, 22, 22, 23, 20, // Bottom face
            ];

            let wireframe_indices = &[0, 1, 1, 2];

            (vertices, indices.to_vec(), wireframe_indices.to_vec())
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
                        bind_group_layouts: &[camera_bind_group_layout],
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
                            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
            height_map_width,
            height_map_height,

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

            draw_wireframe: false,
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

    pub fn update(&mut self, _delta_time: f32) {}

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");

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
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
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
                wgpu::IndexFormat::Uint16,
            );
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.draw_indexed(0..self.wireframe_index_count, 0, 0..1);
        }
    }
}
