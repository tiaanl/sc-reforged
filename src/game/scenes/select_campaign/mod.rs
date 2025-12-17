use crate::{
    engine::{
        assets::AssetError,
        context::EngineContext,
        input::InputState,
        renderer::{Frame, Renderer},
        scene::{LoadContext, Scene, SceneLoader},
    },
    game::{config::CampaignDefs, data_dir::data_dir, scenes::world::WorldSceneLoader},
};

pub struct SelectCampaignScene {
    engine_context: EngineContext,
    campaign_defs: CampaignDefs,
}

impl SelectCampaignScene {
    pub fn new(engine_context: EngineContext) -> Self {
        let campaign_defs = data_dir().load_campaign_defs().unwrap();
        Self {
            engine_context,
            campaign_defs,
        }
    }
}

impl Scene for SelectCampaignScene {
    fn resize(&mut self, _size: glam::UVec2) {}

    fn update(&mut self, _delta_time: f32, _input: &InputState) {}

    fn render(&mut self, _renderer: &Renderer, frame: &mut Frame) {
        let _render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("loading_scene_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
    }

    fn debug_panel(&mut self, egui: &egui::Context, _frame_index: u64) {
        egui::Window::new("Select campaign").show(egui, |ui| {
            let mut load_campaign = None;
            for campaign in self.campaign_defs.campaigns.iter() {
                if ui.button(&campaign.title).clicked() {
                    load_campaign = Some(campaign.base_name.clone());
                }
            }

            if let Some(campaign_name) = load_campaign {
                self.engine_context.switch_scene(WorldSceneLoader {
                    campaign_name: Some(campaign_name),
                });
            }
        });
    }
}

pub struct SelectCampaignSceneLoader;

impl SceneLoader for SelectCampaignSceneLoader {
    fn load(
        self,
        engine_context: EngineContext,
        _load_context: &LoadContext,
    ) -> Result<Box<dyn Scene>, AssetError> {
        Ok(Box::new(SelectCampaignScene::new(engine_context)))
    }
}
