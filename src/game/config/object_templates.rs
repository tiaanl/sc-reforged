use ahash::HashMap;
use bevy_ecs::prelude as ecs;

use crate::game::config::parser::ConfigLines;

#[derive(Clone, ecs::Component, Copy, Debug, PartialEq)]
pub enum ObjectType {
    Ape,
    Bipedal,
    Bird,
    Boat,
    FourByFour,
    Helicopter,
    Howitzer,
    Scenery,
    SceneryAlarm,
    SceneryBush,
    SceneryFragile,
    SceneryLit,
    SceneryShadowed,
    SceneryStripLight,
    SceneryTree,
    SentryGun,
    SixBySix,
    SnowMobile,
    Structure,
    StructureArmGate,
    StructureBridge,
    StructureBuggable,
    StructureBuilding,
    StructureBuildingGateController,
    StructureDoubleGate,
    StructureFence,
    StructureGuardTower,
    StructureLadderSlant0_11,
    StructureLadderSlant0_14,
    StructureLadderSlant0_16,
    StructureLadderSlant0_2,
    StructureLadderSlant0_3,
    StructureLadderSlant0_5,
    StructureLadderSlant0_6,
    StructureLadderSlant0_9,
    StructureLadderSlant2_2,
    StructureLadderSlant2_4,
    StructureLadderSlant2_5,
    StructureLocker,
    StructureSAM,
    StructureSingleGate,
    StructureSlideBridge,
    StructureSlideBridgeController,
    StructureSlideDoor,
    StructureStripLightController,
    StructureSwingDoor,
    StructureTent,
    StructureWall,
    Treaded,
    TreadedBMP2,
    TreadedChallenger,
    TreadedScorpion,
    TreadedT55,
}

impl ObjectType {
    pub fn from_string(str: &str) -> Option<Self> {
        Some(match str.to_ascii_lowercase().as_str() {
            "4x4" => Self::FourByFour,
            "6x6" => Self::SixBySix,
            "ape" => Self::Ape,
            "bipedal" => Self::Bipedal,
            "bird" => Self::Bird,
            "boat" => Self::Boat,
            "helicopter" => Self::Helicopter,
            "howitzer" => Self::Howitzer,
            "scenery_alarm" => Self::SceneryAlarm,
            "scenery_bush" => Self::SceneryBush,
            "scenery_fragile" => Self::SceneryFragile,
            "scenery_lit" => Self::SceneryLit,
            "scenery_shadowed" => Self::SceneryShadowed,
            "scenery_strip_light" => Self::SceneryStripLight,
            "scenery_tree" => Self::SceneryTree,
            "scenery" => Self::Scenery,
            "sentry_gun" => Self::SentryGun,
            "snow_mobile" => Self::SnowMobile,
            "structure_arm_gate" => Self::StructureArmGate,
            "structure_bridge" => Self::StructureBridge,
            "structure_buggable" => Self::StructureBuggable,
            "structure_building_gate_controller" => Self::StructureBuildingGateController,
            "structure_building" => Self::StructureBuilding,
            "structure_double_gate" => Self::StructureDoubleGate,
            "structure_fence" => Self::StructureFence,
            "structure_guard_tower" => Self::StructureGuardTower,
            "structure_ladder_slant0_11" => Self::StructureLadderSlant0_11,
            "structure_ladder_slant0_14" => Self::StructureLadderSlant0_14,
            "structure_ladder_slant0_16" => Self::StructureLadderSlant0_16,
            "structure_ladder_slant0_2" => Self::StructureLadderSlant0_2,
            "structure_ladder_slant0_3" => Self::StructureLadderSlant0_3,
            "structure_ladder_slant0_5" => Self::StructureLadderSlant0_5,
            "structure_ladder_slant0_6" => Self::StructureLadderSlant0_6,
            "structure_ladder_slant0_9" => Self::StructureLadderSlant0_9,
            "structure_ladder_slant2_2" => Self::StructureLadderSlant2_2,
            "structure_ladder_slant2_4" => Self::StructureLadderSlant2_4,
            "structure_ladder_slant2_5" => Self::StructureLadderSlant2_5,
            "structure_locker" => Self::StructureLocker,
            "structure_sam" => Self::StructureSAM,
            "structure_single_gate" => Self::StructureSingleGate,
            "structure_slide_bridge_controller" => Self::StructureSlideBridgeController,
            "structure_slide_bridge" => Self::StructureSlideBridge,
            "structure_slide_door" => Self::StructureSlideDoor,
            "structure_strip_light_controller" => Self::StructureStripLightController,
            "structure_swing_door" => Self::StructureSwingDoor,
            "structure_tent" => Self::StructureTent,
            "structure_wall" => Self::StructureWall,
            "structure" => Self::Structure,
            "treaded_bmp2" => Self::TreadedBMP2,
            "treaded_challenger" => Self::TreadedChallenger,
            "treaded_scorpion" => Self::TreadedScorpion,
            "treaded_t55" => Self::TreadedT55,
            "treaded" => Self::Treaded,

            _ => return None,
        })
    }
}

#[derive(Debug)]
pub struct ObjectTemplates {
    pub _templates: HashMap<String, ObjectType>,
}

impl From<ConfigLines> for ObjectTemplates {
    fn from(value: ConfigLines) -> Self {
        let mut templates = HashMap::default();

        for line in value.into_iter() {
            if let Some(object_type) = line.maybe_param::<String>(0) {
                let Some(object_type) = ObjectType::from_string(object_type.as_str()) else {
                    panic!("Invalid object type: {object_type}");
                };

                templates.insert(line.key, object_type);
            }
        }

        ObjectTemplates {
            _templates: templates,
        }
    }
}
