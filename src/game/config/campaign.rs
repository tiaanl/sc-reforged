use glam::Vec2;

use crate::game::asset_loader::AssetError;

use super::ConfigFile;

#[derive(Default)]
pub struct ViewInitial {
    pub from: Vec2,
    pub to: Vec2,
}

#[derive(Default)]
pub struct Campaign {
    pub view_initial: ViewInitial,
}

pub fn read_campaign(config: &mut ConfigFile) -> Result<Campaign, AssetError> {
    let mut campaign = Campaign::default();

    while let Some(current) = config.current() {
        match current[0] {
            "SPECIFY_VIEW_INITIAL" => {
                campaign.view_initial.from.x = current[1].parse().unwrap();
                campaign.view_initial.from.y = current[2].parse().unwrap();
                campaign.view_initial.to.x = current[3].parse().unwrap();
                campaign.view_initial.to.y = current[4].parse().unwrap();
            }
            _ => {}
        }
        config.next();
    }

    Ok(campaign)
}
