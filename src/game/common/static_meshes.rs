use crate::{engine::assets::AssetStorage, Asset, Handle, Renderer, Vertex};

use super::{assets::Image, mesh_renderer::BlendMode};

pub struct StaticMeshes {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    textures: AssetStorage<Texture>,
    meshes: AssetStorage<Mesh>,
}

pub struct Texture {
    view: wgpu::TextureView,
    blend_mode: BlendMode,
}

impl Asset for Texture {}

struct Lod {
    indices: std::ops::Range<u32>,
}

pub struct Mesh {
    lods: Vec<Lod>,
    texture: Handle<Texture>,
}

impl Asset for Mesh {}

impl StaticMeshes {
    pub fn new(renderer: &Renderer) -> Self {
        Self {
            vertices: Vec::default(),
            indices: Vec::default(),
            textures: AssetStorage::default(),
            meshes: AssetStorage::default(),
        }
    }

    pub fn add_texture(&mut self, renderer: &Renderer, image: &Image) -> Handle<Texture> {
        let view = renderer.create_texture_view("texture", &image.data);
        self.textures.add(Texture {
            view,
            blend_mode: image.blend_mode,
        })
    }

    pub fn add_mesh(&mut self, texture: Handle<Texture>) -> Handle<Mesh> {
        self.meshes.add(Mesh {
            lods: Vec::default(),
            texture,
        })
    }

    pub fn add_lod(&mut self, mesh: Handle<Mesh>, vertices: &[Vertex], indices: &[u32]) {
        if let Some(mesh) = self.meshes.get_mut(mesh) {
            let first_vertex = self.vertices.len() as u32;

            self.vertices.extend_from_slice(vertices);
            self.indices
                .extend(indices.iter().map(|i| *i + first_vertex));

            mesh.lods.push(Lod {
                indices: first_vertex..(first_vertex + indices.len() as u32),
            });
        }
    }
}
