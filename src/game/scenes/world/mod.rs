use crate::{
    engine::{assets::Assets, renderer::Renderer, scene::Scene},
    game::config::CampaignDef,
};
use terrain::*;

mod terrain;

/// The [Scene] that renders the ingame world view.

pub struct WorldScene {
    campaign_def: CampaignDef,
    terrain: Terrain,
}

impl WorldScene {
    pub fn new(_assets: &Assets, _renderer: &Renderer, campaign_def: CampaignDef) -> Self {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);
        Self {
            campaign_def,
            terrain: Terrain::default(),
        }
    }
}

impl Scene for WorldScene {
    fn update(&mut self, _delta_time: f32) {}

    fn render(
        &self,
        _renderer: &crate::engine::renderer::Renderer,
        _encoder: &mut wgpu::CommandEncoder,
        _output: &wgpu::TextureView,
    ) {
    }
}
