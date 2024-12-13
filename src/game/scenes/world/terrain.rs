use byteorder::{LittleEndian as LE, ReadBytesExt};
use glam::{Vec2, Vec3, Vec4};
use tracing::info;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::*},
    game::{
        asset_loader::{AssetError, AssetLoader},
        config::{CampaignDef, ConfigFile, TerrainMapping},
    },
};

pub struct Terrain {
    height_map_width: u32,
    height_map_height: u32,

    altitude_map_height_base: f32,
    map_dx: f32,
    map_dy: f32,
    nominal_edge_size: f32,

    bounds_min: Vec2,
    bounds_max: Vec2,

    // Rendering
    //
    vertex_buffer: wgpu::Buffer,

    index_buffer: wgpu::Buffer,
    index_count: u32,

    wireframe_index_buffer: wgpu::Buffer,
    wireframe_index_count: u32,

    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,

    terrain_texture_bind_group: wgpu::BindGroup,

    draw_wireframe: bool,
    draw_normals: bool,

    vertices: Vec<Vertex>,

    #[cfg(feature = "load_normals")]
    normals_table: Vec<Vec3>,
}

struct HeightMap {
    width: u32,
    height: u32,
    heights: Vec<u32>,
}

#[repr(C)]
struct PcxHeader {
    // always 0x0A
    manufacturer: u8,

    // 0 = v2.5
    // 2 = v2.8 with palette
    // 3 = v2.8 without palette
    // 4 = Paintbrush for Windows
    // 5 = v3.0 or higher
    version: u8,

    // 0 = uncompressed image (not officially allowed)
    // 1 = PCX run length encoding
    // should be 0x01
    encoding: u8,

    // number of bits per pixel in each entry of the color planes
    bits_per_plane: u8,

    x_min: u16,
    y_min: u16,
    x_max: u16,
    y_max: u16,

    horizontal_dpi: u16,
    vertical_dpi: u16,

    // palette for 16 colors or less, in three-byte RGB entries.
    palette: [u8; 48],

    // should be set to 0.
    reserved: u8,

    // Number of color planes. Multiply by bits_per_pixel to fet the actual color depth.
    color_planes: u8,

    // number of bytes to read for a single plane's scanline.
    bytes_per_plane_line: u16,

    // 1 = color/bw
    // 2 = grayscale
    palette_info: u16,

    // deals with scrolling, best to just ignore.
    horizontal_screen_size: u16,
    vertical_screen_size: u16,

    padding: [u8; 54],
}

fn read_pcx_header<R>(reader: &mut R) -> std::io::Result<PcxHeader>
where
    R: std::io::Read,
{
    let mut header: PcxHeader = unsafe {
        // We will overwrite all the fields, so we can leave them as garbage.
        std::mem::MaybeUninit::uninit().assume_init_read()
    };

    header.manufacturer = reader.read_u8()?;
    header.version = reader.read_u8()?;
    header.encoding = reader.read_u8()?;
    header.bits_per_plane = reader.read_u8()?;
    header.x_min = reader.read_u16::<LE>()?;
    header.y_min = reader.read_u16::<LE>()?;
    header.x_max = reader.read_u16::<LE>()?;
    header.y_max = reader.read_u16::<LE>()?;
    header.horizontal_dpi = reader.read_u16::<LE>()?;
    header.vertical_dpi = reader.read_u16::<LE>()?;
    reader.read_exact(&mut header.palette)?;
    header.reserved = reader.read_u8()?;
    header.color_planes = reader.read_u8()?;
    header.bytes_per_plane_line = reader.read_u16::<LE>()?;
    header.palette_info = reader.read_u16::<LE>()?;
    header.horizontal_screen_size = reader.read_u16::<LE>()?;
    header.vertical_screen_size = reader.read_u16::<LE>()?;
    reader.read_exact(&mut header.padding)?;

    Ok(header)
}

fn load_texture_map(data: &[u8]) -> std::io::Result<HeightMap> {
    let mut data = std::io::Cursor::new(data);
    let header = read_pcx_header(&mut data)?;

    if header.manufacturer != 0x0A || header.version != 0x05 {
        panic!("Incorrect/invalid PCX header.");
    }

    let width = header.bytes_per_plane_line as u32;
    let height = (header.y_max - header.y_min + 1) as u32;
    let area = width * height;

    tracing::info!("width: {}, height: {}", width as u32, height as u32);

    let mut heights: Vec<u32> = Vec::with_capacity(area as usize);

    macro_rules! b {
        ($b:expr) => {{
            0x1FF00_u32 | (0xFF - $b) as u32
        }};
    }

    let mut i = 0_usize;
    while i < area as usize {
        let byte = data.read_u8()?;
        if (byte & 0xC0) != 0xC0 {
            heights.push(b!(byte));
            i += 1;
        } else {
            let count = (byte & 0x3F) as usize;
            let new_byte = data.read_u8()?;
            heights.extend(std::iter::repeat(b!(new_byte)).take(count));
            i += count;
        }
    }

    assert_eq!(heights.len(), area as usize);

    Ok(HeightMap {
        width,
        height,
        heights,
    })
}

impl Terrain {
    pub fn new(
        assets: &AssetLoader,
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
            ..
        } = {
            let terrain_mapping_path = format!(
                "textures/terrain/{}/terrain_mapping.txt",
                campaign_def.base_name
            );
            info!("Loading terrain mapping: {}", terrain_mapping_path);
            let data = assets.load_string(terrain_mapping_path)?;
            TerrainMapping::from(ConfigFile::new(&data))
        };

        let terrain_texture_bind_group = {
            // use crate::engine::assets::Image;

            let path = format!("trnhigh/{}.jpg", texture_map_base_name);
            info!("Loading high detail terrain texture: {path}");

            let texture_view =
                renderer.create_texture_view("terrain_texture", assets.load_jpeg(path)?);

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
            let path = format!("maps/{}.pcx", campaign_def.base_name); // TODO: Get the name of the map from the [CampaignDef].
            info!("Loading terrain height map: {path}");
            let data = assets.load_raw(path)?;
            load_texture_map(&data).map_err(|_| AssetError::DecodeError)?
        };

        // Generate an array for each edge.
        let x_sides = (0..height_map.width)
            .map(|v| v as f32 * nominal_edge_size)
            .collect::<Vec<_>>();
        let y_sides = (0..height_map.height)
            .map(|v| v as f32 * nominal_edge_size)
            .collect::<Vec<_>>();

        let bounds_min = Vec2::new(x_sides[0] - 2500.0, y_sides[0] - 2500.0);
        let bounds_max = Vec2::new(
            *x_sides.last().unwrap() + 2500.0,
            *y_sides.last().unwrap() + 2500.0,
        );

        tracing::info!(
            "Terrain bounds: ({}, {}) - ({}, {})",
            bounds_min.x,
            bounds_min.y,
            bounds_max.x,
            bounds_max.y
        );

        #[cfg(feature = "load_normals")]
        {
            let normals_lookup = {
                let path = format!(
                    "textures/terrain/{}/{}_vn.dat",
                    campaign_def.base_name, campaign_def.base_name
                );
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
        }

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map.width, height_map.height,
        );

        // Build a lookup table for altitudes.
        let heights = (0..0x100)
            .map(|h| (h as f32) * altitude_map_height_base)
            .collect::<Vec<_>>();

        macro_rules! index {
            ($x:expr,$y:expr) => {{
                (($y as u32) * height_map.height + ($x as u32)) as u32
            }};
        }

        #[cfg(feature = "load_normals")]
        {
            let value =
                normals_lookup[index!(height_map.width / 2, height_map.height / 2) as usize];
            info!("value: {}", value);
        }

        let (mut vertices, indices, wireframe_indices) = {
            let mut vertices =
                Vec::with_capacity(height_map.width as usize * height_map.height as usize);
            let mut indices = Vec::with_capacity(
                (height_map.width as usize - 1) * (height_map.height as usize - 1) * 6,
            );
            let mut wireframe_indices =
                Vec::with_capacity((height_map.width as usize) * (height_map.height as usize) * 4);

            for y in 0..height_map.height {
                for x in 0..height_map.width {
                    let index = y as usize * height_map.width as usize + x as usize;
                    let altitude = heights[height_map.heights[index] as usize & 0xFF];
                    #[cfg(feature = "load_normals")]
                    let normal = normals_table[normals_lookup[index!(x, y) as usize] as usize];

                    #[cfg(not(feature = "load_normals"))]
                    let normal = Vec3::Y;

                    vertices.push(Vertex {
                        position: Vec3::new(
                            (x as f32) * nominal_edge_size,
                            (y as f32) * nominal_edge_size,
                            altitude as f32,
                        ),
                        normal,
                        tex_coord: Vec2::new(
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

        // Calculate the normals of each vertex of the terrain.
        {
            let (width, height) = (height_map.width as usize, height_map.height as usize);
            for y in 1..(height - 1) {
                for x in 1..(width - 1) {
                    let center = y * width + x;
                    let c_pos = vertices[center].position;

                    let right = (vertices[center + 1].position - c_pos).normalize();
                    let down = (vertices[center - width].position - c_pos).normalize();
                    let left = (vertices[center - 1].position - c_pos).normalize();
                    let up = (vertices[center + width].position - c_pos).normalize();

                    let n1 = right.cross(down);
                    let n2 = down.cross(left);
                    let n3 = left.cross(up);
                    let n4 = up.cross(right);

                    let normal = (n1 + n2 + n3 + n4).normalize();
                    vertices[center].normal = normal;
                }
            }
        }

        let vertex_buffer = renderer.create_vertex_buffer("terrain_vertex_buffer", &vertices);
        let index_buffer = renderer.create_index_buffer("terrain_index_buffer", &indices);
        let wireframe_index_buffer =
            renderer.create_index_buffer("terrain_wireframe_index_buffer", &wireframe_indices);

        let shader_module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
        );

        let pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("terrain", &shader_module)
                .bind_group_layout(camera_bind_group_layout)
                .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        let wireframe_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("terrain_wireframe", &shader_module)
                .vertex_entry("vertex_main_wireframe")
                .fragment_entry("fragment_main_wireframe")
                .bind_group_layout(camera_bind_group_layout)
                // .bind_group_layout(renderer.uniform_bind_group_layout())
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                })
                .disable_depth_buffer(),
        );

        Ok(Self {
            height_map_width: height_map.width,
            height_map_height: height_map.height,

            altitude_map_height_base,
            map_dx,
            map_dy,
            nominal_edge_size,

            bounds_min,
            bounds_max,

            vertex_buffer,

            index_buffer,
            index_count: indices.len() as u32,

            wireframe_index_buffer,
            wireframe_index_count: wireframe_indices.len() as u32,

            pipeline,
            wireframe_pipeline,

            terrain_texture_bind_group,

            draw_wireframe: false,
            draw_normals: false,

            vertices,
            #[cfg(feature = "load_normals")]
            normals_table,
        })
    }

    pub fn render_normals(&self) -> Vec<GizmoVertex> {
        const LENGTH: f32 = 100.0;

        let mut vertices = vec![];

        if !self.draw_normals {
            return vertices;
        }

        let width = self.height_map_width as usize;
        let height = self.height_map_height as usize;

        let color = Vec4::new(0.0, 1.0, 1.0, 1.0);

        for y in 0..height {
            for x in 0..width {
                let index = y * width + x;
                vertices.push(GizmoVertex::new(self.vertices[index].position, color));
                let n = self.vertices[index].position + self.vertices[index].normal * LENGTH;
                vertices.push(GizmoVertex::new(n, color));
            }
        }

        vertices
    }

    #[cfg(not(feature = "load_normals"))]
    pub fn render_normals_lookup(&self) -> Vec<GizmoVertex> {
        vec![]
    }

    #[cfg(feature = "load_normals")]
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

    pub fn render_frame(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        {
            let mut render_pass = frame.begin_basic_render_pass("terrain_render_pass", true);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_texture_bind_group, &[]);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        if self.draw_wireframe {
            let mut render_pass =
                frame.begin_basic_render_pass("terrain_wireframe_render_pass", false);

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
