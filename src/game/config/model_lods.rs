use std::collections::HashMap;

use crate::game::asset_loader::AssetError;

use super::ConfigFile;

#[derive(Debug, Default)]
pub struct SubModelDefinition {
    pub sub_model_model: String,
    pub sub_model_range: f32,
}

#[derive(Debug, Default)]
pub struct LodModelProfileDefinition {
    pub lod_model_name: String,
    pub sub_model_definitions: Vec<SubModelDefinition>,
}

impl TryFrom<String> for LodModelProfileDefinition {
    type Error = AssetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut config = ConfigFile::new(&value);

        enum State {
            None,
            Def(LodModelProfileDefinition),
            SubDef(LodModelProfileDefinition, SubModelDefinition),
        }
        let mut state = State::None;

        while let Some(current) = config.current() {
            match current[0] {
                "LOD_MODEL_PROFILE_DEFINITION" => match state {
                    State::None => state = State::Def(LodModelProfileDefinition::default()),
                    State::Def(..) | State::SubDef(..) => {
                        tracing::warn!(
                            "Can't do multiple LOD_MODEL_PROFILE_DEFINITION per config file."
                        );
                        break;
                    }
                },

                "LOD_MODEL_NAME" => match state {
                    State::None => {
                        panic!("LOD_MODEL_NAME without LOD_MODEL_PROFILE_DEFINITION");
                    }
                    State::Def(ref mut def) | State::SubDef(ref mut def, _) => {
                        def.lod_model_name = current[1].to_string();
                    }
                },

                "SUB_MODEL_DEFINITION" => match state {
                    State::None => {
                        panic!("SUB_MODEL_DEFINITION without LOD_MODEL_PROFILE_DEFINITION")
                    }
                    State::Def(def) => {
                        state = State::SubDef(def, SubModelDefinition::default());
                    }
                    State::SubDef(mut def, sub_def) => {
                        def.sub_model_definitions.push(sub_def);
                        state = State::SubDef(def, SubModelDefinition::default());
                    }
                },

                "SUB_MODEL_MODEL" => match state {
                    State::None => panic!("SUB_MODEL_MODEL without LOD_MODEL_PROFILE_DEFINITION"),
                    State::Def(_) => panic!("SUB_MODEL_MODEL without SUB_MODEL_DEFINITION"),
                    State::SubDef(_, ref mut sub_def) => {
                        sub_def.sub_model_model = current[1].to_string();
                    }
                },

                "SUB_MODEL_RANGE" => match state {
                    State::None => panic!("SUB_MODEL_RANGE without LOD_MODEL_PROFILE_DEFINITION"),
                    State::Def(_) => panic!("SUB_MODEL_RANGE without SUB_MODEL_DEFINITION"),
                    State::SubDef(_, ref mut sub_def) => {
                        sub_def.sub_model_range = current[1].parse().unwrap();
                    }
                },

                _ => {}
            }
            config.next();
        }

        let mut def = match state {
            State::None => panic!("No LOD_MODEL_PROFILE_DEFINITION"),
            State::Def(def) => def,
            State::SubDef(mut def, sub_def) => {
                def.sub_model_definitions.push(sub_def);
                def
            }
        };

        def.sub_model_definitions.sort_by(|a, b| {
            if a.sub_model_range < b.sub_model_range {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        Ok(def)
    }
}
