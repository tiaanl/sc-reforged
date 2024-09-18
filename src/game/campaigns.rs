#![allow(dead_code)]

#[derive(Debug, Default)]
pub struct EmitterConfig {
    name: String,
    value: String,
}

#[derive(Debug, Default)]
pub struct ClothingInfiltrationMod {
    title: String,
    v1: i32,
    v2: i32,
}

#[derive(Debug, Default)]
pub struct Action {
    key: String,
    value: Option<String>,
}

#[derive(Debug, Default)]
pub struct CampaignDef {
    base_name: String,
    title: String,
    multiplayer_active: bool,
    exclude_from_campaign_tree: bool,
    disable_help_tips: bool,
    skip_team_equipment_validation: bool,
    playtest_funds: u32,
    multiplayer_funds: [u32; 3],
    cutscene: String,
    disable_team_and_equipping: String,
    lighting_threshholds: [u32; 2],
    enemy_grenade_use_chance: u32,
    alarm_audio: String,

    emitter_config: EmitterConfig,
    clothing_infiltration_mod: Vec<ClothingInfiltrationMod>,
    pre_actions: Vec<Action>,
    post_actions: Vec<Action>,
    precondition: Vec<Vec<String>>,
}

pub fn read_compaign_defs(data: &str) -> Vec<CampaignDef> {
    let mut campaigns = vec![];

    for line in data
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with(';'))
    {
        let params = line.split_whitespace().collect::<Vec<_>>();

        match params[0].to_lowercase().as_str() {
            "campaign_def" => campaigns.push(CampaignDef::default()),
            "basename" => {
                if let Some(last) = campaigns.last_mut() {
                    last.base_name = params[1].to_string();
                }
            }
            "title" => {
                if let Some(last) = campaigns.last_mut() {
                    last.title = params[1].to_string();
                }
            }
            "multiplayer_active" => {
                if let Some(last) = campaigns.last_mut() {
                    last.multiplayer_active = true;
                }
            }
            "exclude_from_campaign_tree" => {
                if let Some(last) = campaigns.last_mut() {
                    last.exclude_from_campaign_tree = true;
                }
            }
            "disable_help_tips" => {
                if let Some(last) = campaigns.last_mut() {
                    last.disable_help_tips = true;
                }
            }
            "skip_team_equipment_validation" => {
                if let Some(last) = campaigns.last_mut() {
                    last.skip_team_equipment_validation = true;
                }
            }
            "playtest_funds" => {
                if let Some(last) = campaigns.last_mut() {
                    last.playtest_funds = params[1].parse().unwrap();
                }
            }
            "multiplayer_funds" => {
                if let Some(last) = campaigns.last_mut() {
                    last.multiplayer_funds = [
                        params[1].parse().unwrap(),
                        params[2].parse().unwrap(),
                        params[3].parse().unwrap(),
                    ];
                }
            }
            "cutscene" => {
                if let Some(last) = campaigns.last_mut() {
                    last.cutscene = params[1].to_string();
                }
            }
            "disable_team_and_equipping" => {
                if let Some(last) = campaigns.last_mut() {
                    last.disable_team_and_equipping = params[1].to_string();
                }
            }
            "lighting_threshholds" => {
                if let Some(last) = campaigns.last_mut() {
                    last.lighting_threshholds =
                        [params[1].parse().unwrap(), params[2].parse().unwrap()];
                }
            }
            "enemy_grenade_use_chance" => {
                if let Some(last) = campaigns.last_mut() {
                    last.enemy_grenade_use_chance = params[1].parse().unwrap();
                }
            }
            "alarm_audio" => {
                if let Some(last) = campaigns.last_mut() {
                    last.alarm_audio = params[1].to_string();
                }
            }
            "emitter_config" => {
                if let Some(last) = campaigns.last_mut() {
                    last.emitter_config = EmitterConfig {
                        name: params[1].to_string(),
                        value: params[2].to_string(),
                    };
                }
            }
            "clothing_infiltration_mod" => {
                if let Some(last) = campaigns.last_mut() {
                    last.clothing_infiltration_mod
                        .push(ClothingInfiltrationMod {
                            title: params[1].to_string(),
                            v1: params[2].parse().unwrap(),
                            v2: params[3].parse().unwrap(),
                        });
                }
            }
            "pre_action" => {
                if let Some(last) = campaigns.last_mut() {
                    last.pre_actions.push(Action {
                        key: params[1].to_string(),
                        value: params.get(2).map(|s| s.to_string()),
                    });
                }
            }
            "post_action" => {
                if let Some(last) = campaigns.last_mut() {
                    last.post_actions.push(Action {
                        key: params[1].to_string(),
                        value: params.get(2).map(|s| s.to_string()),
                    });
                }
            }
            "precondition" => {
                if let Some(last) = campaigns.last_mut() {
                    last.precondition
                        .push(params[1..].iter().map(|s| s.to_string()).collect());
                }
            }

            _ => unreachable!("invalid entry: {:?}", params),
        }
    }

    campaigns
}
