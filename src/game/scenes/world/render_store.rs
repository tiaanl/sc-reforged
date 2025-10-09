use std::ops::Range;

use crate::{
    engine::{
        mesh::{GpuIndexedMesh, IndexedMesh},
        prelude::Renderer,
        storage::{Handle, Storage},
    },
    game::{image::images, model::Model, scenes::world::render_world::RenderWorld},
};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub node_index: u32,
}

pub struct ModelMesh {
    mesh: GpuIndexedMesh,
    opaque_range: Range<u32>,
    alpha_range: Range<u32>,
    additive_range: Range<u32>,
}

pub struct ModelTexture {}

pub struct RenderStore {
    pub camera_bind_group_layout: wgpu::BindGroupLayout,

    pub meshes: Storage<ModelMesh>,
    pub textures: Storage<ModelTexture>,
}

impl RenderStore {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_bind_group_layout = RenderWorld::create_camera_bind_group_layout(renderer);

        let meshes = Storage::default();
        let textures = Storage::default();

        Self {
            camera_bind_group_layout,
            meshes,
            textures,
        }
    }

    pub fn add_model(&mut self, model: &Model, renderer: &Renderer) -> Handle<ModelMesh> {
        let mut opaque_mesh = IndexedMesh::default();
        let mut alpha_mesh = IndexedMesh::default();
        let mut additive_mesh = IndexedMesh::default();

        // TODO: The vertex format conversion seems like a waste here. Resuse the one from .smf?

        for mesh in model.meshes.iter() {
            if let Some(image) = images().get(mesh.image) {
                let _ = match image.blend_mode {
                    crate::game::image::BlendMode::Opaque
                    | crate::game::image::BlendMode::ColorKeyed => &mut opaque_mesh,
                    crate::game::image::BlendMode::Alpha => &mut alpha_mesh,
                    crate::game::image::BlendMode::Additive => &mut additive_mesh,
                }
                .extend(mesh.mesh.map(|v| Vertex {
                    position: v.position.to_array(),
                    normal: v.normal.to_array(),
                    tex_coord: v.tex_coord.to_array(),
                    node_index: v.node_index,
                }));
            }
        }

        let mut full_mesh = IndexedMesh::default();
        let opaque_range = full_mesh.extend(opaque_mesh);
        let alpha_range = full_mesh.extend(alpha_mesh);
        let additive_range = full_mesh.extend(additive_mesh);

        let mesh = full_mesh.to_gpu(&renderer.device);

        self.meshes.insert(ModelMesh {
            mesh,
            opaque_range,
            alpha_range,
            additive_range,
        })
    }
}
