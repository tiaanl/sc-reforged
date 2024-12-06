use crate::engine::prelude::*;

use glam::{Mat4, Quat, Vec2, Vec3};
use wgpu::util::DeviceExt;

pub fn register_camera_shader(shaders: &mut Shaders) {
    shaders.add_module(include_str!("camera.wgsl"), "camera.wgsl");
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Debug)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}

impl Ray {
    pub fn intersect_plane(&self, plane: &Plane) -> Option<Vec3> {
        let denom = self.direction.dot(plane.normal);

        // Check if the ray is parallel to the plane
        if denom.abs() < 1e-6 {
            return None;
        }

        let t = (plane.point - self.origin).dot(plane.normal) / denom;

        // Check if the intersection is behind the ray's origin
        if t < 0.0 {
            return None;
        }

        // Compute the intersection point
        Some(self.origin + t * self.direction)
    }
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct Matrices {
    pub projection: Mat4,
    pub view: Mat4,
}

#[derive(Debug, Default)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub const FORWARD: Vec3 = Vec3::Y;
    pub const RIGHT: Vec3 = Vec3::X;
    pub const UP: Vec3 = Vec3::Z;

    pub fn new(
        position: Vec3,
        rotation: Quat,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Camera {
            position,
            rotation,
            fov,
            aspect_ratio,
            near,
            far,
        }
    }

    pub fn calculate_matrices(&self) -> Matrices {
        let projection = Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far);

        let target = self.position + self.rotation * Self::FORWARD;
        let view = Mat4::look_at_lh(self.position, target, self.rotation * Self::UP);

        Matrices { projection, view }
    }

    /// Generates a ray in world space based on the mouse position.
    pub fn generate_ray(&self, mouse_ndc: Vec2) -> Ray {
        let ndc = Vec3::new(mouse_ndc.x, mouse_ndc.y, 1.0);

        // TODO: Can we cache the matrices somewhere?
        let matrices = self.calculate_matrices();
        let projection_matrix = matrices.projection;
        let view_matrix = matrices.view;

        let inverse_view_proj = (projection_matrix * view_matrix).inverse();

        let world_coords = inverse_view_proj.project_point3(ndc);

        let ray_origin = self.position;
        let ray_direction = (world_coords - ray_origin).normalize();

        Ray {
            origin: ray_origin,
            direction: ray_direction,
        }
    }
}

pub struct GpuCamera {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
    pub fn new(renderer: &Renderer) -> Self {
        let matrices = Matrices::default();
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera_buffer"),
                contents: bytemuck::cast_slice(&[matrices]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_layout =
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

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("camera_bind_group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
                }],
            });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn upload_matrices(&self, queue: &wgpu::Queue, matrices: &Matrices) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[*matrices]));
    }
}

pub struct FreeCameraController {
    pub position: Vec3,
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees

    pub movement_speed: f32,
    pub mouse_sensitivity: f32,

    // Track whether the values have changed since last update.
    dirty: Dirty,
}

impl FreeCameraController {
    pub fn new(movement_speed: f32, mouse_sensitivity: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            movement_speed,
            mouse_sensitivity,
            dirty: Dirty::smudged(),
        }
    }

    #[inline]
    fn rotation(&self) -> Quat {
        Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }

    pub fn move_forward(&mut self, distance: f32) {
        self.dirty.smudge();
        self.position += self.rotation() * Camera::FORWARD * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        self.dirty.smudge();
        self.position += self.rotation() * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.dirty.smudge();
        self.position += self.rotation() * Camera::UP * distance;
    }

    pub fn update(&mut self, input: &InputState, delta_time: f32) {
        let delta = delta_time * self.movement_speed;
        if input.key_pressed(KeyCode::KeyW) {
            self.move_forward(delta);
        }
        if input.key_pressed(KeyCode::KeyS) {
            self.move_forward(-delta);
        }
        if input.key_pressed(KeyCode::KeyA) {
            self.move_right(delta);
        }
        if input.key_pressed(KeyCode::KeyD) {
            self.move_right(-delta);
        }
        if input.key_pressed(KeyCode::KeyE) {
            self.move_up(delta);
        }
        if input.key_pressed(KeyCode::KeyQ) {
            self.move_up(-delta);
        }

        if input.mouse_pressed(MouseButton::Left) {
            if let Some(delta) = input.mouse_delta() {
                let delta = delta * self.mouse_sensitivity;
                if delta.x != 0.0 || delta.y != 0.0 {
                    self.yaw += delta.x;
                    self.pitch -= delta.y;
                    self.dirty.smudge();
                }
            }
        }
    }

    pub fn update_camera_if_dirty(&self, camera: &mut Camera) -> bool {
        self.dirty.if_dirty(|| {
            camera.position = self.position;
            camera.rotation = self.rotation();
        })
    }
}

#[derive(Default)]
pub struct ArcBacllCameraController {
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees
    pub distance: f32,

    pub mouse_sensitivity: f32,

    dirty: Dirty,
}

impl ArcBacllCameraController {
    pub fn new(mouse_sensitivity: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            distance: 0.0,
            mouse_sensitivity,
            dirty: Dirty::smudged(),
        }
    }

    pub fn position_and_rotation(&self) -> (Vec3, Quat) {
        let rotation = Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians());
        let position = rotation * Vec3::new(0.0, -self.distance, 0.0);

        (position, rotation)
    }

    pub fn on_input(&mut self, input: &InputState, _delta_time: f32) {
        if input.mouse_pressed(MouseButton::Left) {
            if let Some(delta) = input.mouse_delta() {
                let delta = delta * self.mouse_sensitivity;
                self.yaw += delta.x;
                self.pitch -= delta.y;
                self.pitch = self.pitch.clamp(-89.0_f32, 89.0_f32);
            }
        }
        let distance = self.distance / 10.0;
        self.distance -= input.wheel_delta() * distance;
        // self.distance = self.distance.clamp(camera.near, camera.far);
    }

    pub fn update_camera_if_changed(&self, camera: &mut Camera) -> bool {
        self.dirty.if_dirty(|| {
            let (position, rotation) = self.position_and_rotation();
            camera.position = position;
            camera.rotation = rotation;
        })
    }
}
