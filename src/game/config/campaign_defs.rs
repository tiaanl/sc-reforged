#![allow(dead_code)]

use crate::game::config::parser::{ConfigLine, ConfigLines};

#[derive(Clone, Debug, Default)]
pub struct EmitterConfig {
    mi_subset: String,
    emitter_type_desired: String,
}

impl From<ConfigLine> for EmitterConfig {
    fn from(value: ConfigLine) -> Self {
        // EMITTER_CONFIG	<MI_subset>	<emitter_type_desired>
        Self {
            mi_subset: value.param(0),
            emitter_type_desired: value.param(1),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ClothingInfiltrationMod {
    clothing_type: String,
    day_infiltration_modifier: i32,
    night_infiltration_modifier: i32,
}

impl From<ConfigLine> for ClothingInfiltrationMod {
    fn from(value: ConfigLine) -> Self {
        // CLOTHING_INFILTRATION_MOD	<Clothing_Type>	<day_infiltration_modifier> <night_infiltration_modifier>
        Self {
            clothing_type: value.param(0),
            day_infiltration_modifier: value.param(1),
            night_infiltration_modifier: value.param(2),
        }
    }
}

/// Actions to be performed before (pre) or after (post) a campaign.
#[derive(Clone, Debug)]
pub enum Action {
    /// This is the amount of money given to the player.
    Reward {
        amount: i32,
    },
    /// Specifies a number of an explicit object to be stocked to the dealers
    /// inventory.
    DealerStockExplicit {
        object_name: String,
        count: i32,
    },
    /// Specifies a number of an explicit object to be stocked to the dealers
    /// inventory. (multiplayer only)
    MultiplayerDealerStockExplicit {
        object_name: String,
        count: i32,
    },

    /// Adds `count` random items to the dealer stock with a `modifier` chance
    /// modifier.  A positive `modifier` indicates more rare weapons will be
    /// stocked.  Eg: the formulae is, if, (1..100) < stocked_chance+`modifier`,
    /// for a particular item, that item will be successfully chosen.
    DealerStockRandom {
        count: i32,
        modifier: i32,
    },
    /// This allows you to set the dealer price markup; the markup will take
    /// effect on all currently existing dealer equipment, as well as all
    /// future equipment (unless the markup is reset)  So, a markup of 100
    /// will make equipment cost twice as much as that listed in the file
    /// (what the player will get for selling them).
    DealerSetMarkup {
        markup: i32,
    },
    /// Specifies an object to give `count` of to the shadow company inventory
    /// automatically.
    PlayerStockExplicit {
        object_name: String,
        count: i32,
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

impl From<ConfigLine> for Action {
    fn from(value: ConfigLine) -> Self {
        match value.string(0).as_str() {
            "REWARD" => Self::Reward {
                amount: value.param(1),
            },
            "DEALER_STOCK_EXPLICIT" => Self::DealerStockExplicit {
                object_name: value.param(1),
                count: value.param(2),
            },
            "DEALER_STOCK_RANDOM" => Self::DealerStockRandom {
                count: value.param(1),
                modifier: value.param(2),
            },
            "ROSTER_MERC" => Self::RosterMerc {
                merc_name: value.param(1),
            },
            "HIRE_MERC" => Self::HireMerc {
                merc_name: value.param(1),
            },
            "DEALER_REMOVE_ALL" => Self::DealerRemoveAll,
            "DEALER_SET_MARKUP" => Self::DealerSetMarkup {
                markup: value.param(1),
            },
            "PLAYER_STOCK_EXPLICIT" => Self::PlayerStockExplicit {
                object_name: value.param(1),
                count: value.param(2),
            },
            "MPM_DEALER_STOCK_EXPLICIT" => Self::MultiplayerDealerStockExplicit {
                object_name: value.param(1),
                count: value.param(2),
            },
            "PLAYER_REMOVE_EXPLICIT" => Self::PlayerRemoveExplicit {
                object_name: value.param(1),
            },

            _ => panic!("Invalid action! {value:?}"),
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

#[derive(Clone, Debug)]
struct Precondition {
    name: String,
    typ: PreconditionType,
}

impl From<ConfigLine> for Precondition {
    fn from(value: ConfigLine) -> Self {
        Self {
            name: value.param(0),
            typ: match value.string(1).as_str() {
                "MUST_TAKE" => PreconditionType::MustTake {
                    param_id: value.param(2),
                    help_id_on_failure: value.param(3),
                    help_id_on_failure_in_multiplayer: value.param(4),
                },
                _ => panic!("Invalid precondition type. {value:?}"),
            },
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
    pub playtest_funds: i32,
    pub multiplayer_funds: [i32; 3],
    pub cutscene: String,
    pub disable_team_and_equipping: String,
    pub lighting_threshholds: [i32; 2],
    pub enemy_grenade_use_chance: i32,
    pub alarm_audio: String,

    emitter_config: EmitterConfig,
    clothing_infiltration_mod: Vec<ClothingInfiltrationMod>,
    pre_actions: Vec<Action>,
    post_actions: Vec<Action>,
    precondition: Vec<Precondition>,
}

pub struct CampaignDefs {
    pub campaigns: Vec<CampaignDef>,
}

impl From<ConfigLines> for CampaignDefs {
    fn from(value: ConfigLines) -> Self {
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

        for line in value.into_iter() {
            match line.key.as_str() {
                "CAMPAIGN_DEF" => match state.0 {
                    None => state = State(Some(CampaignDef::default())),
                    Some(ref mut campaign_def) => {
                        campaigns.push(std::mem::take(campaign_def));
                    }
                },
                "BASENAME" => state.with_campaign().base_name = line.param(0),
                "TITLE" => state.with_campaign().title = line.param(0),
                "MULTIPLAYER_ACTIVE" => state.with_campaign().multiplayer_active = true,
                "EXCLUDE_FROM_CAMPAIGN_TREE" => {
                    state.with_campaign().exclude_from_campaign_tree = true
                }
                "DISABLE_HELP_TIPS" => state.with_campaign().disable_help_tips = true,
                "SKIP_TEAM_EQUIPMENT_VALIDATION" => {
                    state.with_campaign().skip_team_equipment_validation = true
                }
                "PLAYTEST_FUNDS" => {
                    state.with_campaign().playtest_funds = line.param(0);
                }
                "MULTIPLAYER_FUNDS" => {
                    state.with_campaign().multiplayer_funds =
                        [line.param(0), line.param(1), line.param(2)]
                }
                "CUTSCENE" => state.with_campaign().cutscene = line.param(0),
                "DISABLE_TEAM_AND_EQUIPPING" => {
                    state.with_campaign().disable_team_and_equipping = line.param(0)
                }
                "LIGHTING_THRESHHOLDS" => {
                    state.with_campaign().lighting_threshholds = [line.param(0), line.param(1)]
                }
                "ENEMY_GRENADE_USE_CHANCE" => {
                    state.with_campaign().enemy_grenade_use_chance = line.param(0)
                }
                "ALARM_AUDIO" => state.with_campaign().alarm_audio = line.param(0),
                "EMITTER_CONFIG" => state.with_campaign().emitter_config = line.into(),
                "CLOTHING_INFILTRATION_MOD" => state
                    .with_campaign()
                    .clothing_infiltration_mod
                    .push(line.into()),
                "PRE_ACTION" => state.with_campaign().pre_actions.push(line.into()),
                "POST_ACTION" => state.with_campaign().post_actions.push(line.into()),
                "PRECONDITION" => state.with_campaign().precondition.push(line.into()),

                _ => panic!("Unknown CampaignDefs key. {}", line.key),
            }
        }

        if let Some(campaign_def) = state.0 {
            campaigns.push(campaign_def);
        }

        Self { campaigns }
    }
}
