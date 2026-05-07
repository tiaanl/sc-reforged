use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use ahash::HashMap;
use glam::Quat;
use shadow_company_tools::smf;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, StorageMap},
        transform::Transform,
    },
    game::{
        assets::{
            images::Images,
            model::{CollisionBox, Mesh, Model, NodeIndex, Vertex},
        },
        config::{LodModelProfileDefinition, SubModelDefinition, load_config},
        globals,
        math::BoundingBox,
        models::ModelName,
        skeleton::{Bone, Skeleton},
    },
};

pub struct Models {
    images: Arc<Images>,
    model_lod_defs: HashMap<String, Vec<SubModelDefinition>>,
    storage: RwLock<StorageMap<ModelName, Model, Arc<Model>>>,
}

impl Models {
    pub fn new(images: Arc<Images>) -> Result<Self, AssetError> {
        let mut model_lod_defs: HashMap<String, Vec<SubModelDefinition>> = HashMap::default();

        let profiles_path = PathBuf::from("config").join("lod_model_profiles");
        for lod_path in globals::file_system().dir(&profiles_path)?.filter(|path| {
            path.extension()
                .filter(|ext| ext.eq_ignore_ascii_case("txt"))
                .is_some()
        }) {
            let profile = load_config::<LodModelProfileDefinition>(lod_path)?;
            model_lod_defs.insert(
                profile.lod_model_name,
                profile.sub_model_definitions.clone(),
            );
        }

        Ok(Self {
            images,
            model_lod_defs,
            storage: RwLock::new(StorageMap::default()),
        })
    }

    pub fn images(&self) -> Arc<Images> {
        Arc::clone(&self.images)
    }

    pub fn get(&self, handle: Handle<Model>) -> Option<Arc<Model>> {
        self.storage.read().unwrap().get(handle).map(Arc::clone)
    }

    pub fn load(&self, name: ModelName) -> Result<Handle<Model>, AssetError> {
        {
            let storage = self.storage.read().unwrap();
            if let Some(handle) = storage.get_handle_by_key(&name) {
                return Ok(handle);
            }
        }

        let path = match &name {
            ModelName::Object(name) => {
                let lod_name = self
                    .model_lod_defs
                    .get(name)
                    .map(|def| def[0].sub_model_model.as_str())
                    .unwrap_or(name);

                PathBuf::from(lod_name).join(lod_name)
            }
            ModelName::Body(name) => PathBuf::from("people").join("bodies").join(name).join(name),
            ModelName::Head(name) => PathBuf::from("people").join("heads").join(name).join(name),
            ModelName::_Misc(name) => PathBuf::from("people").join("misc").join(name).join(name),
            ModelName::BodyDefinition(..) => panic!("Can't load body definition models!"),
        };

        let path = PathBuf::from("models").join(&path).with_extension("smf");

        let data = globals::file_system().load(&path)?;
        let model = self.build_model_from_smf(&path, &data)?;

        if model.meshes.is_empty() {
            return Err(AssetError::custom(path, "Model does not contain meshes!"));
        }

        let handle = {
            let mut storage = self.storage.write().unwrap();
            storage.insert(name, Arc::new(model))
        };

        Ok(handle)
    }

    /// Insert a synthesized model under the given name, returning the existing
    /// handle if one is already registered.
    pub fn insert(&self, name: ModelName, model: Model) -> Handle<Model> {
        {
            let storage = self.storage.read().unwrap();
            if let Some(handle) = storage.get_handle_by_key(&name) {
                return handle;
            }
        }

        let mut storage = self.storage.write().unwrap();
        if let Some(handle) = storage.get_handle_by_key(&name) {
            return handle;
        }

        storage.insert(name, Arc::new(model))
    }

    fn build_model_from_smf(&self, path: &PathBuf, data: &[u8]) -> Result<Model, AssetError> {
        type NameLookup = HashMap<String, NodeIndex>;

        fn smf_mesh_to_mesh(
            smf_mesh: &smf::Mesh,
            node_index: u32,
        ) -> crate::engine::mesh::IndexedMesh<Vertex> {
            let vertices = smf_mesh
                .vertices
                .iter()
                .map(|v| Vertex {
                    position: v.position,
                    normal: -v.normal,
                    tex_coord: v.tex_coord,
                    node_index,
                })
                .collect();

            let indices = smf_mesh.faces.iter().flat_map(|i| i.indices).collect();

            crate::engine::mesh::IndexedMesh { vertices, indices }
        }

        let mut reader = std::io::Cursor::new(data);
        let smf = shadow_company_tools::smf::Model::read(&mut reader)
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let mut nodes = Vec::with_capacity(smf.nodes.len());
        let mut meshes = Vec::default();
        let mut collision_boxes = Vec::new();
        let mut names = NameLookup::default();

        for (node_index, smf_node) in smf.nodes.into_iter().enumerate() {
            names.insert(smf_node.name.clone(), node_index as u32);

            let parent_node_index = if smf_node.parent_name == "<root>" {
                NodeIndex::MAX
            } else {
                match names.get(&smf_node.parent_name) {
                    Some(id) => *id,
                    None => {
                        let n = names.keys().cloned().collect::<Vec<_>>().join(", ");
                        return Err(AssetError::custom(
                            &smf.name,
                            format!(
                                "Parent name [{}] not found, existing names: {}",
                                smf_node.parent_name, n
                            ),
                        ));
                    }
                }
            };

            nodes.push(LocalNode {
                parent: parent_node_index,
                transform: Transform::new(smf_node.position, Quat::IDENTITY),
                bone_id: smf_node.tree_id,
                name: smf_node.name.clone(),
            });

            for smf_mesh in smf_node.meshes.iter() {
                let mesh = smf_mesh_to_mesh(smf_mesh, node_index as u32);

                let texture_path = PathBuf::from("textures")
                    .join("shared")
                    .join(&smf_mesh.texture_name);

                let image_handle = match self.images.load(texture_path) {
                    Ok(handle) => handle,
                    Err(err) => {
                        tracing::warn!("Could not load mesh texture: {}", err);
                        continue;
                    }
                };

                meshes.push(Mesh {
                    node_index: node_index as u32,
                    image_name: smf_mesh.texture_name.clone(),
                    image: image_handle,
                    mesh,
                });
            }

            for smf_collision_box in smf_node.bounding_boxes.iter() {
                collision_boxes.push(CollisionBox {
                    node_index: node_index as u32,
                    min: smf_collision_box.min,
                    max: smf_collision_box.max,
                });
            }
        }

        let skeleton = Skeleton {
            bones: nodes
                .iter()
                .map(|node| Bone {
                    parent: node.parent,
                    transform: node.transform.clone(),
                    id: node.bone_id,
                    _name: node.name.clone(),
                })
                .collect(),
        };

        let mut bounding_box = BoundingBox::default();

        for mesh in meshes.iter() {
            let local = skeleton.local_transform(mesh.node_index);
            mesh.mesh.vertices.iter().for_each(|v| {
                let local = local.transform_point3(v.position);
                bounding_box.expand(local);
            });
        }

        Ok(Model {
            skeleton,
            meshes,
            collision_boxes,
            bounding_box,
            name_lookup: names,
        })
    }
}

struct LocalNode {
    parent: NodeIndex,
    transform: Transform,
    bone_id: u32,
    name: String,
}
