use std::path::PathBuf;

use crate::{
    engine::{
        assets::AssetError,
        renderer::Renderer,
        storage::{Handle, Storage},
        transform::Transform,
    },
    game::{
        config::{BodyDefinition, CharacterProfiles, ObjectType},
        data_dir::data_dir,
        image::images,
        math::{BoundingBox, RaySegment},
        model::{Mesh, Model, ModelRayHit},
        models::{ModelName, models},
        scenes::world::{
            render::{ModelRenderFlags, RenderStore, RenderWrapper},
            sim_world::{
                orders::{Order, OrderKind},
                sequences::Sequencer,
            },
            systems::InteractionHit,
        },
    },
};

use super::{order_queue::OrderQueue, static_bvh::StaticBvh};

pub enum ObjectData {
    Scenery {
        model: Handle<Model>,
    },
    Biped {
        model: Handle<Model>,
        order_queue: OrderQueue,
        sequencer: Sequencer,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel {
        model: Handle<Model>,
    },
}

impl ObjectData {
    fn interact(&mut self, hit: &InteractionHit) {
        match self {
            ObjectData::Scenery { .. } => {}
            ObjectData::Biped { order_queue, .. } => match hit {
                InteractionHit::Terrain { world_position, .. } => {
                    // User clicked on the terrain, order a move.
                    order_queue.enqueue(
                        OrderKind::Move {
                            world_position: *world_position,
                        }
                        .into(),
                    );
                }
                InteractionHit::Object { .. } => {}
            },
            ObjectData::SingleModel { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RayIntersectionMode {
    _CollisionBoxes,
    Meshes,
}

pub struct Object {
    pub name: String,
    pub title: String,
    pub transform: Transform,
    pub bounding_box: BoundingBox,
    pub data: ObjectData,
}

impl Object {
    pub fn gather_models_to_render(&self, renderer: &mut RenderWrapper, flags: ModelRenderFlags) {
        let model = match self.data {
            ObjectData::Scenery { model } => model,
            ObjectData::Biped { model, .. } => model,
            ObjectData::SingleModel { model } => model,
        };

        renderer.render_model(self.transform.to_mat4(), model, flags);
    }

    /// Intersect this object with a world-space ray segment using the selected model data.
    pub fn ray_intersection(
        &self,
        ray_segment: &RaySegment,
        mode: RayIntersectionMode,
    ) -> Option<ModelRayHit> {
        // Quad tree already applied coarse culling; do only fine model test here.
        let object_to_world = self.transform.to_mat4();

        let model_handle = match &self.data {
            ObjectData::Scenery { model }
            | ObjectData::Biped { model, .. }
            | ObjectData::SingleModel { model } => *model,
        };

        let model = models().get(model_handle)?;
        match mode {
            RayIntersectionMode::_CollisionBoxes => {
                model.intersect_ray_segment_with_transform(object_to_world, ray_segment)
            }
            RayIntersectionMode::Meshes => model.intersect_ray_segment_meshes_with_transform(
                object_to_world,
                ray_segment,
                false,
            ),
        }
    }

    /// The user is interacting with the object.
    pub fn interact(&mut self, hit: &InteractionHit) {
        self.data.interact(hit);
    }
}

pub struct Objects {
    character_profiles: CharacterProfiles,

    /// A list for all objects iun the world.
    pub objects: Storage<Object>,

    pub static_bvh: StaticBvh,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
}

impl Objects {
    pub fn new() -> Result<Self, AssetError> {
        let character_profiles = data_dir().load_character_profiles()?;

        let static_bvh = StaticBvh::new(8);

        Ok(Self {
            character_profiles,
            objects: Storage::default(),
            static_bvh,
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
        let mut bounding_box = BoundingBox::default();

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

                bounding_box.expand_to_include(&model.bounding_box);

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

                bounding_box.expand_to_include(&model.bounding_box);

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
                    order_queue: OrderQueue::default(),
                    sequencer: Sequencer::default(),
                }
            }

            _ => {
                let (model_handle, model) = models().load_model(ModelName::Object(name.into()))?;

                bounding_box.expand_to_include(&model.bounding_box);

                self.models_to_prepare.push(model_handle);

                ObjectData::SingleModel {
                    model: model_handle,
                }
            }
        };

        debug_assert!(bounding_box.is_valid(), "invalid bounding box for {name:?}");

        let handle = self.objects.insert(Object {
            name: name.to_string(),
            title: title.to_string(),
            transform,
            bounding_box,
            data: object_data,
        });

        Ok((handle, self.objects.get(handle).unwrap()))
    }

    #[inline]
    pub fn get(&self, handle: Handle<Object>) -> Option<&Object> {
        self.objects.get(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<Object>) -> Option<&mut Object> {
        self.objects.get_mut(handle)
    }

    pub fn finalize(&mut self) {
        let bounding_boxes = self
            .objects
            .iter()
            .map(|(handle, object)| {
                (
                    handle,
                    object.bounding_box.transformed(object.transform.to_mat4()),
                )
            })
            .collect::<Vec<_>>();

        self.static_bvh.rebuild(&bounding_boxes);
    }

    /// Take all models that were used during `spawn` and prepare them to be rendered.
    pub fn prepare_models(&mut self, renderer: &Renderer, render_store: &mut RenderStore) {
        if self.models_to_prepare.is_empty() {
            return;
        }

        let mut models_to_prepare = Vec::default();
        std::mem::swap(&mut self.models_to_prepare, &mut models_to_prepare);

        tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

        for model_handle in models_to_prepare {
            if let Err(err) = render_store.get_or_create_render_model(renderer, model_handle) {
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
}
