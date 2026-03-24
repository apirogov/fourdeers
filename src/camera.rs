//! Camera and view-related functionality

use nalgebra::{UnitQuaternion, Vector3};

use crate::rotation4d::{Rotation4D, RotationPlane};

/// First-person camera state with 4D orientation
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub orientation: UnitQuaternion<f32>,

    pub w: f32,

    pub rotation_4d: Rotation4D,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: -5.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        }
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = -5.0;
        self.orientation = UnitQuaternion::identity();
        self.w = 0.0;
        self.rotation_4d = Rotation4D::identity();
    }

    /// Forward vector in world space (direction camera is looking)
    /// +Z is forward, +X is right, +Y is up
    pub fn forward_vector(&self) -> (f32, f32, f32) {
        let forward = self
            .orientation
            .transform_vector(&Vector3::new(0.0, 0.0, 1.0));
        (forward.x, forward.y, forward.z)
    }

    /// Right vector in world space
    pub fn right_vector(&self) -> (f32, f32, f32) {
        let right = self
            .orientation
            .transform_vector(&Vector3::new(1.0, 0.0, 0.0));
        (right.x, right.y, right.z)
    }

    /// Up vector in world space
    pub fn up_vector(&self) -> (f32, f32, f32) {
        let up = self
            .orientation
            .transform_vector(&Vector3::new(0.0, 1.0, 0.0));
        (up.x, up.y, up.z)
    }

    /// Move camera along a direction vector
    pub fn move_along(&mut self, dir: (f32, f32, f32), speed: f32) {
        self.x += dir.0 * speed;
        self.y += dir.1 * speed;
        self.z += dir.2 * speed;
    }

    /// Rotate camera by delta mouse movement
    /// delta_x: horizontal movement (positive = drag right)
    /// delta_y: vertical movement (positive = drag down)
    ///
    /// Standard FPS controls:
    /// - Drag right -> look right (world appears to move left)
    /// - Drag down -> look down (world appears to move up)
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        // Negative yaw for drag right to look right
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -delta_x * 0.005);
        // Positive pitch for drag down to look down
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta_y * 0.005);

        // Apply rotations: yaw (around world Y) then pitch (around local X)
        self.orientation = yaw_rot * self.orientation * pitch_rot;
    }

    /// Get yaw angle (rotation around Y axis) in radians
    pub fn yaw(&self) -> f32 {
        let forward = self.forward_vector();
        forward.0.atan2(forward.2)
    }

    /// Get pitch angle (rotation around X axis) in radians
    pub fn pitch(&self) -> f32 {
        let forward = self.forward_vector();
        let horizontal_len = (forward.0 * forward.0 + forward.2 * forward.2).sqrt();
        forward.1.atan2(horizontal_len)
    }

    /// Set orientation from yaw and pitch angles
    pub fn set_yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.orientation = yaw_rot * pitch_rot;
    }

    pub fn rotate_4d(&mut self, delta_x: f32, delta_y: f32) {
        let tilt_zw = Rotation4D::from_plane_angle(RotationPlane::ZW, -delta_x * 0.005);
        let tilt_yw = Rotation4D::from_plane_angle(RotationPlane::YW, delta_y * 0.005);
        let delta_rot = tilt_zw.then(&tilt_yw);
        self.rotation_4d = delta_rot.then(&self.rotation_4d);
    }

    pub fn tilt_slice_up(&mut self, amount: f32) {
        let tilt = Rotation4D::from_plane_angle(RotationPlane::YW, amount * 0.02);
        self.rotation_4d = tilt.then(&self.rotation_4d);
    }

    pub fn tilt_slice_down(&mut self, amount: f32) {
        let tilt = Rotation4D::from_plane_angle(RotationPlane::YW, -amount * 0.02);
        self.rotation_4d = tilt.then(&self.rotation_4d);
    }

    pub fn tilt_slice_left(&mut self, amount: f32) {
        let tilt = Rotation4D::from_plane_angle(RotationPlane::ZW, amount * 0.02);
        self.rotation_4d = tilt.then(&self.rotation_4d);
    }

    pub fn tilt_slice_right(&mut self, amount: f32) {
        let tilt = Rotation4D::from_plane_angle(RotationPlane::ZW, -amount * 0.02);
        self.rotation_4d = tilt.then(&self.rotation_4d);
    }

    pub fn get_4d_basis(&self) -> [[f32; 4]; 4] {
        self.rotation_4d.basis_vectors()
    }

    pub fn get_slice_w_axis(&self) -> [f32; 4] {
        self.rotation_4d.basis_w()
    }

    pub fn is_slice_tilted(&self) -> bool {
        !self.rotation_4d.is_pure_3d()
    }

    pub fn get_4d_direction_label(&self, direction: SliceDirection) -> String {
        let basis = self.rotation_4d.basis_vectors();
        let v = match direction {
            SliceDirection::Forward => basis[2],
            SliceDirection::Backward => [-basis[2][0], -basis[2][1], -basis[2][2], -basis[2][3]],
            SliceDirection::Left => [-basis[0][0], -basis[0][1], -basis[0][2], -basis[0][3]],
            SliceDirection::Right => basis[0],
            SliceDirection::Up => basis[1],
            SliceDirection::Down => [-basis[1][0], -basis[1][1], -basis[1][2], -basis[1][3]],
            SliceDirection::WPositive => basis[3],
            SliceDirection::WNegative => [-basis[3][0], -basis[3][1], -basis[3][2], -basis[3][3]],
        };
        format_4d_vector(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
    WPositive,
    WNegative,
}

fn format_4d_vector(v: [f32; 4]) -> String {
    fn fmt_comp(val: f32) -> String {
        if val.abs() < 0.01 {
            String::new()
        } else if (val - 1.0).abs() < 0.01 {
            "+".to_string()
        } else if (val + 1.0).abs() < 0.01 {
            "-".to_string()
        } else {
            format!("{:+.2}", val)
        }
    }
    let parts: Vec<String> = [
        (fmt_comp(v[0]), "X"),
        (fmt_comp(v[1]), "Y"),
        (fmt_comp(v[2]), "Z"),
        (fmt_comp(v[3]), "W"),
    ]
    .iter()
    .filter(|(comp, _)| !comp.is_empty())
    .map(|(comp, axis)| format!("{}{}", comp, axis))
    .collect();
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
        assert!((a - b).abs() < epsilon, "Expected {:.6}, got {:.6}", b, a);
    }

    #[test]
    fn test_camera_new() {
        let camera = Camera::new();
        assert_eq!(camera.x, 0.0);
        assert_eq!(camera.y, 0.0);
        assert_eq!(camera.z, -5.0);
        assert_eq!(camera.orientation, UnitQuaternion::identity());
        assert_eq!(camera.w, 0.0);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_camera_reset() {
        let mut camera = Camera::new();
        camera.x = 10.0;
        camera.y = 20.0;
        camera.z = 30.0;
        camera.orientation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 1.0);
        camera.w = 3.0;
        camera.rotation_4d = Rotation4D::from_plane_angle(RotationPlane::XW, 0.5);

        camera.reset();

        assert_eq!(camera.x, 0.0);
        assert_eq!(camera.y, 0.0);
        assert_eq!(camera.z, -5.0);
        assert_eq!(camera.orientation, UnitQuaternion::identity());
        assert_eq!(camera.w, 0.0);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_forward_vector_identity() {
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let forward = camera.forward_vector();
        assert_approx_eq(forward.0, 0.0, 1e-6);
        assert_approx_eq(forward.1, 0.0, 1e-6);
        assert_approx_eq(forward.2, 1.0, 1e-6);
    }

    #[test]
    fn test_right_vector_identity() {
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let right = camera.right_vector();
        assert_approx_eq(right.0, 1.0, 1e-6);
        assert_approx_eq(right.1, 0.0, 1e-6);
        assert_approx_eq(right.2, 0.0, 1e-6);
    }

    #[test]
    fn test_up_vector_identity() {
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let up = camera.up_vector();
        assert_approx_eq(up.0, 0.0, 1e-6);
        assert_approx_eq(up.1, 1.0, 1e-6);
        assert_approx_eq(up.2, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_yaw() {
        // Rotate 90° around Y (yaw right)
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 2.0),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let forward = camera.forward_vector();
        assert_approx_eq(forward.0, 1.0, 1e-6);
        assert_approx_eq(forward.1, 0.0, 1e-6);
        assert_approx_eq(forward.2, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_pitch() {
        // Rotate 45° around X (pitch up)
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 4.0),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let forward = camera.forward_vector();
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;
        // Positive rotation around X rotates Y toward Z, so Z rotates toward -Y
        assert_approx_eq(forward.0, 0.0, 1e-6);
        assert_approx_eq(forward.1, -sqrt2_2, 1e-6);
        assert_approx_eq(forward.2, sqrt2_2, 1e-6);
    }

    #[test]
    fn test_rotate() {
        let mut camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        camera.rotate(1.0, 0.0);
        let forward = camera.forward_vector();

        // After rotation around Y axis, forward should be rotated
        assert!(forward.0.abs() > 1e-6 || forward.2.abs() < 0.99);
    }

    #[test]
    fn test_orthonormal_basis() {
        // Test that forward, right, up form an orthonormal basis
        let orientations = vec![
            UnitQuaternion::identity(),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 4.0),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 2.0),
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 6.0),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 4.0)
                * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 6.0),
        ];

        for orientation in orientations {
            let camera = Camera {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                orientation,
                w: 0.0,
                rotation_4d: Rotation4D::identity(),
            };

            let forward = camera.forward_vector();
            let right = camera.right_vector();
            let up = camera.up_vector();

            // Test orthogonality
            let dot_fr = forward.0 * right.0 + forward.1 * right.1 + forward.2 * right.2;
            let dot_fu = forward.0 * up.0 + forward.1 * up.1 + forward.2 * up.2;
            let dot_ru = right.0 * up.0 + right.1 * up.1 + right.2 * up.2;

            assert_approx_eq(dot_fr, 0.0, 1e-6);
            assert_approx_eq(dot_fu, 0.0, 1e-6);
            assert_approx_eq(dot_ru, 0.0, 1e-6);

            // Test normalization
            let len_f =
                (forward.0 * forward.0 + forward.1 * forward.1 + forward.2 * forward.2).sqrt();
            let len_r = (right.0 * right.0 + right.1 * right.1 + right.2 * right.2).sqrt();
            let len_u = (up.0 * up.0 + up.1 * up.1 + up.2 * up.2).sqrt();

            assert_approx_eq(len_f, 1.0, 1e-6);
            assert_approx_eq(len_r, 1.0, 1e-6);
            assert_approx_eq(len_u, 1.0, 1e-6);

            // Test right-hand coordinate system: forward × right = up
            let cross_x = forward.1 * right.2 - forward.2 * right.1;
            let cross_y = forward.2 * right.0 - forward.0 * right.2;
            let cross_z = forward.0 * right.1 - forward.1 * right.0;

            assert_approx_eq(cross_x, up.0, 1e-6);
            assert_approx_eq(cross_y, up.1, 1e-6);
            assert_approx_eq(cross_z, up.2, 1e-6);
        }
    }

    #[test]
    fn test_coordinate_consistency_forward() {
        // Test that moving forward increases Z when facing forward
        let mut camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let initial_z = camera.z;
        let forward = camera.forward_vector();

        assert_approx_eq(forward.2, 1.0, 1e-6);

        camera.move_along(forward, 1.0);
        assert_approx_eq(camera.z - initial_z, 1.0, 1e-6);
    }

    #[test]
    fn test_coordinate_consistency_right() {
        // Test that moving right increases X when facing forward
        let mut camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let initial_x = camera.x;
        let right = camera.right_vector();

        assert_approx_eq(right.0, 1.0, 1e-6);

        camera.move_along(right, 1.0);
        assert_approx_eq(camera.x - initial_x, 1.0, 1e-6);
    }

    #[test]
    fn test_backward_movement_inverts_forward() {
        // Test that moving backward is opposite to forward
        let mut camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: UnitQuaternion::identity(),
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
        };

        let initial_z = camera.z;
        let forward = camera.forward_vector();

        camera.move_along(forward, 1.0);
        let after_forward_z = camera.z;

        camera.z = initial_z;
        camera.move_along((-forward.0, -forward.1, -forward.2), 1.0);
        let after_backward_z = camera.z;

        assert_approx_eq(after_forward_z - initial_z, 1.0, 1e-6);
        assert_approx_eq(after_backward_z - initial_z, -1.0, 1e-6);
    }

    #[test]
    fn test_move_along() {
        let mut camera = Camera::new();
        camera.move_along((1.0, 2.0, 3.0), 0.5);

        assert_approx_eq(camera.x, 0.5, 1e-6);
        assert_approx_eq(camera.y, 1.0, 1e-6);
        assert_approx_eq(camera.z, -5.0 + 1.5, 1e-6);
    }
}
