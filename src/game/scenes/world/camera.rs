use glam::{Mat4, Quat, Vec3};

pub struct Camera {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    aspect: f32,
    near: f32,
    far: f32,

    bounds_min: Vec3,
    bounds_max: Vec3,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct Matrices {
    pub projection: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}

impl Camera {
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            aspect: 1.0,
            near: 0.1,
            far: 100000.0,

            bounds_min: Vec3::NEG_INFINITY,
            bounds_max: Vec3::INFINITY,
        }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("camera_info").show(ui, |ui| {
            ui.label("position");
            ui.vertical(|ui| {
                let (min, max) = (self.bounds_min, self.bounds_max);

                if min.x.is_infinite() || max.x.is_infinite() {
                    ui.label(format!("{}", self.position.x));
                } else {
                    ui.add(egui::Slider::new(&mut self.position.x, min.x..=max.x));
                }
                if min.y.is_infinite() || max.y.is_infinite() {
                    ui.label(format!("{}", self.position.y));
                } else {
                    ui.add(egui::Slider::new(&mut self.position.y, min.y..=max.y));
                }
                if min.z.is_infinite() || max.z.is_infinite() {
                    ui.label(format!("{}", self.position.z));
                } else {
                    ui.add(egui::Slider::new(&mut self.position.z, min.z..=max.z));
                }
            });
            ui.end_row();

            // ui.label("rotation");
            // ui.label(format!("{:?}", self.rotation));
            // ui.end_row();
        });
    }

    /// Adjust the aspect ratio of the camera view plane.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }

    pub fn set_bounds(&mut self, min: Vec3, max: Vec3) {
        self.bounds_min = min;
        self.bounds_max = max;
    }

    /// Create and returns the projection and view matrices based on the position and rotation of the camera.
    pub fn create_matrices(&self) -> Matrices {
        let projection =
            glam::Mat4::perspective_lh(45.0_f32.to_radians(), self.aspect, self.near, self.far);

        let rotation = Mat4::from_quat(self.rotation);
        // Translation is inverted, because we're moving the world, not the camera.
        let translation = Mat4::from_translation(self.position);
        let view = rotation * translation;

        Matrices {
            projection: projection.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
        }
    }

    /// Calculate and return the camera's forward vector.
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn _look_at(&mut self, target: Vec3) {
        let direction = (target - self.position).normalize();
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let dot = forward.dot(direction);

        // If looking directly opposite, rotate by PI around the Y axis
        if dot < -0.9999 {
            self.rotation = Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI);
        } else if dot > 0.9999 {
            // If already looking at the target, keep the current rotation
            self.rotation = Quat::IDENTITY;
        } else {
            let axis = forward.cross(direction).normalize();
            let angle = dot.acos();
            self.rotation = Quat::from_axis_angle(axis, angle);
        }
    }
}
