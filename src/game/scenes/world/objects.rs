use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        config::{CharacterProfiles, ObjectType},
        data_dir::data_dir,
        math::BoundingSphere,
        model::Model,
        models::{ModelName, models},
        scenes::world::{render::RenderStore, systems::RenderWrapper},
    },
};

enum ObjectData {
    Scenery {
        model: Handle<Model>,
    },
    Biped {
        body_model: Handle<Model>,
        head_model: Handle<Model>,
        pack_model: Option<Handle<Model>>,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel {
        model: Handle<Model>,
    },
}

pub struct Object {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
    data: ObjectData,
}

impl Object {
    pub fn gather_models_to_render(&self, renderer: &mut RenderWrapper) {
        match self.data {
            ObjectData::Scenery { model } => renderer.render_model(self.transform.to_mat4(), model),

            ObjectData::Biped {
                body_model,
                head_model,
                pack_model,
            } => {
                let transform = self.transform.to_mat4();
                renderer.render_model(transform, body_model);
                renderer.render_model(transform, head_model);
                if let Some(pack_model) = pack_model {
                    renderer.render_model(transform, pack_model);
                }
            }

            ObjectData::SingleModel { model } => {
                renderer.render_model(self.transform.to_mat4(), model)
            }
        }
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

                let body_model = {
                    let (handle, model) =
                        models().load_model(ModelName::Body(body_definition.body_model.clone()))?;
                    self.models_to_prepare.push(handle);
                    bounding_sphere.expand_to_include(&model.bounding_sphere);
                    handle
                };

                let head_model = {
                    let (handle, model) =
                        models().load_model(ModelName::Head(body_definition.head_model.clone()))?;
                    self.models_to_prepare.push(handle);
                    bounding_sphere.expand_to_include(&model.bounding_sphere);
                    handle
                };

                let pack_model = if body_definition.pack_model.is_empty() {
                    None
                } else {
                    let (handle, model) =
                        models().load_model(ModelName::Misc(String::from("smallpack")))?;
                    self.models_to_prepare.push(handle);
                    bounding_sphere.expand_to_include(&model.bounding_sphere);
                    Some(handle)
                };

                ObjectData::Biped {
                    body_model,
                    head_model,
                    pack_model,
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
}
