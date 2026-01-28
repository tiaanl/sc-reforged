use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{GeometryBuffer, RenderPipeline, RenderTargets},
        },
    },
    wgsl_shader,
};

use super::RenderBindings;

pub struct Compositor {
    pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new(renderer: &Renderer, render_targets: &RenderTargets) -> Self {
        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("compositor"));

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compositor_pipeline_layout"),
                bind_group_layouts: &[&render_targets.geometry_buffer.bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("compositor_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: render_targets.surface_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }
}

impl RenderPipeline for Compositor {
    fn prepare(
        &mut self,
        _assets: &AssetReader,
        _renderer: &Renderer,
        _bindings: &mut RenderBindings,
        _snapshot: &RenderSnapshot,
    ) {
        // No preparation required.
    }

    fn queue(
        &self,
        _bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        _snapshot: &RenderSnapshot,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compositor_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, geometry_buffer.bind_group(), &[]);
        render_pass.draw(0..3, 0..1);
    }
}
