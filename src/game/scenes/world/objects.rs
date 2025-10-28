use std::path::PathBuf;

use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        data_dir::data_dir,
        image::images,
        math::{BoundingSphere, RaySegment},
        model::{Mesh, Model, ModelRayHit},
        models::{ModelName, models},
        scenes::world::{render::RenderStore, systems::RenderWrapper},
    },
};

pub enum ObjectData {
    Scenery {
        model: Handle<Model>,
    },
    Biped {
        model: Handle<Model>,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel {
        model: Handle<Model>,
    },
}

pub struct Object {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
    pub data: ObjectData,
}

impl Object {
    pub fn gather_models_to_render(&self, renderer: &mut RenderWrapper) {
        match self.data {
            ObjectData::Scenery { model } => renderer.render_model(self.transform.to_mat4(), model),

            ObjectData::Biped { model } => {
                renderer.render_model(self.transform.to_mat4(), model);
            }

            ObjectData::SingleModel { model } => {
                renderer.render_model(self.transform.to_mat4(), model)
            }
        }
    }

    /// Intersect this object with a world-space ray segment using the model's collision boxes.
    /// Returns Some((t, world_position)) for the closest hit, or None if no hit.
    pub fn ray_intersection(&self, ray_segment: &RaySegment) -> Option<ModelRayHit> {
        // Quad tree already applied coarse culling; do only fine model test here.
        let object_to_world = self.transform.to_mat4();

        let model_handle = match &self.data {
            ObjectData::Scenery { model }
            | ObjectData::Biped { model }
            | ObjectData::SingleModel { model } => *model,
        };

        let model = models().get(model_handle)?;
        model.intersect_ray_segment_with_transform(object_to_world, ray_segment)
    }
}

pub struct Objects {
    character_profiles: CharacterProfiles,

    /// A list for all objects iun the world.
    pub objects: Storage<Object>,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
}

impl Objects {
    pub fn new() -> Result<Self, AssetError> {
        let character_profiles = data_dir().load_character_profiles()?;

        Ok(Self {
            character_profiles,
            objects: Storage::default(),
            models_to_prepare: Vec::default(),
        })
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        name: &str,
        title: &str,
    ) -> Result<(Handle<Object>, &Object), AssetError> {
        let mut bounding_sphere = BoundingSphere::ZERO;

        let object_data = match object_type {
            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree
            | ObjectType::Structure
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
                let (model_handle, model) = models().load_model(ModelName::Object(name.into()))?;

                bounding_sphere.expand_to_include(&model.bounding_sphere);

                self.models_to_prepare.push(model_handle);

                ObjectData::Scenery {
                    model: model_handle,
                }
            }

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

                self.models_to_prepare.push(model_handle);

                ObjectData::Biped {
                    model: model_handle,
                }
            }

            _ => {
                let (model_handle, model) = models().load_model(ModelName::Object(name.into()))?;

                bounding_sphere.expand_to_include(&model.bounding_sphere);

                self.models_to_prepare.push(model_handle);

                ObjectData::SingleModel {
                    model: model_handle,
                }
            }
        };

        // Move to bounding sphere into position.
        bounding_sphere.center += transform.translation;

        let handle = self.objects.insert(Object {
            transform,
            bounding_sphere,
            data: object_data,
        });

        Ok((handle, self.objects.get(handle).unwrap()))
    }

    #[inline]
    pub fn get(&self, handle: Handle<Object>) -> Option<&Object> {
        self.objects.get(handle)
    }

    /// Take all models that were used during `spawn` and prepare them to be rendered.
    pub fn prepare_models(&mut self, render_store: &mut RenderStore) {
        if self.models_to_prepare.is_empty() {
            return;
        }

        let mut models_to_prepare = Vec::default();
        std::mem::swap(&mut self.models_to_prepare, &mut models_to_prepare);

        tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

        for model_handle in models_to_prepare {
            if let Err(err) = render_store.get_or_create_render_model(model_handle) {
                tracing::warn!("Could not prepare model! ({err})");
            }
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
