use std::path::PathBuf;

use bevy_ecs::prelude::*;

use crate::{
    engine::{assets::AssetError, storage::Handle, transform::Transform},
    game::{
        AssetLoader,
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        math::BoundingBox,
        model::{Mesh, Model},
        models::ModelName,
        scenes::world::sim_world::{DynamicBvh, Order, StaticBvhHandle, ecs::BoundingBoxComponent},
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
        assets: &mut AssetLoader,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        match object_type {
            ObjectType::Ape | ObjectType::Bipedal => {
                self.spawn_bipedal(world, assets, title, name, object_type, transform)
            }

            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree => {
                self.spawn_scenery(world, assets, title, name, object_type, transform)
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
                self.spawn_structure(world, assets, title, name, object_type, transform)
            }

            ObjectType::SixBySix
            | ObjectType::SnowMobile
            | ObjectType::Treaded
            | ObjectType::TreadedBMP2
            | ObjectType::TreadedChallenger
            | ObjectType::TreadedScorpion
            | ObjectType::TreadedT55 => {
                self.spawn_vehicle(world, assets, title, name, object_type, transform)
            }

            // These are unknown at this time.
            ObjectType::Bird
            | ObjectType::Boat
            | ObjectType::FourByFour
            | ObjectType::Helicopter
            | ObjectType::Howitzer
            | ObjectType::SentryGun => {
                self.spawn_structure(world, assets, title, name, object_type, transform)
            }
        }
    }

    fn spawn_bipedal(
        &mut self,
        world: &mut World,
        assets: &mut AssetLoader,
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

        let model = build_body_definition_model(body_definition, assets)?;

        let bounding_box = model.bounding_box;

        let (model_handle, _) = assets.add_model(
            ModelName::BodyDefinition(
                character_profile.character.clone(),
                body_definition.body_type.clone(),
            ),
            model,
        );

        self.models_to_prepare.push(model_handle);

        let entity = world.spawn_empty().id();

        let dynamic_bvh_handle = {
            let mut dynamic_bvh = world.resource_mut::<DynamicBvh>();
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
        assets: &mut AssetLoader,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let (model_handle, model) =
            assets.get_or_load_model(ModelName::Object(name.to_string()))?;

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
        assets: &mut AssetLoader,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        let (model_handle, model) =
            assets.get_or_load_model(ModelName::Object(name.to_string()))?;

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
        assets: &mut AssetLoader,
        title: &str,
        name: &str,
        object_type: ObjectType,
        transform: Transform,
    ) -> Result<Entity, AssetError> {
        self.spawn_structure(world, assets, title, name, object_type, transform)
    }
}

fn build_body_definition_model(
    body_definition: &BodyDefinition,
    assets: &mut AssetLoader,
) -> Result<Model, AssetError> {
    let (body_skeleton, body_meshes, body_collision_boxes, body_bounding_box, body_name_lookup) = {
        let (_, body_model) =
            assets.get_or_load_model(ModelName::Body(body_definition.body_model.clone()))?;
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
        let (_, head_model) =
            assets.get_or_load_model(ModelName::Head(body_definition.head_model.clone()))?;
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
    let (body_image, _) = assets.get_or_load_image(&body_image_path)?;
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
    let (head_image, _) = assets.get_or_load_image(&head_image_path)?;
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
