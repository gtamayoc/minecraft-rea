//! First-person perspective 3D camera with View and Projection matrix generation.

use glam::{Mat4, Vec3};

/// First person 3D camera.
#[derive(Debug, Clone)]
pub struct Camera {
    /// World position of the camera (player eyes).
    pub position: Vec3,
    /// Yaw angle in degrees (horizontal rotation).
    pub yaw: f32,
    /// Pitch angle in degrees (vertical rotation: -89.0 to 89.0).
    pub pitch: f32,
    /// Field of view in radians.
    pub fov_y: f32,
    /// Aspect ratio (width / height).
    pub aspect: f32,
    /// Near clipping plane distance.
    pub near: f32,
    /// Far clipping plane distance.
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 65.0, 0.0),
            yaw: -90.0, // Default looking down negative Z
            pitch: 0.0,
            fov_y: 70.0f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    /// Creates a new Camera with custom position and aspect ratio.
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            aspect,
            ..Default::default()
        }
    }

    /// Calculates the forward direction vector from yaw and pitch.
    pub fn forward(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        let x = yaw_rad.cos() * pitch_rad.cos();
        let y = pitch_rad.sin();
        let z = yaw_rad.sin() * pitch_rad.cos();

        Vec3::new(x, y, z).normalize()
    }

    /// Calculates the right direction vector.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Calculates the up direction vector.
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    /// Builds the view transformation matrix.
    pub fn view_matrix(&self) -> Mat4 {
        let target = self.position + self.forward();
        Mat4::look_at_rh(self.position, target, Vec3::Y)
    }

    /// Builds the perspective projection matrix for WGPU (reverse-Z compatible or standard).
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    /// Computes the combined View-Projection matrix.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Rotates the camera view given mouse delta offsets.
    pub fn rotate(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        self.yaw += dx * sensitivity;
        self.pitch -= dy * sensitivity;
        self.pitch = self.pitch.clamp(-89.9, 89.9);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_default_direction() {
        let cam = Camera::default();
        let forward = cam.forward();

        // Yaw = -90, Pitch = 0 points roughly towards (0, 0, -1)
        assert!((forward.x).abs() < 1e-4);
        assert!((forward.y).abs() < 1e-4);
        assert!((forward.z - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn camera_view_projection_matrix_valid() {
        let cam = Camera::new(Vec3::new(0.0, 64.0, 0.0), 16.0 / 9.0);
        let vp = cam.view_projection_matrix();
        assert!(!vp.is_nan());
    }
}
