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
    pub mtf_name: Option<String>,
}

impl TryFrom<String> for Campaign {
    type Error = AssetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut campaign = Campaign::default();

        let mut config = ConfigFile::new(&value);

        while let Some(current) = config.current() {
            match current[0] {
                "SPECIFY_VIEW_INITIAL" => {
                    campaign.view_initial.from.x = current[1].parse().unwrap();
                    campaign.view_initial.from.y = current[2].parse().unwrap();
                    campaign.view_initial.to.x = current[3].parse().unwrap();
                    campaign.view_initial.to.y = current[4].parse().unwrap();
                }
                "SPECIFY_MTF" => {
                    campaign.mtf_name = Some(current[1].into());
                }
                _ => {}
            }
            config.next();
        }

        Ok(campaign)
    }
}
