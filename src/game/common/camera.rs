use crate::engine::{gizmos::GizmoVertex, prelude::*};

use glam::{FloatExt, Mat4, Quat, Vec2, Vec3, Vec4};

pub fn register_camera_shader(shaders: &mut Shaders) {
    shaders.add_module(include_str!("camera.wgsl"), "camera.wgsl");
}

#[derive(Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl From<Vec4> for Plane {
    fn from(value: Vec4) -> Self {
        Self {
            normal: Vec3 {
                x: value.x,
                y: value.y,
                z: value.z,
            },
            distance: value.w,
        }
    }
}

impl Plane {
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub fn distance_to(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn intersect_plane(&self, plane: &Plane) -> Option<Vec3> {
        let denom = self.direction.dot(plane.normal);

        // Check if the ray is parallel to the plane
        if denom.abs() < 1e-6 {
            return None;
        }

        let t = ((plane.normal * plane.distance) - self.origin).dot(plane.normal) / denom;

        // Check if the intersection is behind the ray's origin
        if t < 0.0 {
            return None;
        }

        // Compute the intersection point
        Some(self.origin + t * self.direction)
    }
}

#[derive(Debug, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn center(&self) -> Vec3 {
        self.min + (self.max - self.min)
    }
}

pub struct Frustum {
    pub planes: [Plane; 6],
}

impl From<Mat4> for Frustum {
    fn from(value: Mat4) -> Self {
        let left = value.row(3) + value.row(0);
        let right = value.row(3) - value.row(0);
        let bottom = value.row(3) + value.row(1);
        let top = value.row(3) - value.row(1);
        let near = value.row(3) + value.row(2);
        let far = value.row(3) + value.row(2);

        Self {
            planes: [
                Plane::from(left.normalize()),   // left
                Plane::from(right.normalize()),  // right
                Plane::from(bottom.normalize()), // bottom
                Plane::from(top.normalize()),    // top
                Plane::from(near.normalize()),   // near
                Plane::from(far.normalize()),    // far
            ],
        }
    }
}

impl Frustum {
    pub fn contains_bounding_box(&self, bounding_box: &BoundingBox) -> bool {
        // Get all 8 corners of the bounding box
        let corners = [
            bounding_box.min,
            Vec3::new(bounding_box.max.x, bounding_box.min.y, bounding_box.min.z),
            Vec3::new(bounding_box.min.x, bounding_box.max.y, bounding_box.min.z),
            Vec3::new(bounding_box.min.x, bounding_box.min.y, bounding_box.max.z),
            Vec3::new(bounding_box.max.x, bounding_box.max.y, bounding_box.min.z),
            Vec3::new(bounding_box.max.x, bounding_box.min.y, bounding_box.max.z),
            Vec3::new(bounding_box.min.x, bounding_box.max.y, bounding_box.max.z),
            bounding_box.max,
        ];

        // Check if the box is outside any plane
        for plane in &self.planes {
            let mut all_outside = true;

            for corner in &corners {
                if plane.distance_to(*corner) >= 0.0 {
                    all_outside = false;
                    break;
                }
            }

            // If all corners are outside this plane, the box is outside the frustum
            if all_outside {
                return false;
            }
        }

        true
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

    pub fn look_at(&mut self, camera_to: Vec3) {
        let forward = (camera_to - self.position).normalize();
        let world_up = Self::UP;
        let right = world_up.cross(forward).normalize();
        let up = forward.cross(right);
        let rotation_matrix = glam::Mat3::from_cols(right, up, forward);
        self.rotation = Quat::from_mat3(&rotation_matrix);
    }
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct CameraBuffer {
    pub proj: Mat4,
    pub view: Mat4,
    pub position: Vec4,
    pub frustum: [Vec4; 6],
}

pub struct GpuCamera {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
    pub fn new() -> Self {
        let buffer = renderer().device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_buffer"),
            size: std::mem::size_of::<CameraBuffer>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("camera_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::all(),
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = renderer()
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

    pub fn upload_matrices(&self, queue: &wgpu::Queue, matrices: &Matrices, position: Vec3) {
        let proj_view = matrices.projection * matrices.view;

        let data = CameraBuffer {
            proj: matrices.projection,
            view: matrices.view,
            position: position.extend(1.0),
            frustum: Frustum::from(proj_view)
                .planes
                .map(|p| p.normal.extend(p.distance)),
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }
}

pub trait Controller {
    fn update(&mut self, delta_time: f32, input: &InputState);
}

pub struct FreeCameraControls {
    mouse_button: MouseButton,
    forward: KeyCode,
    back: KeyCode,
    right: KeyCode,
    left: KeyCode,
    up: KeyCode,
    down: KeyCode,
}

impl Default for FreeCameraControls {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
            forward: KeyCode::KeyW,
            back: KeyCode::KeyS,
            right: KeyCode::KeyD,
            left: KeyCode::KeyA,
            up: KeyCode::KeyE,
            down: KeyCode::KeyQ,
        }
    }
}

pub struct FreeCameraController {
    pub position: Vec3,
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees

    pub movement_speed: f32,
    pub mouse_sensitivity: f32,

    /// Track whether the values have changed since last update.
    dirty: Dirty,

    /// The controls used to control the camera.
    controls: FreeCameraControls,
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
            controls: FreeCameraControls::default(),
        }
    }

    pub fn smudge(&self) {
        self.dirty.smudge();
    }

    /// Create a new [FreeCameraController] with the given control scheme.
    pub fn with_controls(mut self, controls: FreeCameraControls) -> Self {
        self.controls = controls;
        self
    }

    pub fn move_to(&mut self, position: Vec3) {
        self.position = position;
        self.dirty.smudge();
    }

    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.position).normalize();
        self.yaw = -direction.y.atan2(direction.x).to_degrees();
        self.pitch = direction.z.asin().to_degrees();
        self.dirty.smudge();
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
        // Because of our left-handed, z-up coord-system, right is negative.
        self.position -= self.rotation() * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.dirty.smudge();
        self.position += self.rotation() * Camera::UP * distance;
    }

    pub fn update_camera_if_dirty(&self, camera: &mut Camera) -> bool {
        self.dirty.if_dirty(|| {
            camera.position = self.position;
            camera.rotation = self.rotation();
        })
    }
}

impl Controller for FreeCameraController {
    fn update(&mut self, delta_time: f32, input: &InputState) {
        let delta =
            if input.key_pressed(KeyCode::ShiftLeft) || input.key_pressed(KeyCode::ShiftRight) {
                self.movement_speed * 2.0
            } else {
                self.movement_speed
            } * delta_time;

        if input.key_pressed(self.controls.forward) {
            self.move_forward(delta);
        }
        if input.key_pressed(self.controls.back) {
            self.move_forward(-delta);
        }
        if input.key_pressed(self.controls.right) {
            self.move_right(delta);
        }
        if input.key_pressed(self.controls.left) {
            self.move_right(-delta);
        }
        if input.key_pressed(self.controls.up) {
            self.move_up(delta);
        }
        if input.key_pressed(self.controls.down) {
            self.move_up(-delta);
        }

        if input.mouse_pressed(self.controls.mouse_button) {
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
}

#[derive(Default)]
pub struct ArcBallCameraController {
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees
    pub distance: f32,

    pub mouse_sensitivity: f32,

    dirty: Dirty,
}

impl ArcBallCameraController {
    pub fn new(mouse_sensitivity: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            distance: 100.0,
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
            self.dirty.smudge();
            if let Some(delta) = input.mouse_delta() {
                let delta = delta * self.mouse_sensitivity;
                self.yaw += delta.x;
                self.pitch -= delta.y;
                self.pitch = self.pitch.clamp(-89.0_f32, 89.0_f32);
            }
        }
        let delta = input.wheel_delta();
        if delta != 0.0 {
            self.dirty.smudge();
            let distance = self.distance / 10.0;
            self.distance -= delta * distance;
            // self.distance = self.distance.clamp(camera.near, camera.far);
        }
    }

    pub fn update_camera_if_changed(&self, camera: &mut Camera) -> bool {
        self.dirty.if_dirty(|| {
            let (position, rotation) = self.position_and_rotation();
            camera.position = position;
            camera.rotation = rotation;
        })
    }

    pub fn dirty(&mut self) {
        self.dirty.smudge();
    }
}

pub struct GameCameraControls {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub right: KeyCode,
    pub left: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub look_up: KeyCode,
    pub look_down: KeyCode,
    pub rotate_mouse_button: MouseButton,
}

impl Default for GameCameraControls {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            right: KeyCode::KeyD,
            left: KeyCode::KeyA,
            up: KeyCode::PageUp,
            down: KeyCode::PageDown,
            look_up: KeyCode::Home,
            look_down: KeyCode::End,
            rotate_mouse_button: MouseButton::Right,
        }
    }
}

#[derive(Default)]
struct GameCameraData {
    /// The position of the camera.
    position: Vec3,
    /// The direction the camera is looking horizontally.
    yaw: f32,
    /// The pitch of the camera in range level forward to straight down (with some buffer on each
    /// angle).
    pitch: f32,
}

impl GameCameraData {
    fn lerp(&mut self, target: &GameCameraData, progress: f32) {
        self.position = self.position.lerp(target.position, progress);
        self.yaw = self.yaw.lerp(target.yaw, progress);
        self.pitch = self.pitch.lerp(target.pitch, progress);
    }

    #[inline]
    fn rotation(&self) -> Quat {
        Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }
}

#[derive(Default)]
pub struct GameCameraController {
    /// Movement speed.
    speed: f32,
    /// Rotation sensitivity.
    sensitivity: f32,

    /// Controls used for the camera.
    controls: GameCameraControls,

    /// The desired data for the camera which will be interpolated each frame.
    desired: GameCameraData,

    /// The current data for the camera this frame.
    current: GameCameraData,

    /// A flag to keep track of changed values inside the camera.
    dirty: Dirty,
}

impl GameCameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            controls: GameCameraControls::default(),
            desired: GameCameraData::default(),
            current: GameCameraData::default(),
            dirty: Dirty::smudged(),
        }
    }

    pub fn with_controls(mut self, controls: GameCameraControls) -> Self {
        self.controls = controls;
        self
    }

    pub fn move_to_direct(&mut self, position: Vec3) {
        self.current.position = position;
        self.desired.position = self.current.position;
        self.dirty.smudge();
    }

    pub fn look_at_direct(&mut self, target: Vec3) {
        let direction = (target - self.current.position).normalize();
        self.current.yaw = -direction.y.atan2(direction.x).to_degrees();
        self.desired.yaw = self.current.yaw;
        self.current.pitch = direction.z.asin().to_degrees();
        self.desired.pitch = self.current.pitch;
        self.dirty.smudge();
    }

    pub fn move_forward(&mut self, distance: f32) {
        self.dirty.smudge();
        self.desired.position +=
            Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::FORWARD * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        self.dirty.smudge();
        // Because of our left-handed, z-up coord-system, right is negative.
        self.desired.position -=
            Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.dirty.smudge();
        self.desired.position += Camera::UP * distance;
    }

    pub fn update_camera_if_dirty(&self, camera: &mut Camera) -> bool {
        // self.dirty.if_dirty(|| {
        camera.position = self.current.position;
        camera.rotation = self.current.rotation();
        // })
        true
    }
}

impl Controller for GameCameraController {
    fn update(&mut self, delta_time: f32, input: &InputState) {
        let move_delta =
            if input.key_pressed(KeyCode::ShiftLeft) || input.key_pressed(KeyCode::ShiftRight) {
                self.speed * 2.0
            } else {
                self.speed
            } * delta_time;

        if input.key_pressed(self.controls.forward) {
            self.move_forward(move_delta);
        }
        if input.key_pressed(self.controls.backward) {
            self.move_forward(-move_delta);
        }
        if input.key_pressed(self.controls.right) {
            self.move_right(move_delta);
        }
        if input.key_pressed(self.controls.left) {
            self.move_right(-move_delta);
        }
        if input.key_pressed(self.controls.up) {
            self.move_up(move_delta);
        }
        if input.key_pressed(self.controls.down) {
            self.move_up(-move_delta);
        }
        if input.key_pressed(self.controls.look_up) {
            self.desired.pitch += delta_time;
            self.dirty.smudge();
        }
        if input.key_pressed(self.controls.look_down) {
            self.desired.pitch -= delta_time;
            self.dirty.smudge();
        }

        if input.mouse_pressed(MouseButton::Right) {
            if let Some(delta) = input.mouse_delta() {
                let delta = delta * self.sensitivity;
                if delta.x != 0.0 {
                    self.desired.yaw += delta.x;
                    self.dirty.smudge();
                }
                if delta.y != 0.0 {
                    self.desired.position.z -= delta.y * move_delta;
                    self.dirty.smudge();
                }
            }
        }

        if input.wheel_delta() != 0.0 {
            self.desired.position.z += input.wheel_delta() * self.speed;
            self.dirty.smudge();
        }

        // Interpolate the desired closer to the target.
        self.current.lerp(&self.desired, 0.1);
    }
}

pub fn render_camera_frustum(camera: &Camera, gizmo_vertices: &mut Vec<GizmoVertex>) {
    use glam::Vec4Swizzles;

    let ndc_corners = [
        Vec4::new(-1.0, -1.0, 0.0, 1.0), // Near-bottom-left
        Vec4::new(-1.0, 1.0, 0.0, 1.0),  // Near-top-left
        Vec4::new(1.0, 1.0, 0.0, 1.0),   // Near-top-right
        Vec4::new(1.0, -1.0, 0.0, 1.0),  // Near-bottom-right
        Vec4::new(-1.0, -1.0, 1.0, 1.0), // Far-bottom-left
        Vec4::new(-1.0, 1.0, 1.0, 1.0),  // Far-top-left
        Vec4::new(1.0, 1.0, 1.0, 1.0),   // Far-top-right
        Vec4::new(1.0, -1.0, 1.0, 1.0),  // Far-bottom-right
    ];

    // Calculate the view-projection matrix
    let matrices = camera.calculate_matrices();
    let view_projection = matrices.projection * matrices.view;

    // Invert the view-projection matrix
    let inv_view_projection = view_projection.inverse();

    let green = Vec4::new(0.0, 1.0, 0.0, 1.0);

    // Transform NDC corners to world space
    let vertices = ndc_corners.map(|corner| {
        let world_space = inv_view_projection * corner;
        GizmoVertex::new(world_space.xyz() / world_space.w, green)
    });

    // Should we even render this rectangle?  Its extremely small.
    gizmo_vertices.extend_from_slice(&[
        vertices[0],
        vertices[1],
        vertices[1],
        vertices[2],
        vertices[2],
        vertices[3],
        vertices[3],
        vertices[0],
    ]);

    gizmo_vertices.extend_from_slice(&[
        vertices[0],
        vertices[4],
        vertices[1],
        vertices[5],
        vertices[2],
        vertices[6],
        vertices[3],
        vertices[7],
    ]);

    gizmo_vertices.extend_from_slice(&[
        vertices[4],
        vertices[5],
        vertices[5],
        vertices[6],
        vertices[6],
        vertices[7],
        vertices[7],
        vertices[4],
    ]);
}
