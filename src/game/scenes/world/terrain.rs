use glam::{UVec2, Vec3Swizzles};
use tracing::info;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        config::{CampaignDef, ConfigFile, TerrainMapping},
    },
};

use super::height_map::{Chunk, HeightMap, Resolution};

pub struct Terrain {
    height_map: HeightMap,

    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,

    terrain_texture_bind_group: wgpu::BindGroup,

    draw_wireframe: bool,
    draw_normals: bool,

    chunks: Vec<Chunk>,
    resolution: Resolution,
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
            map_dx, map_dy, height_map.size.x, height_map.size.y,
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

        let bounds_min = height_map.position(UVec2::ZERO).xy().map(|v| v - 2500.0);
        let bounds_max = height_map
            .position(height_map.size - UVec2::ONE)
            .xy()
            .map(|v| v + 2500.0);

        tracing::info!(
            "Terrain bounds: ({}, {}) - ({}, {})",
            bounds_min.x,
            bounds_min.y,
            bounds_max.x,
            bounds_max.y
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
            height_map,

            pipeline,
            wireframe_pipeline,

            terrain_texture_bind_group,

            draw_wireframe: false,
            draw_normals: false,

            chunks,
            resolution: Resolution::High,
        })
    }

    pub fn update(&mut self, _delta_time: f32) {}

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");

        ui.radio_value(&mut self.resolution, Resolution::High, "High");
        ui.radio_value(&mut self.resolution, Resolution::Medium, "Medium");
        ui.radio_value(&mut self.resolution, Resolution::Low, "Low");
        ui.radio_value(&mut self.resolution, Resolution::Terrible, "Terrible");

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

    pub fn render_frame(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        {
            let mut render_pass = frame.begin_basic_render_pass("terrain_render_pass", true);

            render_pass.set_pipeline(&self.pipeline);

            for chunk in self.chunks.iter() {
                let mesh = chunk.mesh_at(self.resolution);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.terrain_texture_bind_group, &[]);
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        if self.draw_wireframe {
            let mut render_pass =
                frame.begin_basic_render_pass("terrain_wireframe_render_pass", false);

            render_pass.set_pipeline(&self.wireframe_pipeline);

            for chunk in self.chunks.iter() {
                let mesh = chunk.mesh_at(self.resolution);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    mesh.wireframe_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.draw_indexed(0..mesh.wireframe_index_count, 0, 0..1);
            }
        }
    }
}
