pub struct ClearRenderTargets;

impl super::System for ClearRenderTargets {
    fn queue(&mut self, context: &mut super::QueueContext) {
        let fog_color = wgpu::Color {
            r: context.render_world.camera_env.fog_color[0] as f64,
            g: context.render_world.camera_env.fog_color[1] as f64,
            b: context.render_world.camera_env.fog_color[2] as f64,
            a: 1.0,
        };

        let _ = context
            .frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_surface"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &context.frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(fog_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
    }
}
