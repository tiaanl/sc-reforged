use glam::{Mat4, Quat, Vec3};
use shadow_company_tools::smf;
use tracing::warn;

use crate::engine::{
    arena::{Arena, Handle},
    assets::Assets,
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{RenderPipelineConfig, Renderer},
};

use super::textures::Textures;

#[derive(Debug)]
pub struct Mesh {
    pub mesh: GpuMesh,
    pub texture: Handle<wgpu::BindGroup>,
}

#[derive(Debug)]
pub struct Node {
    pub position: Vec3,
    pub rotation: Quat,
    pub meshes: Vec<Mesh>,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct Model {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct RenderInfo {
    position: Vec3,
    rotation: Vec3,
    handle: Handle<Model>,
}

impl RenderInfo {
    pub fn new(position: Vec3, rotation: Vec3, handle: Handle<Model>) -> Self {
        Self {
            position,
            rotation,
            handle,
        }
    }
}

pub struct Models {
    render_pipeline: wgpu::RenderPipeline,

    textures: Textures,
    models: Arena<Model>,
}

impl Models {
    pub fn new(renderer: &Renderer) -> Self {
        let shader_module =
            renderer.create_shader_module("model_renderer", include_str!("model.wgsl"));

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        let textures = Textures::default();

        let models = Arena::default();

        Self {
            render_pipeline,
            textures,
            models,
        }
    }

    pub fn insert(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        smf: &smf::Model,
    ) -> Handle<Model> {
        let model = self
            .smf_to_model(renderer, assets, smf)
            .expect("Could not load model! FIX THIS");
        self.models.insert(model)
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        textures: &Textures,
        camera_bind_group: &wgpu::BindGroup,
        batch: &[RenderInfo],
    ) {
        let mut render_pass = Self::create_render_pass(renderer, encoder, output);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        batch.iter().for_each(|render_info| {
            let Some(model) = self.models.get(&render_info.handle) else {
                return;
            };

            model.nodes.iter().for_each(|node| {
                self.render_node(
                    renderer,
                    &mut render_pass,
                    node,
                    render_info.position,
                    glam::Quat::from_euler(
                        glam::EulerRot::XYZ,
                        render_info.rotation.x,
                        render_info.rotation.y,
                        render_info.rotation.z,
                    ),
                );
            });
        });
    }

    fn render_node(
        &self,
        renderer: &Renderer,
        render_pass: &mut wgpu::RenderPass<'_>,
        node: &Node,
        position: Vec3,
        rotation: Quat,
    ) {
        {
            let model_matrix = Mat4::from_rotation_translation(rotation, position);
            let data = model_matrix.to_cols_array_2d();
            let buffer = renderer.create_uniform_buffer("model_matrix_buffer", data);
            let model_bind_group = renderer.create_uniform_bind_group("model_matrix", &buffer);

            render_pass.set_bind_group(1, &model_bind_group, &[]);

            node.meshes.iter().for_each(|mesh| {
                if let Some(texture_bind_group) = self.textures.get(&mesh.texture) {
                    render_pass.set_bind_group(2, texture_bind_group, &[]);
                    render_pass.draw_mesh(&mesh.mesh);
                } else {
                    warn!("Could not find model texture!");
                }
            });
        }

        for node in node.children.iter() {
            self.render_node(
                renderer,
                render_pass,
                node,
                position + node.position,
                rotation * node.rotation,
            )
        }
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

impl Models {
    fn smf_to_model(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        smf: &smf::Model,
    ) -> Option<Model> {
        let nodes = self.children_of(renderer, assets, &smf.nodes, "<root>");
        if nodes.len() != 1 {
            panic!("Only a single root node is supported!");
        }

        Some(Model { nodes })
    }

    fn children_of(
        &mut self,
        renderer: &Renderer,
        assets: &Assets,
        nodes: &[smf::Node],
        parent_name: &str,
    ) -> Vec<Node> {
        // Find all the child nodes.
        nodes
            .iter()
            .filter(|node| node.parent_name == parent_name)
            .map(|node| Node {
                position: node.position,
                rotation: node.rotation,
                meshes: node
                    .meshes
                    .iter()
                    .map(|mesh| self.smf_mesh_to_mesh(renderer, assets, mesh))
                    .collect(),
                children: self.children_of(renderer, assets, nodes, &node.name),
            })
            .collect()
    }

    fn smf_mesh_to_mesh(&mut self, renderer: &Renderer, assets: &Assets, mesh: &smf::Mesh) -> Mesh {
        let vertices = mesh
            .vertices
            .iter()
            .map(|v| crate::engine::mesh::Vertex {
                position: v.position,
                normal: v.normal,
                tex_coord: v.tex_coord,
            })
            .collect::<Vec<_>>();

        let indices = mesh
            .faces
            .iter()
            .flat_map(|i| i.indices)
            .collect::<Vec<_>>();

        let gpu_mesh = crate::engine::mesh::Mesh { vertices, indices }.to_gpu(renderer);

        let texture_path = std::path::PathBuf::from("textures")
            .join("shared")
            .join(&mesh.texture_name);

        let texture_handle = self
            .textures
            .get_by_path_or_insert(texture_path, |path| {
                let image = match assets.load_bmp(path) {
                    Ok(image) => image,
                    Err(e) => {
                        warn!("Could not load texture! {:?}", e);
                        return None;
                    }
                };
                let texture_view = renderer.create_texture_view(path.to_str().unwrap(), image);

                // TODO: Reuse a sampler.
                let sampler = renderer.create_sampler(
                    "texture_sampler",
                    wgpu::AddressMode::ClampToEdge,
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Linear,
                );

                let bind_group = renderer.create_texture_bind_group(
                    path.to_str().unwrap(),
                    &texture_view,
                    &sampler,
                );

                Some(bind_group)
            })
            .expect("Could not create texture");

        Mesh {
            mesh: gpu_mesh,
            texture: texture_handle,
        }
    }
}

/*
fn smf_to_model(
    renderer: &Renderer,
    assets: &Assets,
    textures: &mut textures::Textures,
    smf: smf::Model,
) -> Result<models::Model, AssetError> {
    fn do_node(
        renderer: &Renderer,
        assets: &Assets,
        textures: &mut textures::Textures,
        nodes: &[smf::Node],
        parent_node_name: &str,
    ) -> Vec<models::Node> {
        fn do_mesh(
            renderer: &Renderer,
            assets: &Assets,
            textures: &mut textures::Textures,
            mesh: &smf::Mesh,
        ) -> Result<models::ModelMesh, ()> {
            // Load the texture
            let texture_path = PathBuf::from("textures")
                .join("shared")
                .join(&mesh.texture_name);
            let texture_handle = textures.get_by_path_or_insert(texture_path, |path| {
                let image = match assets.load_bmp(path) {
                    Ok(image) => image,
                    Err(e) => {
                        warn!("Could not load texture! {:?}", e);
                        return None;
                    }
                };
                let texture_view = renderer.create_texture_view(path.to_str().unwrap(), image);

                // TODO: Reuse a sampler.
                let sampler = renderer.create_sampler(
                    "texture_sampler",
                    wgpu::AddressMode::ClampToEdge,
                    wgpu::FilterMode::Linear,
                    wgpu::FilterMode::Linear,
                );

                let bind_group = renderer.create_texture_bind_group(
                    path.to_str().unwrap(),
                    &texture_view,
                    &sampler,
                );

                Some(bind_group)
            })?;

            let mesh = mesh::Mesh {
                vertices: mesh
                    .vertices
                    .iter()
                    .map(|v| mesh::Vertex {
                        position: v.position,
                        normal: v.normal,
                        tex_coord: v.tex_coord,
                    })
                    .collect(),
                indices: mesh.faces.iter().flat_map(|f| f.indices).collect(),
            }
            .to_gpu(renderer);

            Ok(models::ModelMesh {
                mesh,
                texture_handle,
            })
        }

        nodes
            .iter()
            .filter(|node| node.parent_name == parent_node_name)
            .map(|node| models::ModelNode {
                position: node.position,
                rotation: node.rotation,
                meshes: node
                    .meshes
                    .iter()
                    .filter_map(|mesh| do_mesh(renderer, assets, textures, mesh).ok())
                    .collect(),
                children: do_node(renderer, assets, textures, nodes, &node.name),
            })
            .collect()
    }

    Ok(models::Model {
        nodes: do_node(renderer, assets, textures, &smf.nodes, "<root>"),
    })
}
*/
