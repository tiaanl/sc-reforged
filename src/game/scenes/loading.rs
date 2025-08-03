use std::path::PathBuf;

use crate::{engine::prelude::*, game::data_dir::data_dir};

pub struct LoadingScene {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl LoadingScene {
    pub fn new(renderer: &Renderer) -> Self {
        let image = data_dir()
            .load_image(
                PathBuf::from("textures")
                    .join("interface")
                    .join("loadscr2.jpg"),
            )
            .unwrap();
        let texture_view =
            renderer.create_texture_view("texture: textures/interface/loadscr2.jpg", &image.data);

        let sampler = renderer.create_sampler(
            "sampler: textures/interface/loadscr2.jpg",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let bind_group =
            renderer.create_texture_bind_group("loading_texture", &texture_view, &sampler);

        let shader_module =
            renderer.create_shader_module("loading_scene", include_str!("loading.wgsl"));

        let pipeline = renderer
            .build_render_pipeline::<()>("loading", &shader_module)
            .binding(renderer.texture_bind_group_layout())
            .build();

        Self {
            pipeline,
            bind_group,
        }
    }
}

impl Scene for LoadingScene {
    fn render(&mut self, frame: &mut Frame) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("loading_scene_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}
