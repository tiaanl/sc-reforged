use crate::{
    engine::assets::{Asset, AssetError},
    game::{
        assets::{Config, TextFile},
        config::ConfigFile,
    },
};

#[derive(Clone, Debug, Default)]
pub struct EmitterConfig {
    mi_subset: String,
    emitter_type_desired: String,
}

impl EmitterConfig {
    fn from_params(params: &[&str]) -> Self {
        // EMITTER_CONFIG	<MI_subset>	<emitter_type_desired>
        let mi_subset = params[0].to_string();
        let emitter_type_desired = params[1].to_string();
        Self {
            mi_subset,
            emitter_type_desired,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ClothingInfiltrationMod {
    clothing_type: String,
    day_infiltration_modifier: i32,
    night_infiltration_modifier: i32,
}

impl ClothingInfiltrationMod {
    fn from_params(params: &[&str]) -> Self {
        // CLOTHING_INFILTRATION_MOD	<Clothing_Type>	<day_infiltration_modifier> <night_infiltration_modifier>
        let clothing_type = params[0].to_string();
        let day_infiltration_modifier = params[1].parse().unwrap();
        let night_infiltration_modifier = params[1].parse().unwrap();
        Self {
            clothing_type,
            day_infiltration_modifier,
            night_infiltration_modifier,
        }
    }
}

/// Actions to be performed before (pre) or after (post) a campaign.
#[derive(Clone, Debug)]
pub enum Action {
    /// This is the amount of money given to the player.
    Reward {
        amount: u32,
    },
    /// Specifies a number of an explicit object to be stocked to the dealers
    /// inventory.
    DealerStockExplicit {
        object_name: String,
        count: u32,
    },
    /// Specifies a number of an explicit object to be stocked to the dealers
    /// inventory. (multiplayer only)
    MultiplayerDealerStockExplicit {
        object_name: String,
        count: u32,
    },

    /// Adds `count` random items to the dealer stock with a `modifier` chance
    /// modifier.  A positive `modifier` indicates more rare weapons will be
    /// stocked.  Eg: the formulae is, if, (1..100) < stocked_chance+`modifier`,
    /// for a particular item, that item will be successfully chosen.
    DealerStockRandom {
        count: u32,
        modifier: i32,
    },
    /// This allows you to set the dealer price markup; the markup will take
    /// effect on all currently existing dealer equipment, as well as all
    /// future equipment (unless the markup is reset)  So, a markup of 100
    /// will make equipment cost twice as much as that listed in the file
    /// (what the player will get for selling them).
    DealerSetMarkup {
        markup: u32,
    },
    /// Specifies an object to give `count` of to the shadow company inventory
    /// automatically.
    PlayerStockExplicit {
        object_name: String,
        count: u32,
    },
    /// Adds a specific merc to the roster.
    RosterMerc {
        merc_name: String,
    },
    // Addes a specific merc to the team list.
    HireMerc {
        merc_name: String,
    },
    /// Causes the dealer's inventory to be completely flushed of all items.
    DealerRemoveAll,
    /// Attempts to find the specific object in the player's item inventory; if
    /// it is not found in the shadow company inventory, the individual's
    /// inventories will be searched as well (needed for taking stuff like
    /// mutagen and lithium cores from the player after a mission).
    PlayerRemoveExplicit {
        object_name: String,
    },
}

impl Action {
    fn from_params(params: &[&str]) -> Self {
        match params[0] {
            "REWARD" => Self::Reward {
                amount: params[1].parse().unwrap(),
            },
            "DEALER_STOCK_EXPLICIT" => Self::DealerStockExplicit {
                object_name: params[1].to_string(),
                count: params[2].parse().unwrap(),
            },
            "DEALER_STOCK_RANDOM" => Self::DealerStockRandom {
                count: params[1].parse().unwrap(),
                modifier: params[2].parse().unwrap(),
            },
            "ROSTER_MERC" => Self::RosterMerc {
                merc_name: params[1].to_string(),
            },
            "HIRE_MERC" => Self::HireMerc {
                merc_name: params[1].to_string(),
            },
            "DEALER_REMOVE_ALL" => Self::DealerRemoveAll,
            "DEALER_SET_MARKUP" => Self::DealerSetMarkup {
                markup: params[1].parse().unwrap(),
            },
            "PLAYER_STOCK_EXPLICIT" => Self::PlayerStockExplicit {
                object_name: params[1].to_string(),
                count: params[2].parse().unwrap(),
            },
            "MPM_DEALER_STOCK_EXPLICIT" => Self::MultiplayerDealerStockExplicit {
                object_name: params[1].to_string(),
                count: params[2].parse().unwrap(),
            },
            "PLAYER_REMOVE_EXPLICIT" => Self::PlayerRemoveExplicit {
                object_name: params[1].to_string(),
            },
            _ => panic!("Invalid action! {params:?}"),
        }
    }
}

#[derive(Clone, Debug)]
enum PreconditionType {
    MustTake {
        param_id: String,
        help_id_on_failure: String,
        help_id_on_failure_in_multiplayer: String,
    },
}

impl PreconditionType {
    fn from_params(params: &[&str]) -> Self {
        match params[0] {
            "MUST_TAKE" => Self::MustTake {
                param_id: params[1].to_string(),
                help_id_on_failure: params[2].to_string(),
                help_id_on_failure_in_multiplayer: params[3].to_string(),
            },
            _ => panic!("Invalid precondition type. {params:?}"),
        }
    }
}

#[derive(Clone, Debug)]
struct Precondition {
    name: String,
    typ: PreconditionType,
}

impl Precondition {
    fn from_params(params: &[&str]) -> Self {
        Self {
            name: params[0].to_string(),
            typ: PreconditionType::from_params(&params[1..]),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CampaignDef {
    pub base_name: String,
    pub title: String,
    pub multiplayer_active: bool,
    pub exclude_from_campaign_tree: bool,
    pub disable_help_tips: bool,
    pub skip_team_equipment_validation: bool,
    pub playtest_funds: u32,
    pub multiplayer_funds: [u32; 3],
    pub cutscene: String,
    pub disable_team_and_equipping: String,
    pub lighting_threshholds: [u32; 2],
    pub enemy_grenade_use_chance: u32,
    pub alarm_audio: String,

    emitter_config: EmitterConfig,
    clothing_infiltration_mod: Vec<ClothingInfiltrationMod>,
    pre_actions: Vec<Action>,
    post_actions: Vec<Action>,
    precondition: Vec<Precondition>,
}

pub fn read_compaign_defs(data: &str) -> Vec<CampaignDef> {
    let mut campaigns = vec![];

    let mut config = ConfigFile::new(data);

    struct State(Option<CampaignDef>);
    impl State {
        fn with_campaign(&mut self) -> &mut CampaignDef {
            let Some(campaign_def) = &mut self.0 else {
                panic!("No current campaign!");
            };

            campaign_def
        }
    }

    let mut state = State(None);

    while let Some(current) = config.current() {
        match current[0] {
            "CAMPAIGN_DEF" => match state.0 {
                None => state = State(Some(CampaignDef::default())),
                Some(ref mut campaign_def) => {
                    campaigns.push(std::mem::take(campaign_def));
                }
            },
            "BASENAME" => state.with_campaign().base_name = current[1].to_string(),
            "TITLE" => state.with_campaign().title = current[1].to_string(),
            "MULTIPLAYER_ACTIVE" => state.with_campaign().multiplayer_active = true,
            "EXCLUDE_FROM_CAMPAIGN_TREE" => state.with_campaign().exclude_from_campaign_tree = true,
            "DISABLE_HELP_TIPS" => state.with_campaign().disable_help_tips = true,
            "SKIP_TEAM_EQUIPMENT_VALIDATION" => {
                state.with_campaign().skip_team_equipment_validation = true
            }
            "PLAYTEST_FUNDS" => {
                state.with_campaign().playtest_funds = current[1].parse().unwrap();
            }
            "MULTIPLAYER_FUNDS" => {
                state.with_campaign().multiplayer_funds = [
                    current[1].parse().unwrap(),
                    current[2].parse().unwrap(),
                    current[3].parse().unwrap(),
                ]
            }
            "CUTSCENE" => state.with_campaign().cutscene = current[1].to_string(),
            "DISABLE_TEAM_AND_EQUIPPING" => {
                state.with_campaign().disable_team_and_equipping = current[1].to_string()
            }
            "LIGHTING_THRESHHOLDS" => {
                state.with_campaign().lighting_threshholds =
                    [current[1].parse().unwrap(), current[2].parse().unwrap()]
            }
            "ENEMY_GRENADE_USE_CHANCE" => {
                state.with_campaign().enemy_grenade_use_chance = current[1].parse().unwrap()
            }
            "ALARM_AUDIO" => state.with_campaign().alarm_audio = current[1].to_string(),
            "EMITTER_CONFIG" => {
                state.with_campaign().emitter_config = EmitterConfig::from_params(&current[1..])
            }
            "CLOTHING_INFILTRATION_MOD" => state
                .with_campaign()
                .clothing_infiltration_mod
                .push(ClothingInfiltrationMod::from_params(&current[1..])),
            "PRE_ACTION" => state
                .with_campaign()
                .pre_actions
                .push(Action::from_params(&current[1..])),
            "POST_ACTION" => state
                .with_campaign()
                .post_actions
                .push(Action::from_params(&current[1..])),
            "PRECONDITION" => state
                .with_campaign()
                .precondition
                .push(Precondition::from_params(&current[1..])),

            _ => panic!("Invalid config line. {current:?}"),
        }

        config.next();
    }

    if let Some(campaign_def) = state.0 {
        campaigns.push(campaign_def);
    }

    campaigns
}

pub struct CampaignDefs {
    pub campaigns: Vec<CampaignDef>,
}

impl Config for CampaignDefs {
    fn from_string(str: &str) -> Result<Self, AssetError> {
        let mut config = ConfigFile::new(str);

        let mut campaigns = vec![];

        struct State(Option<CampaignDef>);
        impl State {
            fn with_campaign(&mut self) -> &mut CampaignDef {
                let Some(campaign_def) = &mut self.0 else {
                    panic!("No current campaign!");
                };

                campaign_def
            }
        }

        let mut state = State(None);

        while let Some(current) = config.current() {
            match current[0] {
                "CAMPAIGN_DEF" => match state.0 {
                    None => state = State(Some(CampaignDef::default())),
                    Some(ref mut campaign_def) => {
                        campaigns.push(std::mem::take(campaign_def));
                    }
                },
                "BASENAME" => state.with_campaign().base_name = current[1].to_string(),
                "TITLE" => state.with_campaign().title = current[1].to_string(),
                "MULTIPLAYER_ACTIVE" => state.with_campaign().multiplayer_active = true,
                "EXCLUDE_FROM_CAMPAIGN_TREE" => {
                    state.with_campaign().exclude_from_campaign_tree = true
                }
                "DISABLE_HELP_TIPS" => state.with_campaign().disable_help_tips = true,
                "SKIP_TEAM_EQUIPMENT_VALIDATION" => {
                    state.with_campaign().skip_team_equipment_validation = true
                }
                "PLAYTEST_FUNDS" => {
                    state.with_campaign().playtest_funds = current[1].parse().unwrap();
                }
                "MULTIPLAYER_FUNDS" => {
                    state.with_campaign().multiplayer_funds = [
                        current[1].parse().unwrap(),
                        current[2].parse().unwrap(),
                        current[3].parse().unwrap(),
                    ]
                }
                "CUTSCENE" => state.with_campaign().cutscene = current[1].to_string(),
                "DISABLE_TEAM_AND_EQUIPPING" => {
                    state.with_campaign().disable_team_and_equipping = current[1].to_string()
                }
                "LIGHTING_THRESHHOLDS" => {
                    state.with_campaign().lighting_threshholds =
                        [current[1].parse().unwrap(), current[2].parse().unwrap()]
                }
                "ENEMY_GRENADE_USE_CHANCE" => {
                    state.with_campaign().enemy_grenade_use_chance = current[1].parse().unwrap()
                }
                "ALARM_AUDIO" => state.with_campaign().alarm_audio = current[1].to_string(),
                "EMITTER_CONFIG" => {
                    state.with_campaign().emitter_config = EmitterConfig::from_params(&current[1..])
                }
                "CLOTHING_INFILTRATION_MOD" => state
                    .with_campaign()
                    .clothing_infiltration_mod
                    .push(ClothingInfiltrationMod::from_params(&current[1..])),
                "PRE_ACTION" => state
                    .with_campaign()
                    .pre_actions
                    .push(Action::from_params(&current[1..])),
                "POST_ACTION" => state
                    .with_campaign()
                    .post_actions
                    .push(Action::from_params(&current[1..])),
                "PRECONDITION" => state
                    .with_campaign()
                    .precondition
                    .push(Precondition::from_params(&current[1..])),

                _ => panic!("Invalid config line. {current:?}"),
            }

            config.next();
        }

        if let Some(campaign_def) = state.0 {
            campaigns.push(campaign_def);
        }

        Ok(Self { campaigns })
    }
}
