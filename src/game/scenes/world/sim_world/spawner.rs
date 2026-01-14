use std::path::PathBuf;

use bevy_ecs::prelude::*;

use crate::{
    engine::{assets::AssetError, storage::Handle, transform::Transform},
    game::{
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        image::images,
        math::BoundingBox,
        model::{Mesh, Model},
        models::{ModelName, models},
        scenes::world::sim_world::{DynamicBvh, Order, ecs::BoundingBoxComponent},
    },
};

#[derive(Component)]
pub struct SpawnInfo {
    pub _name: String,
    pub _title: String,
    pub _object_type: ObjectType,
}

#[derive(Resource)]
pub struct Spawner {
    character_profiles: CharacterProfiles,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
}

impl Spawner {
    pub fn new(character_profiles: CharacterProfiles) -> Self {
        Self {
            character_profiles,
            models_to_prepare: Vec::default(),
        }
    }

    pub fn spawn(
        &mut self,
        world: &mut World,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        match object_type {
            ObjectType::Ape | ObjectType::Bipedal => {
                self.spawn_bipedal(world, title, name, object_type, transform)
            }

            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree => {
                self.spawn_scenery(world, title, name, object_type, transform)
            }

            ObjectType::Structure
            | ObjectType::StructureArmGate
            | ObjectType::StructureBridge
            | ObjectType::StructureBuggable
            | ObjectType::StructureBuilding
            | ObjectType::StructureBuildingGateController
            | ObjectType::StructureDoubleGate
            | ObjectType::StructureFence
            | ObjectType::StructureGuardTower
            | ObjectType::StructureLadderSlant0_11
            | ObjectType::StructureLadderSlant0_14
            | ObjectType::StructureLadderSlant0_16
            | ObjectType::StructureLadderSlant0_2
            | ObjectType::StructureLadderSlant0_3
            | ObjectType::StructureLadderSlant0_5
            | ObjectType::StructureLadderSlant0_6
            | ObjectType::StructureLadderSlant0_9
            | ObjectType::StructureLadderSlant2_2
            | ObjectType::StructureLadderSlant2_4
            | ObjectType::StructureLadderSlant2_5
            | ObjectType::StructureLocker
            | ObjectType::StructureSAM
            | ObjectType::StructureSingleGate
            | ObjectType::StructureSlideBridge
            | ObjectType::StructureSlideBridgeController
            | ObjectType::StructureSlideDoor
            | ObjectType::StructureStripLightController
            | ObjectType::StructureSwingDoor
            | ObjectType::StructureTent
            | ObjectType::StructureWall => {
                self.spawn_structure(world, title, name, object_type, transform)
            }

            ObjectType::SixBySix
            | ObjectType::SnowMobile
            | ObjectType::Treaded
            | ObjectType::TreadedBMP2
            | ObjectType::TreadedChallenger
            | ObjectType::TreadedScorpion
            | ObjectType::TreadedT55 => {
                self.spawn_vehicle(world, title, name, object_type, transform)
            }

            // These are unknown at this time.
            ObjectType::Bird
            | ObjectType::Boat
            | ObjectType::FourByFour
            | ObjectType::Helicopter
            | ObjectType::Howitzer
            | ObjectType::SentryGun => {
                self.spawn_structure(world, title, name, object_type, transform)
            }
        }
    }

    fn spawn_bipedal(
        &mut self,
        world: &mut World,
        title: &str,
        _name: &str,
        _object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let Some(character_profile) = self.character_profiles.get(title) else {
            tracing::warn!("Character profile not found! ({title})");
            return Err(AssetError::Custom(
                std::path::PathBuf::new(),
                String::from("Character profile not found!"),
            ));
        };

        let body_initial = character_profile.body_initial.as_str();
        let Some(body_definition) = character_profile.body_definitions.get(body_initial) else {
            return Err(AssetError::Custom(
                std::path::PathBuf::new(),
                String::from("Could not find initial body definition!"),
            ));
        };

        let model = build_body_definition_model(body_definition)?;

        let bounding_box = model.bounding_box;

        let model_handle = models().add(
            ModelName::BodyDefinition(
                character_profile.character.clone(),
                body_definition.body_type.clone(),
            ),
            model,
        );

        self.models_to_prepare.push(model_handle);

        let entity = world.spawn_empty().id();

        let dynamic_bvh_handle = {
            let mut dynamic_bvh = world.resource_mut::<DynamicBvh<Entity>>();
            dynamic_bvh.insert(entity, bounding_box.transformed(transform.to_mat4()))
        };

        world.entity_mut(entity).insert((
            transform.clone(),
            model_handle,
            BoundingBoxComponent(bounding_box),
            Order::default(),
            dynamic_bvh_handle,
        ));

        Ok(entity)
    }

    fn spawn_scenery(
        &self,
        world: &mut World,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let (model_handle, model) = models().load_model(ModelName::Object(name.to_string()))?;

        Ok(world
            .spawn((
                transform,
                model_handle,
                BoundingBoxComponent(model.bounding_box),
                SpawnInfo {
                    _name: name.to_string(),
                    _title: title.to_string(),
                    _object_type: object_type,
                },
            ))
            .id())
    }

    fn spawn_structure(
        &self,
        world: &mut World,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let (model_handle, model) = models().load_model(ModelName::Object(name.to_string()))?;

        Ok(world
            .spawn((
                transform,
                model_handle,
                BoundingBoxComponent(model.bounding_box),
                SpawnInfo {
                    _name: name.to_string(),
                    _title: title.to_string(),
                    _object_type: object_type,
                },
            ))
            .id())
    }

    fn spawn_vehicle(
        &self,
        world: &mut World,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        self.spawn_structure(world, title, name, object_type, transform)
    }
}

fn build_body_definition_model(body_definition: &BodyDefinition) -> Result<Model, AssetError> {
    let (_, body_model) =
        models().load_model(ModelName::Body(body_definition.body_model.clone()))?;
    let (_, head_model) =
        models().load_model(ModelName::Head(body_definition.head_model.clone()))?;

    let mut new_model = Model {
        skeleton: body_model.skeleton.clone(),
        meshes: Vec::with_capacity(body_model.meshes.len() + head_model.meshes.len()),
        collision_boxes: Vec::with_capacity(
            body_model.collision_boxes.len() + head_model.collision_boxes.len(),
        ),
        bounding_box: BoundingBox::default(),
        name_lookup: body_model.name_lookup.clone(),
    };

    // Merge the body.
    for mesh in body_model.meshes.iter() {
        let image_name = body_definition.body_map.clone();
        let image =
            images().load_image(PathBuf::from("textures").join("shared").join(&image_name))?;
        new_model.meshes.push(Mesh {
            node_index: mesh.node_index,
            image_name,
            image,
            mesh: mesh.mesh.clone(),
        });
    }
    new_model
        .collision_boxes
        .extend(body_model.collision_boxes.iter().cloned());
    new_model
        .bounding_box
        .expand_to_include(&body_model.bounding_box);

    // Merge the head model meshes.
    for mesh in head_model.meshes.iter() {
        let image_name = body_definition.head_map.clone();
        let image =
            images().load_image(PathBuf::from("textures").join("shared").join(&image_name))?;
        new_model.meshes.push(Mesh {
            node_index: mesh.node_index,
            image_name,
            image,
            mesh: mesh.mesh.clone(),
        });
    }
    new_model
        .collision_boxes
        .extend(head_model.collision_boxes.iter().cloned());
    new_model
        .bounding_box
        .expand_to_include(&head_model.bounding_box);

    Ok(new_model)
}
