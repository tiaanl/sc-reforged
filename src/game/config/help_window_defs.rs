use ahash::HashMap;
use glam::{IVec2, UVec2};

use crate::game::config::parser::ConfigLines;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpPosition {
    #[default]
    Centered,
    Absolute(IVec2),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpPointerLocation {
    Top,
    Bottom,
    Left,
    Right,
}

impl HelpPointerLocation {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "TOP" => Self::Top,
            "BOTTOM" => Self::Bottom,
            "LEFT" => Self::Left,
            "RIGHT" => Self::Right,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpPointer {
    pub location: HelpPointerLocation,
    pub destination: IVec2,
}

#[derive(Debug, Default)]
pub struct HelpDef {
    pub id: String,
    pub is_tip: bool,
    pub is_context: bool,
    pub is_confirmation: bool,
    pub do_not_pause_game: bool,
    pub tutorial_id: Option<i32>,
    pub tutorial_next: Option<String>,
    pub position: Option<IVec2>,
    pub dimensions: Option<IVec2>,
    pub body_lines: Vec<String>,
    pub pointer: Option<HelpPointer>,
    pub confirmation_text_1: Option<String>,
    pub confirmation_text_2: Option<String>,
}

#[derive(Debug, Default)]
pub struct HelpWindowDefs {
    help_defs: HashMap<String, HelpDef>,
}

impl HelpWindowDefs {
    /// Returns the help window definition with the given resource id.
    pub fn get(&self, id: &str) -> Option<&HelpDef> {
        self.help_defs.get(id)
    }
}

impl From<ConfigLines> for HelpWindowDefs {
    fn from(lines: ConfigLines) -> Self {
        let mut help_window_defs = HelpWindowDefs::default();

        #[derive(Debug, Default)]
        enum State {
            #[default]
            None,
            HelpDef(HelpDef),
        }

        impl State {
            fn flush(&mut self, help_window_defs: &mut HelpWindowDefs) {
                match self {
                    State::None => {}
                    State::HelpDef(help_def) => {
                        help_window_defs
                            .help_defs
                            .insert(help_def.id.clone(), std::mem::take(help_def));
                        *self = State::None;
                    }
                }
            }

            fn with_help_def<F>(&mut self, f: F)
            where
                F: FnOnce(&mut HelpDef),
            {
                if let Self::HelpDef(help_def) = self {
                    f(help_def);
                } else {
                    tracing::warn!("No HelpDef active: {self:?}");
                }
            }
        }

        let mut state = State::default();

        for line in lines.into_iter() {
            if line.key == "HELP_DEF" {
                state.flush(&mut help_window_defs);
                state = State::HelpDef(HelpDef::default());
                continue;
            }

            match line.key.as_str() {
                "HELP_ID" => {
                    state.with_help_def(|help_def| help_def.id = line.param(0));
                }
                "HELP_TIP" => {
                    state.with_help_def(|help_def| help_def.is_tip = true);
                }
                "HELP_CONTEXT" => {
                    state.with_help_def(|help_def| help_def.is_context = true);
                }
                "HELP_CONFIRMATION" => {
                    state.with_help_def(|help_def| help_def.is_confirmation = true);
                }
                "HELP_DO_NOT_PAUSE_GAME" => {
                    state.with_help_def(|help_def| help_def.do_not_pause_game = true);
                }
                "HELP_TUTOR" => {
                    state.with_help_def(|help_def| help_def.tutorial_id = line.maybe_param(0));
                }
                "HELP_TUTOR_NEXT" => {
                    state.with_help_def(|help_def| help_def.tutorial_next = line.maybe_param(0));
                }
                "HELP_POS" => {
                    state.with_help_def(|help_def| {
                        help_def.position = Some(IVec2::new(line.param(0), line.param(1)))
                    });
                }
                "HELP_DIMS" => {
                    state.with_help_def(|help_def| {
                        let width: i32 = line.param(0);
                        let height: i32 = line.param(1);
                        help_def.dimensions = Some(IVec2::new(width.max(0), height.max(0)));
                    });
                }
                "HELP_BODY" => {
                    state.with_help_def(|help_def| help_def.body_lines.push(line.param(0)));
                }
                "HELP_POINTER" => {
                    let Some(location) = line
                        .maybe_param::<String>(0)
                        .as_deref()
                        .and_then(HelpPointerLocation::parse)
                    else {
                        tracing::warn!(
                            "Invalid HELP_POINTER location: {:?}",
                            line.maybe_param::<String>(0)
                        );
                        continue;
                    };

                    state.with_help_def(|help_def| {
                        help_def.pointer = Some(HelpPointer {
                            location,
                            destination: IVec2::new(line.param(1), line.param(2)),
                        });
                    });
                }
                "HELP_CONFIRMATION_TEXT_1" => {
                    state.with_help_def(|help_def| {
                        help_def.confirmation_text_1 = line.maybe_param(0);
                    });
                }
                "HELP_CONFIRMATION_TEXT_2" => {
                    state.with_help_def(|help_def| {
                        help_def.confirmation_text_2 = line.maybe_param(0);
                    });
                }
                _ => {
                    tracing::warn!("Unknown key for HelpWindowDefs: {} ({state:?})", line.key);
                }
            }
        }

        state.flush(&mut help_window_defs);

        help_window_defs
    }
}
