use glam::{Mat4, Quat, Vec3};
use shadow_company_tools::smf;

use crate::engine::{
    assets::{Asset, AssetLoader, Assets, Handle},
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{RenderPipelineConfig, Renderer},
    shaders::Shaders,
};

#[derive(Debug)]
pub struct Mesh {
    pub gpu_mesh: GpuMesh,
    pub texture: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct BoundingBox {
    _min: Vec3,
    _max: Vec3,
}

#[derive(Debug)]
pub struct Node {
    pub position: Vec3,
    pub rotation: Quat,
    pub meshes: Vec<Mesh>,
    pub children: Vec<Node>,
    pub _bounding_boxes: Vec<BoundingBox>,
}

#[derive(Debug)]
pub struct Model {
    pub nodes: Vec<Node>,
}

impl Asset for Model {}

#[derive(Debug)]
pub struct RenderInfo {
    pub position: Vec3,
    pub rotation: Vec3,
    pub handle: Handle<Model>,
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

pub struct ModelRenderer {
    render_pipeline: wgpu::RenderPipeline,
    models: Assets<Model>,
}

impl ModelRenderer {
    pub fn new(renderer: &Renderer, shaders: &mut Shaders) -> Self {
        let shader_module = shaders.create_shader(
            renderer,
            "model_renderer",
            include_str!("model.wgsl"),
            "model.wgsl",
        );

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .blend_state(wgpu::BlendState::ALPHA_BLENDING)
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.uniform_bind_group_layout())
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        Self {
            render_pipeline,
            models: Assets::default(),
        }
    }

    pub fn add(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
        smf: &smf::Model,
    ) -> Handle<Model> {
        let model = self
            .smf_to_model(renderer, assets, smf)
            .expect("Could not load model! FIX THIS");

        self.models.add(model)
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        batch: &[RenderInfo],
        // bounding_boxes: &mut BoundingBoxes,
        load: wgpu::LoadOp<wgpu::Color>,
    ) {
        let mut render_pass = Self::create_render_pass(renderer, encoder, output, load);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        batch.iter().for_each(|render_info| {
            let Some(model) = self.models.get(&render_info.handle) else {
                return;
            };

            let transform = Mat4::from_rotation_translation(
                Quat::from_euler(
                    glam::EulerRot::XYZ,
                    -render_info.rotation.x,
                    -render_info.rotation.y,
                    -render_info.rotation.z,
                ),
                render_info.position,
            );

            model.nodes.iter().for_each(|node| {
                self.render_node(
                    renderer,
                    &mut render_pass,
                    node,
                    transform,
                    // bounding_boxes,
                );
            });
        });
    }

    fn render_node(
        &self,
        renderer: &Renderer,
        render_pass: &mut wgpu::RenderPass<'_>,
        node: &Node,
        transform: Mat4,
        // bounding_boxes: &mut BoundingBoxes,
    ) {
        // Apply the node's transform to the incoming transform.
        let transform = transform * Mat4::from_rotation_translation(node.rotation, node.position);

        {
            let buffer =
                renderer.create_uniform_buffer("model_matrix_buffer", transform.to_cols_array_2d());
            let model_bind_group = renderer.create_uniform_bind_group("model_matrix", &buffer);

            render_pass.set_bind_group(1, &model_bind_group, &[]);

            node.meshes.iter().for_each(|mesh| {
                render_pass.set_bind_group(2, &mesh.texture, &[]);
                render_pass.draw_mesh(&mesh.gpu_mesh);
            });

            // for b in node.bounding_boxes.iter() {
            //     bounding_boxes.insert(position, rotation, b.min, b.max);
            // }
        }

        for node in node.children.iter() {
            self.render_node(
                renderer,
                render_pass,
                node,
                transform,
                // bounding_boxes,
            )
        }
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Clear(1.0)),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

impl ModelRenderer {
    fn smf_to_model(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
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
        assets: &AssetLoader,
        nodes: &[smf::Node],
        parent_name: &str,
    ) -> Vec<Node> {
        // Find all the child nodes.
        nodes
            .iter()
            .filter(|node| node.parent_name == parent_name)
            .map(|node| Node {
                position: node.position,
                // rotation: transform_quat * node.rotation,
                rotation: Quat::IDENTITY,
                meshes: node
                    .meshes
                    .iter()
                    .map(|mesh| self.smf_mesh_to_mesh(renderer, assets, mesh))
                    .collect(),
                children: self.children_of(renderer, assets, nodes, &node.name),
                _bounding_boxes: node
                    .bounding_boxes
                    .iter()
                    .map(|b| BoundingBox {
                        _min: b.min,
                        _max: b.max,
                    })
                    .collect(),
            })
            .collect()
    }

    fn smf_mesh_to_mesh(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
        mesh: &smf::Mesh,
    ) -> Mesh {
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

        // TODO: Avoid uploding duplicate textures to the GPU.

        let image = assets
            .load_bmp(&texture_path)
            .expect("Could not load .bmp file.");
        let texture_view = renderer.create_texture_view(texture_path.to_str().unwrap(), image);

        // TODO: Reuse samplers.
        let sampler = renderer.create_sampler(
            "texture_sampler",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let bind_group = renderer.create_texture_bind_group(
            texture_path.to_str().unwrap(),
            &texture_view,
            &sampler,
        );

        Mesh {
            gpu_mesh,
            texture: bind_group,
        }
    }
}
