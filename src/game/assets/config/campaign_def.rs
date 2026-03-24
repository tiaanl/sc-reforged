#![allow(clippy::doc_lazy_continuation)]

use std::str::FromStr;

use ahash::HashMap;
use strum::EnumString;

use crate::game::config::parser::{ConfigLine, ConfigLines};

#[derive(Debug, EnumString, PartialEq, Eq, Hash)]
pub enum ClothingType {
    NightOps,
    Arctic,
}

#[derive(Debug)]
#[allow(unused)]
pub enum Action {
    /// This is the amount of money given to the player.
    Reward { cash_amount: i32 },
    /// Specifies a number of an explicit object to be stocked to the dealers'
    /// inventory.
    DealerStockExplicit { object_name: String, count: i32 },
    /// Adds <count> random items to the dealer stock with a <mod> chance
    /// modifier. A positive `<mod>` indicates more rare weapons will be
    /// stocked. Eg: the formulae is, if, `(1 -> 100) < stocked_chance + <mod>`,
    /// for a particular item, that item will be successfully chosen.
    DealerStockRandom { count: i32, mod_: i32 },
    /// This allows you to set the dealer price markup; the markup will take
    /// effect on all currently existing dealer equipment, as well as all future
    /// equipment (unless the markup is reset). So, a markup of 100 will make
    /// equipment cost twice as much as that listed in the file (what the player
    /// will get for selling them).
    DealerSetMarkup { percentage_markup: i32 },
    /// Specifies an object to give <count> of to the shadow company inventory
    /// automatically.
    PlayerStockExplicit { object_name: String, count: i32 },
    /// Adds a specific merc to the roster.
    RosterMerc { merc_name: String },
    /// Adds a specific merc to the team list.
    HireMerc { merc_name: String },
    /// Causes the dealer's inventory to be completely flushed of all items.
    DealerRemoveAll,
    /// Attempts to find the specific object in the player's item inventory; if
    /// it is not found in the shadow company inventory, the individual's
    /// inventories will be searched as well (needed for taking stuff like
    /// mutagen and lithium cores from the player after a mission).
    PlayerRemoveExplicit { object_name: String },
    /// Specifies a number of an explicit object to be stocked to the dealers'
    /// inventory in multiplayer.
    MultiplayerDealerStockExplicit { object_name: String, count: i32 },
}

impl TryFrom<&ConfigLine> for Action {
    type Error = ();

    fn try_from(line: &ConfigLine) -> Result<Self, Self::Error> {
        Ok(match line.string(0).as_str() {
            "REWARD" => Self::Reward {
                cash_amount: line.param(1),
            },
            "DEALER_STOCK_EXPLICIT" => Self::DealerStockExplicit {
                object_name: line.param(1),
                count: line.param(2),
            },
            "DEALER_STOCK_RANDOM" => Self::DealerStockRandom {
                count: line.param(1),
                mod_: line.param(2),
            },
            "DEALER_SET_MARKUP" => Self::DealerSetMarkup {
                percentage_markup: line.param(1),
            },
            "PLAYER_STOCK_EXPLICIT" => Self::PlayerStockExplicit {
                object_name: line.param(1),
                count: line.param(2),
            },
            "ROSTER_MERC" => Self::RosterMerc {
                merc_name: line.param(1),
            },
            "HIRE_MERC" => Self::HireMerc {
                merc_name: line.param(1),
            },
            "DEALER_REMOVE_ALL" => Self::DealerRemoveAll,
            "PLAYER_REMOVE_EXPLICIT" => Self::PlayerRemoveExplicit {
                object_name: line.param(1),
            },
            "MPM_DEALER_STOCK_EXPLICIT" => Self::MultiplayerDealerStockExplicit {
                object_name: line.param(1),
                count: line.param(2),
            },

            _ => {
                tracing::warn!("Unknown Action name: {}", line.string(0));
                return Err(());
            }
        })
    }
}

#[derive(Debug)]
#[allow(unused)]
pub enum Precondition {
    MustTake {
        precondition_name: String,
        param_id: String,
        help_id_on_failure: String,
        help_id_on_failure_in_multiplayer: String,
    },
}

impl TryFrom<&ConfigLine> for Precondition {
    type Error = ();

    fn try_from(line: &ConfigLine) -> Result<Self, Self::Error> {
        Ok(match line.string(1).as_str() {
            "MUST_TAKE" => Self::MustTake {
                precondition_name: line.param(0),
                param_id: line.param(2),
                help_id_on_failure: line.param(3),
                help_id_on_failure_in_multiplayer: line.param(4),
            },

            _ => {
                tracing::warn!("Unknown Precondition name: {}", line.string(0));
                return Err(());
            }
        })
    }
}

#[derive(Debug)]
pub struct CampaignDef {
    /// `<campaign_basename>`
    pub base_name: String,
    /// `<campaign_title>`
    pub title: String,
    /// If this appears, then the campaign can be used in multiplayer.
    pub multiplayer_active: bool,
    /// Excludes this campaign from the campaign tree.
    pub exclude_from_campaign_tree: bool,
    // Skips checking if the team members are carrying equipment before
    // starting campaign.
    pub skip_team_equipment_validation: bool,
    /// Disables help tips for the campaign, no matter what the user setting is.
    pub disable_help_tips: bool,
    /// This is the amount of money that will be set when the campaign is
    /// directly set using the "set_campaign" in-game command. (Hence,
    /// this overrides the REWARDs executed on the campaign path.)
    pub playtest_funds: i32,
    /// Allows setting base cash amounts per campaign & difficulty level for
    /// multiplayer mode.
    /// `[<easy_cash>, <normal_cash>, <hard_cash>]`
    pub multiplayer_funds: [i32; 3],
    /// Specifies the cutscene resource for this mission.
    /// `<anim_profile_name>`
    pub cutscene: Option<String>,
    /// If specified, the player will not be allowed to go to the team or
    /// equipping interfaces from the planning screen. If they try, the
    /// specified context help will be displayed.
    /// `Some(<context_help_resource_name>)`
    pub disable_team_and_equipping: Option<String>,
    /// Specify the lighting threshholds for the campaign; defaults to
    /// `[350, 550]`. Lighting value is r+g+b (on 0-255 scale). If this is less
    /// than the <night> value, it is night time; if it is greater that
    /// `<good_lighting>`, good lighting modifiers are applied.
    /// `[<night>, <good_lighting>]`
    pub lighting_threshholds: [i32; 2],
    /// Allows modification of an additional grenade use percent chance by the
    /// enemies from campaign to campaign.
    /// `<additional % chance to use>`
    pub enemy_grenade_use_chance: i32,
    /// Sets the alarm audio sample for the given campaign.
    /// `<alarm_audio_sample>`
    pub alarm_audio: Option<String>,
    /// Support for emitter configuration.
    /// Example: `EMITTER_CONFIG CHIMNEY EMITTER_CONFIG_CHIMNEY_SMOKE_LIGHT`
    /// Currently valid MI_subsets are:
    ///   - CHIMNEY
    /// Currently valid `emitter_types` are specified in `emitter_defs.txt`.
    /// `<MI_subset> <emitter_type_desired>`
    pub emitter_config: Option<[String; 2]>,
    /// Support for campaign to campaign clothing infiltration modifier.
    /// Example: `CLOTHING_INFILTRATION_MOD NightOps 0 10`
    /// `<Clothing_Type> <day_infiltration_modifier>
    /// <night_infiltration_modifier>`
    pub clothing_infiltration_mod: HashMap<ClothingType, [i32; 2]>,
    /// Action types: POST_ACTION, followed by the following args indicate post
    /// campaign completion actions.
    pub post_actions: Vec<Action>,
    /// Action types: PRE_ACTION, followed by the action args, indicate pre
    /// campaign completion actions.
    pub pre_actions: Vec<Action>,
    /// All PRECONDITIONs must be met, otherwise the campaign will not be
    /// allowed to launch.
    /// Example:
    ///   `PRECONDITION kola_camera MUST_TAKE Camera help_kola_no_camera
    ///   help_kola_no_camera_mpm`
    /// In the above, a merc must have the "Camera" object. If a launch attempt
    /// is made, and no merc has it, the help window "help_kola_no_camera" will
    /// be opened.
    pub preconditions: Vec<Precondition>,
}

impl Default for CampaignDef {
    fn default() -> Self {
        Self {
            base_name: String::default(),
            title: String::default(),
            multiplayer_active: false,
            exclude_from_campaign_tree: false,
            skip_team_equipment_validation: false,
            disable_help_tips: false,
            playtest_funds: 0,
            cutscene: None,
            multiplayer_funds: [0, 0, 0],
            disable_team_and_equipping: None,
            lighting_threshholds: [350, 550],
            enemy_grenade_use_chance: 0,
            alarm_audio: None,
            emitter_config: None,
            clothing_infiltration_mod: HashMap::default(),
            pre_actions: Vec::default(),
            post_actions: Vec::default(),
            preconditions: Vec::default(),
        }
    }
}

/// Defines the campaigns in the game and their parameters. Loaded from:
/// `data\config\campaign_defs.txt`.
#[derive(Debug, Default)]
pub struct CampaignDefs {
    /// Stored in a [Vec] to preserve the order.
    campaign_defs: Vec<CampaignDef>,
}

impl From<ConfigLines> for CampaignDefs {
    fn from(value: ConfigLines) -> Self {
        let mut out = Self::default();

        #[derive(Debug, Default)]
        #[allow(clippy::large_enum_variant)]
        enum State {
            #[default]
            None,
            CampaignDef(CampaignDef),
        }

        impl State {
            fn flush(&mut self, campaign_defs: &mut CampaignDefs) {
                if let Self::CampaignDef(campaign_def) = std::mem::take(self) {
                    campaign_defs.campaign_defs.push(campaign_def);
                }
            }

            fn with_campaign_def<F>(&mut self, f: F)
            where
                F: FnOnce(&mut CampaignDef),
            {
                if let State::CampaignDef(campaign_def) = self {
                    f(campaign_def);
                } else {
                    tracing::warn!("No CampaignDef active: {self:?}");
                }
            }
        }

        let mut state = State::None;

        for line in value.into_iter() {
            if line.key == "CAMPAIGN_DEF" {
                state.flush(&mut out);
                state = State::CampaignDef(CampaignDef::default());
                continue;
            }

            match line.key.as_str() {
                "BASENAME" => {
                    state.with_campaign_def(|c| c.base_name = line.param(0));
                }
                "TITLE" => {
                    state.with_campaign_def(|c| c.title = line.param(0));
                }
                "MULTIPLAYER_ACTIVE" => {
                    state.with_campaign_def(|c| c.multiplayer_active = true);
                }
                "EXCLUDE_FROM_CAMPAIGN_TREE" => {
                    state.with_campaign_def(|c| c.exclude_from_campaign_tree = true);
                }
                "SKIP_TEAM_EQUIPMENT_VALIDATION" => {
                    state.with_campaign_def(|c| c.skip_team_equipment_validation = true);
                }
                "DISABLE_HELP_TIPS" => {
                    state.with_campaign_def(|c| c.disable_help_tips = true);
                }
                "PLAYTEST_FUNDS" => {
                    state.with_campaign_def(|c| c.playtest_funds = line.param(0));
                }
                "CUTSCENE" => {
                    state.with_campaign_def(|c| c.cutscene = line.maybe_param(0));
                }
                "MULTIPLAYER_FUNDS" => {
                    state.with_campaign_def(|c| {
                        c.multiplayer_funds = [line.param(0), line.param(1), line.param(2)]
                    });
                }
                "DISABLE_TEAM_AND_EQUIPPING" => {
                    state.with_campaign_def(|c| c.disable_team_and_equipping = line.maybe_param(0));
                }
                "LIGHTING_THRESHHOLDS" => {
                    state.with_campaign_def(|c| {
                        c.lighting_threshholds = [line.param(0), line.param(1)]
                    });
                }
                "ENEMY_GRENADE_USE_CHANCE" => {
                    state.with_campaign_def(|c| c.enemy_grenade_use_chance = line.param(0));
                }
                "ALARM_AUDIO" => {
                    state.with_campaign_def(|c| c.alarm_audio = line.maybe_param(0));
                }
                "EMITTER_CONFIG" => {
                    state.with_campaign_def(|c| {
                        c.emitter_config = Some([line.param(0), line.param(1)])
                    });
                }
                "CLOTHING_INFILTRATION_MOD" => {
                    if let Some(clothing_type) = line
                        .maybe_param::<String>(0)
                        .and_then(|s| ClothingType::from_str(s.as_str()).ok())
                    {
                        state.with_campaign_def(|c| {
                            c.clothing_infiltration_mod
                                .insert(clothing_type, [line.param(1), line.param(2)]);
                        });
                    }
                }
                "POST_ACTION" => {
                    if let Ok(action) = Action::try_from(&line) {
                        state.with_campaign_def(|c| {
                            c.post_actions.push(action);
                        });
                    }
                }
                "PRE_ACTION" => {
                    if let Ok(action) = Action::try_from(&line) {
                        state.with_campaign_def(|c| {
                            c.pre_actions.push(action);
                        });
                    }
                }
                "PRECONDITION" => {
                    if let Ok(precondition) = Precondition::try_from(&line) {
                        state.with_campaign_def(|c| {
                            c.preconditions.push(precondition);
                        });
                    }
                }

                _ => {
                    tracing::warn!("Unknown key for CampaignDefs: {} ({:?})", line.key, state);
                }
            }
        }

        state.flush(&mut out);

        out
    }
}
