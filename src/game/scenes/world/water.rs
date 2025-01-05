use std::{path::PathBuf, sync::Arc};

use wgpu::util::DeviceExt;

use crate::{
    engine::prelude::*,
    game::asset_loader::{AssetError, AssetLoader},
};

use super::{GpuIndexedMesh, IndexedMesh, RenderPipelineConfig, Renderer, Vertex};
pub struct Water {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    mesh: GpuIndexedMesh,

    water_uniform_buffer: wgpu::Buffer,
    water_bind_group_layout: wgpu::BindGroupLayout,
    water_bind_group: wgpu::BindGroup,

    gpu_water: GpuWater,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct GpuWater {
    fade_start: f32,
    fade_end: f32,
    alpha: f32,
    _padding2: f32,
}

impl Water {
    pub fn new(
        asset_loader: &AssetLoader,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        terrain_size: Vec2,
        water_level: f32,
    ) -> Result<Self, AssetError> {
        let gpu_water = GpuWater {
            fade_start: 0.0,
            fade_end: 1.0,
            alpha: 0.8,
            _padding2: 0.0,
        };

        let water_uniform_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("water"),
                    contents: bytemuck::cast_slice(&[gpu_water]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let water_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("water"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let water_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("water"),
                layout: &water_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: water_uniform_buffer.as_entire_binding(),
                }],
            });

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
                .disable_depth_buffer()
                .blend_state(wgpu::BlendState::ALPHA_BLENDING)
                .bind_group_layout(renderer.texture_bind_group_layout())
                .bind_group_layout(camera_bind_group_layout)
                .bind_group_layout(&renderer.depth_buffer.bind_group_layout)
                .bind_group_layout(&water_bind_group_layout),
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
            water_uniform_buffer,
            water_bind_group_layout,
            water_bind_group,

            gpu_water,
        })
    }

    pub fn render(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        frame.queue.write_buffer(
            &self.water_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.gpu_water]),
        );

        let depth_buffer = Arc::clone(&frame.depth_buffer);
        let mut render_pass = frame.begin_basic_render_pass("water", false);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);
        render_pass.set_bind_group(2, &depth_buffer.bind_group, &[]);
        render_pass.set_bind_group(3, &self.water_bind_group, &[]);
        render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..1);
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Water");
        ui.horizontal(|ui| {
            ui.label("Fade Start");
            ui.add(egui::widgets::DragValue::new(&mut self.gpu_water.fade_start).speed(0.1));
        });
        ui.horizontal(|ui| {
            ui.label("Fade End");
            ui.add(egui::widgets::DragValue::new(&mut self.gpu_water.fade_end).speed(0.1));
        });

        ui.horizontal(|ui| {
            ui.label("Alpha");
            ui.add(
                egui::widgets::Slider::new(&mut self.gpu_water.alpha, 0.0..=1.0)
                    .drag_value_speed(0.01),
            );
        });
    }
}
