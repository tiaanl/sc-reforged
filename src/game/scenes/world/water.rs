use std::path::PathBuf;

use crate::{
    engine::prelude::*,
    game::asset_loader::{AssetError, AssetLoader},
};

use super::{GpuIndexedMesh, IndexedMesh, RenderPipelineConfig, Renderer, Vertex};
pub struct Water {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    mesh: GpuIndexedMesh,
}

impl Water {
    pub fn new(
        asset_loader: &AssetLoader,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        fog_bind_group_layout: &wgpu::BindGroupLayout,
        terrain_size: Vec2,
        water_level: f32,
    ) -> Result<Self, AssetError> {
        let module =
            shaders.create_shader(renderer, "water", include_str!("water.wgsl"), "water.wgsl");

        let pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("water", &module)
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

        let texture_bind_group =
            renderer.create_texture_bind_group("water", &water_texture, &sampler);

        let mut mesh = IndexedMesh::default();
        const TEXTURE_SCALE: f32 = 8.0;
        mesh.vertices.extend_from_slice(&[
            Vertex {
                position: Vec3::new(0.0, 0.0, water_level),
                normal: Vec3::Z,
                tex_coord: Vec2::ZERO,
            },
            Vertex {
                position: Vec3::new(terrain_size.x, 0.0, water_level),
                normal: Vec3::Z,
                tex_coord: Vec2::X * TEXTURE_SCALE,
            },
            Vertex {
                position: Vec3::new(terrain_size.x, terrain_size.y, water_level),
                normal: Vec3::Z,
                tex_coord: (Vec2::X + Vec2::Y) * TEXTURE_SCALE,
            },
            Vertex {
                position: Vec3::new(0.0, terrain_size.y, water_level),
                normal: Vec3::Z,
                tex_coord: Vec2::Y * TEXTURE_SCALE,
            },
        ]);
        mesh.indices = vec![0, 1, 2, 2, 3, 0];
        let mesh = mesh.to_gpu(renderer);

        Ok(Self {
            pipeline,
            texture_bind_group,
            mesh,
        })
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        fog_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame.begin_basic_render_pass("water", true);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);
        render_pass.set_bind_group(2, fog_bind_group, &[]);
        render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..1);
    }
}
