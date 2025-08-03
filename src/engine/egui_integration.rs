use winit::event_loop::ActiveEventLoop;

use crate::engine::prelude::renderer;

pub struct EguiIntegration {
    egui: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl EguiIntegration {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let renderer = renderer();

        let egui = egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::default(),
            event_loop,
            None,
            None,
            None,
        );

        let egui_renderer =
            egui_wgpu::Renderer::new(&renderer.device, renderer.surface.format(), None, 1, false);

        Self {
            egui,
            egui_renderer,
        }
    }

    pub fn window_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.egui.on_window_event(window, event)
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run_ui: impl FnMut(&egui::Context),
    ) {
        let renderer = renderer();

        let raw_input = self.egui.take_egui_input(window);

        let full_output = self.egui.egui_ctx().run(raw_input, run_ui);

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output: _,
        } = full_output;

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: renderer.surface.size().to_array(),
            pixels_per_point,
        };

        self.egui.handle_platform_output(window, platform_output);

        for (id, ref image_delta) in textures_delta.set {
            self.egui_renderer
                .update_texture(&renderer.device, &renderer.queue, id, image_delta);
        }

        for ref id in textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let clipped_primitives = self
            .egui
            .egui_ctx()
            .tessellate(shapes, screen_descriptor.pixels_per_point);

        self.egui_renderer.update_buffers(
            &renderer.device,
            &renderer.queue,
            encoder,
            clipped_primitives.as_ref(),
            &screen_descriptor,
        );

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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

        self.egui_renderer.render(
            &mut render_pass.forget_lifetime(),
            clipped_primitives.as_ref(),
            &screen_descriptor,
        );
    }
}
