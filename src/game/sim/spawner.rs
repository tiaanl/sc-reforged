use std::path::PathBuf;

use bevy_ecs::prelude::*;

use crate::{
    engine::{assets::AssetError, storage::Handle, transform::Transform},
    game::{
        assets::{
            model::{Mesh, Model},
            models::Models,
        },
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        math::BoundingBox,
        models::ModelName,
        sim::{
            DynamicBvh, StaticBvhHandle, ecs::BoundingBoxComponent, sequences::MotionController,
        },
    },
};

use super::{orders::OrdersController, sequences::Pose};

#[derive(Component)]
pub struct SpawnInfo {
    pub _name: String,
    pub _title: String,
    pub _object_type: ObjectType,
}

#[derive(Resource)]
pub struct Spawner {
    character_profiles: CharacterProfiles,
}

impl Spawner {
    pub fn new(character_profiles: CharacterProfiles) -> Self {
        Self { character_profiles }
    }

    pub fn spawn(
        &mut self,
        world: &mut World,
        models: &Models,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        match object_type {
            ObjectType::Ape | ObjectType::Bipedal => {
                self.spawn_bipedal(world, models, title, name, object_type, transform)
            }

            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree => {
                self.spawn_scenery(world, models, title, name, object_type, transform)
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
                self.spawn_structure(world, models, title, name, object_type, transform)
            }

            ObjectType::SixBySix
            | ObjectType::SnowMobile
            | ObjectType::Treaded
            | ObjectType::TreadedBMP2
            | ObjectType::TreadedChallenger
            | ObjectType::TreadedScorpion
            | ObjectType::TreadedT55 => {
                self.spawn_vehicle(world, models, title, name, object_type, transform)
            }

            // These are unknown at this time.
            ObjectType::Bird
            | ObjectType::Boat
            | ObjectType::FourByFour
            | ObjectType::Helicopter
            | ObjectType::Howitzer
            | ObjectType::SentryGun => {
                self.spawn_structure(world, models, title, name, object_type, transform)
            }
        }
    }

    fn spawn_bipedal(
        &mut self,
        world: &mut World,
        models: &Models,
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

        let model = build_body_definition_model(body_definition, models)?;

        let bounding_box = model.bounding_box;

        let model_handle = models.insert(
            ModelName::BodyDefinition(
                character_profile.character.clone(),
                body_definition.body_type.clone(),
            ),
            model,
        );

        let entity = world.spawn_empty().id();

        let dynamic_bvh_handle = {
            let mut dynamic_bvh = world.resource_mut::<DynamicBvh>();
            dynamic_bvh.insert(entity, bounding_box.transformed(transform.to_mat4()))
        };

        let motion_controller = MotionController::default();

        world.entity_mut(entity).insert((
            SpawnInfo {
                _name: _name.to_string(),
                _title: title.to_string(),
                _object_type,
            },
            transform.clone(),
            model_handle,
            BoundingBoxComponent(bounding_box),
            dynamic_bvh_handle,
            motion_controller,
            Pose::default(),
            OrdersController::default(),
        ));

        Ok(entity)
    }

    fn spawn_scenery(
        &self,
        world: &mut World,
        models: &Models,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let model_handle = models.load(ModelName::Object(name.to_string()))?;
        let model = models.get(model_handle).unwrap();

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
                StaticBvhHandle,
            ))
            .id())
    }

    fn spawn_structure(
        &self,
        world: &mut World,
        models: &Models,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let model_handle = models.load(ModelName::Object(name.to_string()))?;
        let model = models.get(model_handle).unwrap();

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
                StaticBvhHandle,
            ))
            .id())
    }

    fn spawn_vehicle(
        &self,
        world: &mut World,
        models: &Models,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        self.spawn_structure(world, models, title, name, object_type, transform)
    }
}

fn build_body_definition_model(
    body_definition: &BodyDefinition,
    models: &Models,
) -> Result<Model, AssetError> {
    let images = models.images();

    let (body_skeleton, body_meshes, body_collision_boxes, body_bounding_box, body_name_lookup) = {
        let body_handle = models.load(ModelName::Body(body_definition.body_model.clone()))?;
        let body_model = models.get(body_handle).unwrap();
        (
            body_model.skeleton.clone(),
            body_model
                .meshes
                .iter()
                .map(|mesh| (mesh.node_index, mesh.mesh.clone()))
                .collect::<Vec<_>>(),
            body_model.collision_boxes.clone(),
            body_model.bounding_box,
            body_model.name_lookup.clone(),
        )
    };

    let (head_meshes, head_collision_boxes, head_bounding_box) = {
        let head_handle = models.load(ModelName::Head(body_definition.head_model.clone()))?;
        let head_model = models.get(head_handle).unwrap();
        (
            head_model
                .meshes
                .iter()
                .map(|mesh| (mesh.node_index, mesh.mesh.clone()))
                .collect::<Vec<_>>(),
            head_model.collision_boxes.clone(),
            head_model.bounding_box,
        )
    };

    let mut new_model = Model {
        skeleton: body_skeleton,
        meshes: Vec::with_capacity(body_meshes.len() + head_meshes.len()),
        collision_boxes: Vec::with_capacity(
            body_collision_boxes.len() + head_collision_boxes.len(),
        ),
        bounding_box: BoundingBox::default(),
        name_lookup: body_name_lookup,
    };

    // Merge the body.
    let body_image_name = body_definition.body_map.clone();
    let body_image_path = PathBuf::from("textures")
        .join("shared")
        .join(&body_image_name);
    let body_image = images.load(&body_image_path)?;
    for (node_index, mesh) in body_meshes {
        new_model.meshes.push(Mesh {
            node_index,
            image_name: body_image_name.clone(),
            image: body_image,
            mesh,
        });
    }
    new_model.collision_boxes.extend(body_collision_boxes);
    new_model.bounding_box.expand_to_include(&body_bounding_box);

    // Merge the head model meshes.
    let head_image_name = body_definition.head_map.clone();
    let head_image_path = PathBuf::from("textures")
        .join("shared")
        .join(&head_image_name);
    let head_image = images.load(&head_image_path)?;
    for (node_index, mesh) in head_meshes {
        new_model.meshes.push(Mesh {
            node_index,
            image_name: head_image_name.clone(),
            image: head_image,
            mesh,
        });
    }
    new_model.collision_boxes.extend(head_collision_boxes);
    new_model.bounding_box.expand_to_include(&head_bounding_box);

    Ok(new_model)
}
