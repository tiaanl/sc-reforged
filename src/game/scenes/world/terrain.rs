use glam::{UVec2, Vec2, Vec3, Vec3Swizzles, Vec4};
use tracing::info;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::*},
    game::{
        asset_loader::{AssetError, AssetLoader},
        config::{CampaignDef, ConfigFile, TerrainMapping},
    },
};

use super::height_map::Chunk;

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

    chunks: Vec<Chunk>,
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
        } = height_map.chunks();

        let mut chunks = Vec::with_capacity(chunks_x as usize * chunks_y as usize);
        for y in 0..chunks_y {
            for x in 0..chunks_x {
                chunks.push(height_map.new_chunk(renderer, x, y));
            }
        }

        // let chunks = vec![
        //     height_map.new_chunk(renderer, 0, 0),
        //     height_map.new_chunk(renderer, 1, 1),
        //     height_map.new_chunk(renderer, 2, 2),
        // ];

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            map_dx, map_dy, height_map.size_x, height_map.size_y,
        );

        /*
        // Generate an array for each edge.
        let x_sides = (0..height_map.size_x)
            .map(|v| v as f32 * nominal_edge_size)
            .collect::<Vec<_>>();
        let y_sides = (0..height_map.size_y)
            .map(|v| v as f32 * nominal_edge_size)
            .collect::<Vec<_>>();
        */

        let bounds_min = height_map.position(0, 0).xy().map(|v| v - 2500.0);
        let bounds_max = height_map
            .position(height_map.size_x - 1, height_map.size_y - 1)
            .xy()
            .map(|v| v + 2500.0);

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
                (0..(height_map.size_x as usize * height_map.size_y as usize))
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

        macro_rules! index {
            ($x:expr,$y:expr) => {{
                (($y as u32) * height_map.size_y + ($x as u32)) as u32
            }};
        }

        #[cfg(feature = "load_normals")]
        {
            let value =
                normals_lookup[index!(height_map.size_x / 2, height_map.size_y / 2) as usize];
            info!("value: {}", value);
        }

        let (mut vertices, indices, wireframe_indices) = {
            let mut vertices =
                Vec::with_capacity(height_map.size_x as usize * height_map.size_y as usize);
            let mut indices = Vec::with_capacity(
                (height_map.size_x as usize - 1) * (height_map.size_y as usize - 1) * 6,
            );
            let mut wireframe_indices =
                Vec::with_capacity((height_map.size_x as usize) * (height_map.size_y as usize) * 4);

            for y in 0..height_map.size_y {
                for x in 0..height_map.size_x {
                    #[cfg(feature = "load_normals")]
                    let normal = normals_table[normals_lookup[index!(x, y) as usize] as usize];

                    #[cfg(not(feature = "load_normals"))]
                    let normal = Vec3::Y;

                    vertices.push(Vertex {
                        position: height_map.position(x, y),
                        normal,
                        tex_coord: Vec2::new(
                            x as f32 / (height_map.size_x - 1) as f32,
                            y as f32 / (height_map.size_y - 1) as f32,
                        ),
                    });
                }
            }

            for y in 0..(height_map.size_y - 1) {
                for x in 0..(height_map.size_x - 1) {
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

                wireframe_indices.push(index!(height_map.size_x - 1, y));
                wireframe_indices.push(index!(height_map.size_x - 1, y + 1));
            }
            for x in 0..(height_map.size_x - 1) {
                wireframe_indices.push(index!(x, height_map.size_y - 1));
                wireframe_indices.push(index!(x + 1, height_map.size_y - 1));
            }

            (vertices, indices, wireframe_indices)
        };

        // Calculate the normals of each vertex of the terrain.
        {
            let (width, height) = (height_map.size_x as usize, height_map.size_y as usize);
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
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                })
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
            height_map_width: height_map.size_x,
            height_map_height: height_map.size_y,

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

            chunks,
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

            /*
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_texture_bind_group, &[]);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
            */

            for chunk in self.chunks.iter() {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, chunk.mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(chunk.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.terrain_texture_bind_group, &[]);
                render_pass.draw_indexed(0..chunk.mesh.index_count, 0, 0..1);
            }
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
