/// Orbit camera for the 3D viewport.
use glam::{Mat4, Vec3};

pub struct OrbitCamera {
    pub yaw: f32,      // horizontal rotation (radians)
    pub pitch: f32,    // vertical rotation (radians)
    pub distance: f32, // distance from target
    pub target: Vec3,  // look-at point
}

impl OrbitCamera {
    pub fn new() -> Self {
        Self {
            yaw: 0.5,
            pitch: 0.3,
            distance: 50.0,
            target: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.target
            + Vec3::new(
                self.distance * self.pitch.cos() * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                self.distance * self.pitch.cos() * self.yaw.cos(),
            )
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye_position(), self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(45.0_f32.to_radians(), aspect, 0.1, 500.0)
    }

    pub fn mvp(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Handle orbit drag (left-click drag)
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.01;
        self.pitch += dy * 0.01;
        // Clamp pitch to avoid gimbal lock
        self.pitch = self.pitch.clamp(
            -std::f32::consts::FRAC_PI_2 + 0.05,
            std::f32::consts::FRAC_PI_2 - 0.05,
        );
    }

    /// Handle zoom (scroll wheel)
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * 0.1;
        self.distance = self.distance.clamp(10.0, 200.0);
    }

    /// Handle pan (middle-click drag)
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let up = Vec3::Y;
        let scale = self.distance * 0.005;
        self.target += right * (-dx * scale) + up * (dy * scale);
    }
}
