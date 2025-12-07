use std::path::PathBuf;

use bevy_ecs::world::World;

use crate::{
    engine::{assets::AssetError, prelude::Transform},
    game::{
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        image::images,
        math::BoundingSphere,
        model::{Mesh, Model},
        models::{ModelName, models},
    },
};

pub struct Spawner<'a> {
    /// Extra data used to spawn characters (ObjectType::Bipedal).
    character_profiles: CharacterProfiles,

    /// The world we will spawn objects into.
    world: &'a mut World,
}

impl<'a> Spawner<'a> {
    pub fn new(character_profiles: CharacterProfiles, world: &'a mut World) -> Self {
        Self {
            character_profiles,
            world,
        }
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        name: &str,
        title: &str,
    ) -> Result<(), AssetError> {
        match object_type {
            ObjectType::Bipedal => {
                let Some(character_profile) = self.character_profiles.get(title) else {
                    tracing::warn!("Character profile not found! ({title})");
                    return Err(AssetError::Custom(
                        std::path::PathBuf::new(),
                        String::from("Character profile not found!"),
                    ));
                };

                let body_initial = character_profile.body_initial.as_str();
                let Some(body_definition) = character_profile.body_definitions.get(body_initial)
                else {
                    return Err(AssetError::Custom(
                        std::path::PathBuf::new(),
                        String::from("Could not find initial body definition!"),
                    ));
                };

                let model = Self::build_body_definition_model(body_definition)?;
                let model_handle = models().add(
                    ModelName::BodyDefinition(
                        character_profile.character.clone(),
                        body_definition.body_type.clone(),
                    ),
                    model,
                );

                self.world.spawn((transform, model_handle));
            }

            _ => {
                let (model_handle, _model) = models().load_model(ModelName::Object(name.into()))?;

                self.world.spawn((transform, model_handle));
            }
        }

        Ok(())
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
            bounding_sphere: BoundingSphere::ZERO,
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
            .bounding_sphere
            .expand_to_include(&body_model.bounding_sphere);

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
            .bounding_sphere
            .expand_to_include(&head_model.bounding_sphere);

        Ok(new_model)
    }
}
