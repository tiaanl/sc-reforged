use crate::{
    engine::{
        arena::{Arena, Handle},
        assets::{AssetError, Assets},
        gizmos::{GizmoVertex, GizmosRenderer},
        mesh::{Mesh, Vertex},
        renderer::{GpuTexture, Renderer},
        scene::Scene,
    },
    game::{config::CampaignDef, smf},
};
use camera::*;
use glam::{vec3, Quat, Vec2, Vec3};
use terrain::*;
use tracing::info;
use winit::{event::MouseButton, keyboard::KeyCode};

mod camera;
mod model;
mod object;
mod terrain;

#[derive(Default)]
pub struct WorldCamera {
    camera: Camera,

    pub pitch: f32,
    pub yaw: f32,

    pub velocity: Vec3,
}

impl WorldCamera {
    pub fn new(camera: Camera, pitch: f32, yaw: f32, velocity: Vec3) -> Self {
        Self {
            camera: camera,
            pitch,
            yaw,
            velocity,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect_ratio = (width as f32) / (height.max(1) as f32);
    }

    pub fn update(&mut self, delta_time: f32) {
        let ref mut camera = self.camera;

        let v = camera.forward_vector();
        camera.position += v * self.velocity.z * delta_time;

        let v = camera.right_vector();
        camera.position += v * self.velocity.x * delta_time;

        let v = camera.up_vector();
        camera.position += v * self.velocity.y * delta_time;
    }

    fn calculate_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }

    pub fn calculate_matrices(&mut self) -> Matrices {
        self.camera.rotation = self.calculate_rotation();
        self.camera.calculate_matrices()
    }
}

/// The [Scene] that renders the ingame world view.
pub struct WorldScene {
    _campaign_def: CampaignDef,

    world_camera: WorldCamera,

    gpu_camera: GpuCamera,

    terrain: Terrain,
    model_renderer: model::ModelRenderer,

    textures: Arena<GpuTexture>,

    last_mouse_position: Vec2,

    rotating_camera: Option<Vec2>,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,

    scene: smf::Scene,
    scene_texture: Handle<GpuTexture>,
}

impl WorldScene {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let camera_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("camera_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let terrain = Terrain::new(assets, renderer, &camera_bind_group_layout, &campaign_def)?;
        let model_renderer = model::ModelRenderer::new(renderer, &camera_bind_group_layout);

        {
            let data =
                assets.load_config_file(format!("maps\\{}_final.mtf", campaign_def.base_name))?;
            let _mtf = crate::game::config::read_mtf(&data);
        }

        let camera = Camera::new(
            vec3(0.0, 1000.0, 1000.0),
            Quat::IDENTITY,
            45_f32.to_radians(),
            1.0,
            0.1,
            100_000.0,
        );
        let world_camera = WorldCamera::new(camera, 0.0, 0.0, Vec3::ZERO);
        let gpu_camera = GpuCamera::new(renderer);

        let gizmos_renderer = GizmosRenderer::new(renderer, &gpu_camera.bind_group_layout);

        let mut textures = Arena::default();

        let data = assets.load_raw("models\\alvhqd-hummer\\alvhqd-hummer.smf")?;
        let mut cursor = std::io::Cursor::new(data);
        let scene = smf::Scene::read(&mut cursor).unwrap();

        let scene_texture = {
            let crate::engine::assets::Image {
                data,
                width,
                height,
            } = assets.load_bmp("textures\\shared\\hummer.bmp")?;

            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            renderer.queue.write_texture(
                wgpu::ImageCopyTextureBase {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::default(),
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&data),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                size,
            );

            let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("texture_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            textures.insert(GpuTexture {
                texture,
                view,
                sampler,
            })
        };

        Ok(Self {
            _campaign_def: campaign_def,

            world_camera,
            gpu_camera,

            terrain,
            model_renderer,

            textures,

            last_mouse_position: Vec2::ZERO,

            rotating_camera: None,

            gizmos_renderer,
            gizmos_vertices: vec![],

            scene,
            scene_texture,
        })
    }
}

impl Scene for WorldScene {
    fn resize(&mut self, width: u32, height: u32) {
        self.world_camera.resize(width, height);
    }

    fn on_key_pressed(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;

        let mut velocity = self.world_camera.velocity;
        match key {
            KeyCode::KeyW => velocity.z += SPEED,
            KeyCode::KeyS => velocity.z -= SPEED,
            KeyCode::KeyD => velocity.x += SPEED,
            KeyCode::KeyA => velocity.x -= SPEED,
            KeyCode::KeyQ => velocity.y -= SPEED,
            KeyCode::KeyE => velocity.y += SPEED,
            _ => info!("key pressed: {key:?}"),
        }
        self.world_camera.velocity = velocity;
    }

    fn on_key_released(&mut self, key: winit::keyboard::KeyCode) {
        const SPEED: f32 = 100.0;

        let mut velocity = self.world_camera.velocity;
        match key {
            KeyCode::KeyW => velocity.z -= SPEED,
            KeyCode::KeyS => velocity.z += SPEED,
            KeyCode::KeyD => velocity.x -= SPEED,
            KeyCode::KeyA => velocity.x += SPEED,
            KeyCode::KeyQ => velocity.y += SPEED,
            KeyCode::KeyE => velocity.y -= SPEED,
            _ => info!("key released: {key:?}"),
        }
        self.world_camera.velocity = velocity;
    }

    fn on_mouse_moved(&mut self, position: glam::Vec2) {
        self.last_mouse_position = position;

        if let Some(pos) = self.rotating_camera {
            const SENSITIVITY: f32 = 0.25;
            let delta = (pos - self.last_mouse_position) * SENSITIVITY;

            self.world_camera.pitch -= delta.y;
            self.world_camera.yaw -= delta.x;

            self.rotating_camera = Some(self.last_mouse_position);
        }
    }

    fn on_mouse_pressed(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.rotating_camera = Some(self.last_mouse_position);
        }
    }
    fn on_mouse_released(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.rotating_camera = None;
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.world_camera.update(delta_time);
        self.terrain.update(delta_time);

        self.gizmos_vertices = vec![
            GizmoVertex {
                position: [0.0, 0.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            GizmoVertex {
                position: [1_000.0, 0.0, 0.0, 1.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            GizmoVertex {
                position: [0.0, 0.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            GizmoVertex {
                position: [0.0, 1_000.0, 0.0, 1.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            GizmoVertex {
                position: [0.0, 0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            GizmoVertex {
                position: [0.0, 0.0, 1_000.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ];
    }

    fn debug_panel(&mut self, egui: &egui::Context) {
        egui::Window::new("World").show(egui, |ui| {
            egui::Grid::new("world_info").show(ui, |ui| {
                ui.label("pitch");
                ui.add(egui::Label::new(format!("{}", self.world_camera.pitch)));
                ui.end_row();

                ui.label("yaw");
                ui.add(egui::Label::new(format!("{}", self.world_camera.yaw)));
                ui.end_row();
            });

            // ui.heading("Camera");
            // self.camera.debug_panel(ui);
            ui.heading("Terrain");
            self.terrain.debug_panel(ui);
        });
    }

    fn render(
        &mut self,
        renderer: &crate::engine::renderer::Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    ) {
        let matrices = self.world_camera.calculate_matrices();
        self.gpu_camera.upload_matrices(renderer, matrices);

        self.terrain
            .render(renderer, encoder, output, &self.gpu_camera.bind_group);

        fn render_node(
            renderer: &Renderer,
            encoder: &mut wgpu::CommandEncoder,
            view: &wgpu::TextureView,
            camera_bind_group: &wgpu::BindGroup,
            mr: &model::ModelRenderer,
            node: &smf::Node,
            texture: &GpuTexture,
        ) {
            node.meshes
                .iter()
                .map(|mesh| {
                    Mesh {
                        vertices: mesh
                            .vertices
                            .iter()
                            .map(|v| {
                                Vertex::new(
                                    vec3(v.position.x, v.position.z, v.position.y),
                                    vec3(v.normal.x, v.normal.z, v.normal.y),
                                    v.tex_coord,
                                )
                            })
                            .collect::<Vec<_>>(),
                        indices: mesh.faces.iter().flat_map(|f| f.indices).collect(),
                    }
                    .to_gpu(renderer)
                })
                .for_each(|mesh| {
                    mr.render(
                        renderer,
                        encoder,
                        view,
                        camera_bind_group,
                        &mesh,
                        &texture,
                        Vec3::ZERO,
                    );
                });
        }

        if let Some(texture) = self.textures.get(&self.scene_texture) {
            self.scene.nodes.iter().for_each(|node| {
                render_node(
                    renderer,
                    encoder,
                    output,
                    &self.gpu_camera.bind_group,
                    &self.model_renderer,
                    node,
                    &texture,
                )
            });
        }

        let mut more_vertices = self.terrain.render_normals();
        self.gizmos_vertices.append(&mut more_vertices);
        let mut more_vertices = self.terrain.render_normals_lookup();
        self.gizmos_vertices.append(&mut more_vertices);

        self.gizmos_renderer.render(
            renderer,
            encoder,
            output,
            &self.gpu_camera.bind_group,
            &self.gizmos_vertices,
        );
        self.gizmos_vertices.clear();
    }
}
