use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use byteorder::{LittleEndian, ReadBytesExt};
use glam::Mat4;
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{
    engine::renderer::Renderer,
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
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    pipeline: wgpu::RenderPipeline,

    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,

    rotation: f32,
}

fn load_texture_map(path: impl AsRef<Path>) -> (u32, u32, Vec<u8>) {
    let mut file = std::fs::File::open(path.as_ref()).unwrap();

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

    let header = file.read_u8().unwrap();
    let version = file.read_u8().unwrap();
    let encoding_method = file.read_u8().unwrap();
    let bits = file.read_u8().unwrap();
    let min_x = file.read_u16::<LittleEndian>().unwrap();
    let min_y = file.read_u16::<LittleEndian>().unwrap();
    let max_x = file.read_u16::<LittleEndian>().unwrap();
    let max_y = file.read_u16::<LittleEndian>().unwrap();
    let dpi_x = file.read_u16::<LittleEndian>().unwrap();
    let dpi_y = file.read_u16::<LittleEndian>().unwrap();
    file.seek(SeekFrom::Current(48)).unwrap(); // Skip the EGA palette.
    let _ = file.read_u8().unwrap(); // Reserved.
    let color_planes = file.read_u8().unwrap();
    let color_plane_bytes = file.read_u16::<LittleEndian>().unwrap();
    let palette_mode = file.read_u16::<LittleEndian>().unwrap();
    let source_width = file.read_u16::<LittleEndian>().unwrap();
    let source_height = file.read_u16::<LittleEndian>().unwrap();
    file.seek(SeekFrom::Current(54)).unwrap();

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;

    let decoded = if encoding_method == 0 {
        let mut decoded = vec![0_u8; (width * height) as usize];
        file.read_exact(decoded.as_mut()).unwrap();
        decoded
    } else if encoding_method == 1 {
        let mut decoded = Vec::with_capacity((width * height) as usize);
        while let Ok(byte) = file.read_u8() {
            if (byte & 0xC0) != 0xC0 {
                decoded.push(byte);
            } else {
                let count = (byte & 0x3F) as usize;
                let data_byte = file.read_u8().unwrap();
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
        renderer: &Renderer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        model_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let root = PathBuf::from(r"C:\Games\shadow_company\Data\");
        let data = std::fs::read_to_string(
            root.join("textures")
                .join("terrain")
                .join("training")
                .join("terrain_mapping.txt"),
        )
        .unwrap();
        let f = ConfigFile::new(data.as_str());
        let tm = TerrainMapping::from(f);

        // let height_map = load_texture_map(root.join("maps").join("training.pcx"));
        let (width, height, height_map) = load_texture_map(r"C:\Code\granite\test.pcx");

        println!("width: {}, height: {}", width, height);

        let height_map = height_map.into_iter().map(|v| v as f32).collect::<Vec<_>>();

        let mut vertices = Vec::with_capacity(width as usize * height as usize);
        for y in 0..width {
            for x in 0..height {
                // let index = y as usize * width as usize + x as usize;
                // let height = height_map[index];

                vertices.push([
                    (x as f32) * 2.0,
                    (y as f32) * 2.0,
                    0.0, // height
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                ]);
            }
        }

        // vx, vy, vz, nx, ny, nz, u, v
        // let vertices: &[[f32; 8]] = &[
        //     // Back face
        //     [-0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 0.0], // 0
        //     [0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 0.0],  // 1
        //     [0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 1.0, 1.0],   // 2
        //     [-0.5, 0.5, -0.5, 0.0, 0.0, -1.0, 0.0, 1.0],  // 3
        //     // Front face
        //     [-0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0], // 4
        //     [0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0],  // 5
        //     [0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0],   // 6
        //     [-0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 1.0],  // 7
        //     // Left face
        //     [-0.5, -0.5, -0.5, -1.0, 0.0, 0.0, 0.0, 0.0], // 8
        //     [-0.5, 0.5, -0.5, -1.0, 0.0, 0.0, 1.0, 0.0],  // 9
        //     [-0.5, 0.5, 0.5, -1.0, 0.0, 0.0, 1.0, 1.0],   // 10
        //     [-0.5, -0.5, 0.5, -1.0, 0.0, 0.0, 0.0, 1.0],  // 11
        //     // Right face
        //     [0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 0.0, 0.0], // 12
        //     [0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 1.0, 0.0],  // 13
        //     [0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 1.0, 1.0],   // 14
        //     [0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0],  // 15
        //     // Top face
        //     [-0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0], // 16
        //     [0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 1.0, 0.0],  // 17
        //     [0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0, 1.0],   // 18
        //     [-0.5, 0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0],  // 19
        //     // Bottom face
        //     [-0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.0, 0.0], // 20
        //     [0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 1.0, 0.0],  // 21
        //     [0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 1.0, 1.0],   // 22
        //     [-0.5, -0.5, 0.5, 0.0, -1.0, 0.0, 0.0, 1.0],  // 23
        // ];

        let indices: &[u16] = &[
            0, 1, 2, 2, 3, 0, // Back face
            4, 5, 6, 6, 7, 4, // Front face
            8, 9, 10, 10, 11, 8, // Left face
            12, 13, 14, 14, 15, 12, // Right face
            16, 17, 18, 18, 19, 16, // Top face
            20, 21, 22, 22, 23, 20, // Bottom face
        ];

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
                contents: bytemuck::cast_slice(indices),
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

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("terrain_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout, model_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("world_render_pipeline"),
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
                depth_stencil: renderer.depth_stencil_state(),
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
            });

        let model = Mat4::IDENTITY.to_cols_array_2d();
        let model_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("model_buffer"),
                contents: bytemuck::cast_slice(&model),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let model_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model_bind_group"),
                layout: &model_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_buffer.as_entire_binding(),
                }],
            });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,

            pipeline,

            model_buffer,
            model_bind_group,

            rotation: 0.0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        // self.rotation += 0.01 * delta_time;
    }

    pub fn render(
        &self,
        renderer: &crate::engine::renderer::Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        // Update the model details.
        let model = Mat4::from_rotation_y(self.rotation).to_cols_array_2d();
        renderer
            .queue
            .write_buffer(&self.model_buffer, 0, bytemuck::cast_slice(&model));

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
            depth_stencil_attachment: renderer.render_pass_depth_stencil_attachment(),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.model_bind_group, &[]);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
