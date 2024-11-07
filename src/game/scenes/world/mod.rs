use crate::{
    engine::{
        assets::{AssetError, Assets},
        gizmos::{GizmoVertex, GizmosRenderer},
        mesh,
        renderer::Renderer,
        scene::Scene,
    },
    game::{config::CampaignDef, smf},
};
use camera::*;
use glam::{Quat, Vec2, Vec3};
use terrain::*;
use tracing::{info, warn};
use winit::{event::MouseButton, keyboard::KeyCode};

mod camera;
mod models;
mod object;
mod terrain;
mod textures;

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

    objects: object::Objects,

    last_mouse_position: Vec2,

    rotating_camera: Option<Vec2>,

    gizmos_renderer: GizmosRenderer,
    gizmos_vertices: Vec<GizmoVertex>,
}

impl WorldScene {
    pub fn new(
        assets: &Assets,
        renderer: &Renderer,
        campaign_def: CampaignDef,
    ) -> Result<Self, AssetError> {
        tracing::info!("Loading campaign \"{}\"...", campaign_def.title);

        let terrain = Terrain::new(assets, renderer, &campaign_def)?;
        let mut objects = object::Objects::new(renderer);

        {
            // Load the image defs.
            let data = assets.load_raw(r"config\image_defs.txt")?;
            let str = String::from_utf8(data).unwrap();
            let _image_defs = crate::game::config::read_image_defs(&str);
        }

        if false {
            let data =
                assets.load_config_file(format!("maps\\{}_final.mtf", campaign_def.base_name))?;
            let mtf = crate::game::config::read_mtf(&data);

            let textures = &mut objects.textures;
            let models = &mut objects.models;

            let mut to_spawn = mtf
                .objects
                .iter()
                .map(|object| {
                    let data =
                        assets.load_raw(format!(r"models\{}\{}.smf", object.name, object.name))?;
                    let mut c = std::io::Cursor::new(data);
                    let smf = smf::Scene::read(&mut c).map_err(|e| {
                        AssetError::FileSystemError(crate::engine::vfs::FileSystemError::Io(e))
                    })?;

                    let model = Self::smf_to_model(renderer, assets, textures, smf)?;
                    let model_handle = models.insert(model);

                    Ok(object::Object {
                        position: object.position,
                        rotation: object.rotation,
                        model_handle,
                    })
                })
                .filter_map(|maybe: Result<object::Object, AssetError>| match maybe {
                    Ok(object) => Some(object),
                    Err(e) => {
                        warn!("Could not load object: {:?}", e);
                        None
                    }
                })
                .collect::<Vec<_>>();

            for object in to_spawn.drain(..) {
                objects.spawn(object);
            }

            // let data = assets.load_raw(r"models\alvhqd-hummer\alvhqd-hummer.smf")?;
            // let mut c = std::io::Cursor::new(data);
            // let smf = smf::Scene::read(&mut c).map_err(|e| {
            //     AssetError::FileSystemError(crate::engine::vfs::FileSystemError::Io(e))
            // })?;

            // let model = Self::smf_to_model(renderer, assets, &mut textures, smf)?;

            // let object = object::Object {
            //     position: Vec3::ZERO,
            //     rotation: Vec3::ZERO,
            //     model_handle: models.insert(model),
            // };
            // let objects = vec![object];
        }

        {
            let data = assets.load_raw(r"models\alvhqd-hummer\alvhqd-hummer.smf")?;
            let mut cursor = std::io::Cursor::new(data);
            let smf = smf::Scene::read(&mut cursor).expect("Could not load model data.");
            let model = Self::smf_to_model(renderer, assets, &mut objects.textures, smf)?;
            let model_handle = objects.models.insert(model);
            objects.spawn(object::Object::new(Vec3::ZERO, Vec3::ZERO, model_handle));
        }

        // 6865.0	12544.0	5602.0	11550.0
        // let initial = vec3(6865.0, 12544.0, 5602.0);
        let initial = Vec3::ZERO;

        let camera = Camera::new(
            // vec3(0.0, 1000.0, 1000.0),
            initial,
            Quat::IDENTITY,
            45_f32.to_radians(),
            1.0,
            0.1,
            100_000.0,
        );
        let world_camera = WorldCamera::new(camera, 0.0, 0.0, Vec3::ZERO);
        let gpu_camera = GpuCamera::new(renderer);

        let gizmos_renderer = GizmosRenderer::new(renderer);

        Ok(Self {
            _campaign_def: campaign_def,

            world_camera,
            gpu_camera,

            terrain,
            objects,

            last_mouse_position: Vec2::ZERO,

            rotating_camera: None,

            gizmos_renderer,
            gizmos_vertices: vec![],
        })
    }

    fn smf_to_model(
        renderer: &Renderer,
        assets: &Assets,
        textures: &mut textures::Textures,
        smf: smf::Scene,
    ) -> Result<models::Model, AssetError> {
        fn do_node(
            renderer: &Renderer,
            assets: &Assets,
            textures: &mut textures::Textures,
            nodes: &[smf::Node],
            parent_node_name: &str,
        ) -> Vec<models::ModelNode> {
            fn do_mesh(
                renderer: &Renderer,
                assets: &Assets,
                textures: &mut textures::Textures,
                mesh: &smf::Mesh,
            ) -> Result<models::ModelMesh, ()> {
                // Load the texture
                let texture_path = format!(r"textures\shared\{}", mesh.texture_name);
                info!("Loading texture: {}", texture_path);
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

        let mut more_vertices = self.terrain.render_normals();
        self.gizmos_vertices.append(&mut more_vertices);
        let mut more_vertices = self.terrain.render_normals_lookup();
        self.gizmos_vertices.append(&mut more_vertices);

        self.objects
            .render(renderer, encoder, output, &self.gpu_camera.bind_group);

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
