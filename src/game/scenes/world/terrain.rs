use std::path::PathBuf;

use glam::UVec2;
use tracing::info;

use crate::{
    engine::prelude::*,
    game::{
        asset_loader::{AssetError, AssetLoader},
        camera::{BoundingBox, Camera, Frustum},
        config::{CampaignDef, TerrainMapping},
    },
};

use super::height_map::{GpuChunkMesh, HeightMap};

pub struct Chunk {
    distance_from_camera: f32,
    resolution: u32,
    visible: bool,
    bounding_box: BoundingBox,
    meshes: [GpuChunkMesh; 4],
}

impl Chunk {
    pub fn new(height_map: &HeightMap, chunk_offset: UVec2, renderer: &Renderer) -> Chunk {
        let min = chunk_offset * HeightMap::CHUNK_SIZE;
        let max = min + UVec2::new(HeightMap::CHUNK_SIZE + 1, HeightMap::CHUNK_SIZE + 1);

        Self {
            distance_from_camera: f32::MAX,
            resolution: HeightMap::MAX_RESOLUTION,
            visible: true,
            bounding_box: BoundingBox {
                min: height_map.position(min),
                max: height_map.position(max),
            },
            meshes: [0, 1, 2, 3].map(|res| height_map.generate_chunk(min, res).into_gpu(renderer)),
        }
    }

    pub fn mesh_at(&self, resolution: u32) -> &GpuChunkMesh {
        assert!(resolution <= HeightMap::MAX_RESOLUTION);
        &self.meshes[resolution as usize]
    }
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

    chunks: Vec<Chunk>,
}

impl Terrain {
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
        } = height_map.chunks();

        let mut chunks = Vec::with_capacity(chunks_x as usize * chunks_y as usize);
        for y in 0..chunks_y {
            for x in 0..chunks_x {
                chunks.push(Chunk::new(&height_map, UVec2 { x, y }, renderer));
            }
        }

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

        Ok(Self {
            height_map,

            max_view_distance: 13_300.0,

            pipeline,
            wireframe_pipeline,

            terrain_texture_bind_group,

            draw_wireframe: false,
            draw_normals: false,

            chunks,
        })
    }

    pub fn update(&mut self, camera: &Camera) {
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

            let res = distance / (self.max_view_distance / HeightMap::MAX_RESOLUTION as f32);
            let res = HeightMap::MAX_RESOLUTION - (res as u32).min(HeightMap::MAX_RESOLUTION);

            chunk.resolution = res;
        }
    }

    pub fn render_chunks(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        fog_bind_group: &wgpu::BindGroup,
    ) {
        {
            let mut render_pass = frame.begin_basic_render_pass("terrain_render_pass", true);

            render_pass.set_pipeline(&self.pipeline);

            for chunk in self.chunks.iter().filter(|chunk| chunk.visible) {
                let mesh = chunk.mesh_at(chunk.resolution);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
                render_pass.set_bind_group(1, camera_bind_group, &[]);
                render_pass.set_bind_group(2, fog_bind_group, &[]);
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        if self.draw_wireframe {
            let mut render_pass =
                frame.begin_basic_render_pass("terrain_wireframe_render_pass", false);

            render_pass.set_pipeline(&self.wireframe_pipeline);

            for chunk in self.chunks.iter().filter(|chunk| chunk.visible) {
                let mesh = chunk.mesh_at(chunk.resolution);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    mesh.wireframe_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.set_bind_group(0, &self.terrain_texture_bind_group, &[]);
                render_pass.set_bind_group(1, camera_bind_group, &[]);
                render_pass.set_bind_group(2, fog_bind_group, &[]);
                render_pass.draw_indexed(0..mesh.wireframe_index_count, 0, 0..1);
            }
        }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");

        ui.add(egui::widgets::DragValue::new(&mut self.max_view_distance).speed(10.0));

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
