use egui::Widget;
use winit::event_loop::ActiveEventLoop;

pub struct EguiIntegration {
    device: wgpu::Device,
    queue: wgpu::Queue,
    egui: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl EguiIntegration {
    pub fn new(
        event_loop: &ActiveEventLoop,
        device: wgpu::Device,
        queue: wgpu::Queue,
        output_color_format: wgpu::TextureFormat,
    ) -> Self {
        let egui = egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::default(),
            event_loop,
            None,
            None,
            None,
        );

        egui.egui_ctx().all_styles_mut(|style| {
            style.text_styles.insert(
                egui::TextStyle::Name("h1".into()),
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Name("h2".into()),
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Name("h3".into()),
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
        });

        let egui_renderer = egui_wgpu::Renderer::new(&device, output_color_format, None, 1, false);

        Self {
            device,
            queue,
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
        let raw_input = self.egui.take_egui_input(window);

        let full_output = self.egui.egui_ctx().run(raw_input, run_ui);

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output: _,
        } = full_output;

        let winit::dpi::PhysicalSize { width, height } = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        self.egui.handle_platform_output(window, platform_output);

        for (id, ref image_delta) in textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, id, image_delta);
        }

        for ref id in textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let clipped_primitives = self
            .egui
            .egui_ctx()
            .tessellate(shapes, screen_descriptor.pixels_per_point);

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
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

pub trait UiExt {
    fn h1(&mut self, text: impl Into<String>) -> egui::Response;
    fn h2(&mut self, text: impl Into<String>) -> egui::Response;
    fn h3(&mut self, text: impl Into<String>) -> egui::Response;
}

impl UiExt for egui::Ui {
    #[inline]
    fn h1(&mut self, text: impl Into<String>) -> egui::Response {
        egui::Label::new(
            egui::RichText::new(text)
                .text_style(egui::TextStyle::Name("h1".into()))
                .strong(),
        )
        .ui(self)
    }

    #[inline]
    fn h2(&mut self, text: impl Into<String>) -> egui::Response {
        egui::Label::new(
            egui::RichText::new(text)
                .text_style(egui::TextStyle::Name("h2".into()))
                .strong(),
        )
        .ui(self)
    }

    #[inline]
    fn h3(&mut self, text: impl Into<String>) -> egui::Response {
        egui::Label::new(
            egui::RichText::new(text)
                .text_style(egui::TextStyle::Name("h3".into()))
                .strong(),
        )
        .ui(self)
    }
}
