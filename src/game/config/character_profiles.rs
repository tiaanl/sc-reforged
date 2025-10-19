use ahash::HashMap;
use glam::IVec2;

use crate::game::config::parser::ConfigLine;

use super::parser::ConfigLines;

#[derive(Debug, Default)]
pub struct BodyDefinition {
    body_type: String,
    head_model: String,
    head_map: String,
    body_model: String,
    body_map: String,
    pack_model: String,
    pack_map: String,
}

#[derive(Debug, Default)]
pub struct Attributes {
    strength: i32,
    intelligence: i32,
    dexterity: i32,
    endurance: i32,

    health_max: i32,
    morale_default: i32,
}

#[derive(Debug, Default)]
pub struct CharacterProfile {
    character: String,
    firstname: String,
    lastname: String,
    nickname: String,
    abrev_name: String,
    default_object_id: i32,

    sound_dir: String,
    script: String,

    age: i32,
    sex: String,
    nationality: String,
    height: String,
    weight: String,

    /// Set PLAYER_CHARACTER; This determines if this is a potentially player controlled commando.
    /// Presence of this line indicates this is true. (defalts to false)
    player_character: bool,

    /// Set ON_INITIAL_ROSTER; This says just that; defaults to false. Should never be true unless
    /// PLAYER_CHARACTER is also true.
    on_initial_roster: bool,

    difficulty_scaled: bool,

    /// Inlcude this flag if we don't want the body to ever gib.
    persistant: bool,

    /// The initial body this character is created with.
    body_initial: String,

    /// NOTE: First body definition is used as the default if a requested body not.
    body_definitions: HashMap<String, BodyDefinition>,

    /// NOTE: ALL Attributes must be defined!
    attributes: Attributes,

    /// BASE_USAGE_COST: Starting cost to use per mission; this amount is increased over usage &
    /// skill increases.
    base_usage_cost: i32,

    /// Specify skill proficiencies. All valid skills are enumerated here.
    /// NOTE: Spaces aren't allowed in the skill name!
    ///       Other than using '_' instead of spaces, the skill name should appear as hey appear in
    ///       the commando window.
    skills: HashMap<String, i32>,

    /// Specify abilities.
    abilities: Vec<String>,

    give_object: Vec<String>,
    give_1_object_of_2: Vec<[String; 2]>,
    give_1_object_of_3: Vec<[String; 3]>,
    give_1_object_of_4: Vec<[String; 4]>,

    bottom_bar_face: IVec2,
    equip_screen_face: IVec2,

    dossier_lines: Vec<String>,
}

#[derive(Default)]
pub struct CharacterProfiles {
    character_profiles: HashMap<String, CharacterProfile>,
}

impl CharacterProfiles {
    pub fn parse_lines(&mut self, lines: ConfigLines) {
        #[derive(Debug)]
        enum State {
            None,
            CharacterProfile(CharacterProfile),
            BodyDefinition(CharacterProfile, BodyDefinition),
            Attributes(CharacterProfile),
        }

        let mut state = State::None;

        for line in lines.into_iter() {
            match state {
                State::None => match line.key.as_str() {
                    "CHARACTER_BEGIN" => {
                        state = State::CharacterProfile(CharacterProfile::default())
                    }
                    _ => panic!("Unexpected character profiles key: {line:?}, state: {state:?}"),
                },

                State::CharacterProfile(ref mut character_profile) => match line.key.as_ref() {
                    "CHARACTER" => character_profile.character = line.param(0),
                    "FIRSTNAME" => character_profile.firstname = line.param(0),
                    "LASTNAME" => character_profile.lastname = line.param(0),
                    "NICKNAME" => character_profile.nickname = line.param(0),
                    "ABREV_NAME" => character_profile.abrev_name = line.param(0),

                    "DEFAULT_OBJECT_ID" => character_profile.default_object_id = line.param(0),

                    "SOUND_DIR" => character_profile.sound_dir = line.param(0),
                    "SCRIPT" => character_profile.script = line.param(0),

                    "AGE" => character_profile.age = line.param(0),
                    "SEX" => character_profile.sex = line.param(0),
                    "NATIONALITY" => character_profile.nationality = line.param(0),
                    "HEIGHT" => character_profile.height = line.param(0),
                    "WEIGHT" => character_profile.weight = line.param(0),

                    "PLAYER_CHARACTER" => character_profile.player_character = true,
                    "ON_INITIAL_ROSTER" => character_profile.on_initial_roster = true,

                    "DIFFICULTY_SCALED" => character_profile.difficulty_scaled = line.param(0),

                    "PERSISTANT" => character_profile.persistant = true,

                    "BODY_INITIAL" => character_profile.body_initial = line.param(0),

                    "BODY_DEFINITION" => {
                        state = State::BodyDefinition(
                            std::mem::take(character_profile),
                            BodyDefinition::default(),
                        );
                    }

                    "ATTRIBUTES" => {
                        state = State::Attributes(std::mem::take(character_profile));
                    }

                    "BASE_USAGE_COST" => character_profile.base_usage_cost = line.param(0),

                    "SKILL" => {
                        character_profile
                            .skills
                            .insert(line.param(0), line.param(1));
                    }

                    "GIVE_OBJECT" => character_profile.give_object.push(line.param(0)),
                    "GIVE_1_OBJECT_OF_2" => character_profile
                        .give_1_object_of_2
                        .push([line.param(0), line.string(1)]),
                    "GIVE_1_OBJECT_OF_3" => character_profile.give_1_object_of_3.push([
                        line.param(0),
                        line.string(1),
                        line.string(2),
                    ]),
                    "GIVE_1_OBJECT_OF_4" => character_profile.give_1_object_of_4.push([
                        line.param(0),
                        line.string(1),
                        line.string(2),
                        line.string(3),
                    ]),

                    "ABILITY" => character_profile.abilities.push(line.param(0)),

                    "BOTTOM_BAR_FACE" => {
                        character_profile.bottom_bar_face = IVec2::new(line.param(0), line.param(1))
                    }

                    "EQUIP_SCREEN_FACE" => {
                        character_profile.equip_screen_face =
                            IVec2::new(line.param(0), line.param(1))
                    }

                    "DOSSIER_LINE" => character_profile.dossier_lines.push(line.param(0)),

                    "CHARACTER_END" => {
                        self.character_profiles.insert(
                            character_profile.character.clone(),
                            std::mem::take(character_profile),
                        );
                        state = State::None;
                    }
                    _ => panic!("Unexpected character profile key: {line:?}, state: {state:?}"),
                },

                State::BodyDefinition(ref mut character_profile, ref mut body_definition) => {
                    match line.key.as_ref() {
                        "BODY_TYPE" => body_definition.body_type = line.param(0),
                        "HEAD_MODEL" => body_definition.head_model = line.param(0),
                        "HEAD_MAP" => body_definition.head_map = line.param(0),
                        "BODY_MODEL" => body_definition.body_model = line.param(0),
                        "BODY_MAP" => body_definition.body_map = line.param(0),
                        "PACK_MODEL" => body_definition.pack_model = line.param(0),
                        "PACK_MAP" => body_definition.pack_map = line.param(0),

                        "BODY_DEFINITION_END" => {
                            character_profile.body_definitions.insert(
                                body_definition.body_type.clone(),
                                std::mem::take(body_definition),
                            );
                            state = State::CharacterProfile(std::mem::take(character_profile));
                        }

                        _ => panic!("Unexpected body definition key: {line:?}, state: {state:?}"),
                    }
                }

                State::Attributes(ref mut character_profile) => match line.key.as_ref() {
                    "STRENGTH" => character_profile.attributes.strength = line.param(0),
                    "INTELLIGENCE" => character_profile.attributes.intelligence = line.param(0),
                    "DEXTERITY" => character_profile.attributes.dexterity = line.param(0),
                    "ENDURANCE" => character_profile.attributes.endurance = line.param(0),

                    "HEALTH_MAX" => character_profile.attributes.health_max = line.param(0),
                    "MORALE_DEFAULT" => character_profile.attributes.morale_default = line.param(0),

                    "ATTRIBUTES_END" => {
                        state = State::CharacterProfile(std::mem::take(character_profile));
                    }

                    _ => panic!("Unexpected attrbiutes key: {line:?}, state: {state:?}"),
                },
            };
        }
    }
}

/*
match line.key.as_str() {
                "CHARACTER_BEGIN" => state = State::CharacterProfile(CharacterProfile::default()),

                k if character_profile.is_some() => {
                    if let Some(character_profile) = character_profile.as_mut() {
                        match k {


                            _ => panic!("Unexpected character key: {line:?}"),
                        }
                    }
                }

                "CHARACTER_END" => {
                    if let Some(character_profile) = character_profile.take() {
                        self.character_profiles
                            .insert(character_profile.character.clone(), character_profile);
                    }
                }

                _ =>
            }
*/
