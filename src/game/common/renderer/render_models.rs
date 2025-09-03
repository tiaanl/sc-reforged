use ahash::HashMap;
use glam::{Vec2, Vec3};

use super::render_textures::{RenderTexture, RenderTextures};
use crate::{
    engine::{
        assets::AssetError,
        mesh::{GpuIndexedMesh, IndexedMesh},
        storage::{Handle, Storage},
    },
    game::{
        image::{BlendMode, Image, images},
        math::BoundingSphere,
        model::Model,
        models::models,
        renderer::{
            render_animations::{RenderAnimation, RenderAnimations},
            render_textures::RenderTextureSet,
        },
    },
};

type NodeIndex = u32;

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct RenderVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
    pub node_index: NodeIndex,
    pub texture_index: u32,
}

pub struct RenderModel {
    /// Opaque mesh data.
    pub opaque_mesh: Option<GpuIndexedMesh>,
    /// Alpha mesh data if there are any meshes with alpha data.
    pub alpha_mesh: Option<GpuIndexedMesh>,
    /// Additive mesh data if there are any meshes with alpha data.
    pub additive_mesh: Option<GpuIndexedMesh>,
    /// A [BoundingSphere] that wraps the entire model. Used for culling.
    pub bounding_sphere: BoundingSphere,
    /// All the textures used by the model.
    pub texture_set: RenderTextureSet,
    /// A [RenderAnimation] with a single frame that represents the model at rest.
    pub rest_pose: Handle<RenderAnimation>,
}

#[derive(Default)]
pub struct RenderModels {
    models: Storage<RenderModel>,

    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,
}

impl RenderModels {
    pub fn add_model(
        &mut self,
        render_textures: &mut RenderTextures,
        animations: &mut RenderAnimations,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let model = models()
            .get(model_handle)
            .expect("Model should have been loaded byt his time.");

        let mut textures: Vec<Handle<RenderTexture>> = Vec::with_capacity(8);

        let mut opaque_mesh = IndexedMesh::default();
        let mut alpha_mesh = IndexedMesh::default();
        let mut additive_mesh = IndexedMesh::default();

        let mut image_to_index: HashMap<Handle<Image>, u32> = HashMap::default();

        for mesh in model.meshes.iter() {
            let texture_index = {
                if let Some(texture_index) = image_to_index.get(&mesh.image) {
                    *texture_index
                } else {
                    let texture_handle = render_textures.add(mesh.image);
                    let texture_index = textures.len() as u32;
                    textures.push(texture_handle);

                    image_to_index.insert(mesh.image, texture_index);

                    texture_index
                }
            };

            // Safety: We just created a texture successfully, so the image *MUST* exist already.
            let image = images().get(mesh.image).unwrap();

            let vertices = mesh
                .mesh
                .vertices
                .iter()
                .map(|v| RenderVertex {
                    position: v.position,
                    normal: v.normal,
                    tex_coord: v.tex_coord,
                    node_index: v.node_index,
                    texture_index,
                })
                .collect();

            let indexed_mesh = IndexedMesh::new(vertices, mesh.mesh.indices.clone());

            match image.blend_mode {
                BlendMode::Opaque | BlendMode::ColorKeyed => opaque_mesh.extend(indexed_mesh),
                BlendMode::Alpha => alpha_mesh.extend(indexed_mesh),
                BlendMode::Additive => additive_mesh.extend(indexed_mesh),
            };
        }

        if opaque_mesh.is_empty() {
            tracing::warn!("Mesh with no vertices or indices! Should be checked up-front!");
        }

        let opaque_mesh = if !opaque_mesh.is_empty() {
            Some(opaque_mesh.to_gpu())
        } else {
            None
        };

        let alpha_mesh = if !alpha_mesh.is_empty() {
            Some(alpha_mesh.to_gpu())
        } else {
            None
        };

        let additive_mesh = if !additive_mesh.is_empty() {
            Some(additive_mesh.to_gpu())
        } else {
            None
        };

        let texture_set = render_textures.create_texture_set(textures);

        let rest_pose = animations.create_rest_pose(&model.skeleton);

        let render_model_handle = self.models.insert(RenderModel {
            opaque_mesh,
            alpha_mesh,
            additive_mesh,
            bounding_sphere: model.bounding_sphere,
            texture_set,
            rest_pose,
        });

        self.model_to_render_model
            .insert(model_handle, render_model_handle);

        Ok(render_model_handle)
    }

    #[inline]
    pub fn get(&self, handle: Handle<RenderModel>) -> Option<&RenderModel> {
        self.models.get(handle)
    }

    #[inline]
    pub fn _get_mut(&mut self, handle: Handle<RenderModel>) -> Option<&mut RenderModel> {
        self.models.get_mut(handle)
    }
}
