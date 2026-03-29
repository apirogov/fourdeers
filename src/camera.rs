//! Camera and view-related functionality

use nalgebra::{UnitQuaternion, Vector3};

use crate::rotation4d::{Rotation4D, RotationPlane};

const ROTATION_SENSITIVITY: f32 = 0.005;

/// First-person camera state with 4D orientation
#[derive(Clone)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub w: f32,

    pub rotation_4d: Rotation4D,

    yaw_l: f32,
    pitch_l: f32,
    yaw_r: f32,
    pitch_r: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: -5.0,
            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            yaw_l: 0.0,
            pitch_l: 0.0,
            yaw_r: 0.0,
            pitch_r: 0.0,
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
        self.w = 0.0;
        self.rotation_4d = Rotation4D::identity();
        self.yaw_l = 0.0;
        self.pitch_l = 0.0;
        self.yaw_r = 0.0;
        self.pitch_r = 0.0;
    }

    /// Forward vector in world space (direction camera is looking)
    /// +Z is forward, +X is right, +Y is up
    pub fn forward_vector(&self) -> (f32, f32, f32) {
        let forward = self
            .rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(0.0, 0.0, 1.0));
        (forward.x, forward.y, forward.z)
    }

    /// Right vector in world space
    pub fn right_vector(&self) -> (f32, f32, f32) {
        let right = self
            .rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(1.0, 0.0, 0.0));
        (right.x, right.y, right.z)
    }

    /// Up vector in world space
    pub fn up_vector(&self) -> (f32, f32, f32) {
        let up = self
            .rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(0.0, 1.0, 0.0));
        (up.x, up.y, up.z)
    }

    /// Move camera along a direction vector
    pub fn move_along(&mut self, dir: (f32, f32, f32), speed: f32) {
        self.x += dir.0 * speed;
        self.y += dir.1 * speed;
        self.z += dir.2 * speed;
    }

    /// Rotate camera by delta mouse movement (3D mode - affects q_left)
    /// delta_x: horizontal movement (positive = drag right)
    /// delta_y: vertical movement (positive = drag down)
    ///
    /// Standard FPS controls:
    /// - Drag right -> look right (world appears to move left)
    /// - Drag down -> look down (world appears to move up)
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        // Negative yaw for drag right to look right
        let yaw_rot =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), delta_x * ROTATION_SENSITIVITY);
        // Positive pitch for drag down to look down
        let pitch_rot =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta_y * ROTATION_SENSITIVITY);

        // Apply rotations: yaw (around world Y) then pitch (around local X)
        // Modify q_left (the 3D-like rotation)
        let new_q_left = yaw_rot * *self.rotation_4d.q_left() * pitch_rot;
        self.rotation_4d = Rotation4D::new(new_q_left, *self.rotation_4d.q_right());

        // Update cached values
        self.yaw_l += delta_x * ROTATION_SENSITIVITY;
        self.pitch_l += delta_y * ROTATION_SENSITIVITY;
    }

    /// Rotate 4D camera (4D mode - affects q_right)
    pub fn rotate_4d(&mut self, delta_x: f32, delta_y: f32) {
        // Modify q_right (the 4D-specific rotation)
        // XW plane for horizontal (like XZ in 3D), YW plane for vertical (like YZ in 3D)
        let tilt_xw =
            Rotation4D::from_plane_angle(RotationPlane::XW, -delta_x * ROTATION_SENSITIVITY);
        let tilt_yw =
            Rotation4D::from_plane_angle(RotationPlane::YW, delta_y * ROTATION_SENSITIVITY);
        // from_plane_angle stores 4D rotations in q_left, so use q_left
        let new_q_right = *tilt_xw.q_left() * *tilt_yw.q_left() * *self.rotation_4d.q_right();
        self.rotation_4d = Rotation4D::new(*self.rotation_4d.q_left(), new_q_right);

        // Update cached values
        self.yaw_r += -delta_x * ROTATION_SENSITIVITY;
        self.pitch_r += delta_y * ROTATION_SENSITIVITY;
    }

    /// Get yaw angle (rotation around Y axis) in radians - for q_left
    pub fn yaw_l(&self) -> f32 {
        self.yaw_l
    }

    /// Get pitch angle (rotation around X axis) in radians - for q_left
    pub fn pitch_l(&self) -> f32 {
        self.pitch_l
    }

    /// Set q_left (3D orientation) from yaw and pitch angles
    pub fn set_yaw_pitch_l(&mut self, yaw: f32, pitch: f32) {
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.rotation_4d = Rotation4D::new(yaw_rot * pitch_rot, *self.rotation_4d.q_right());
        self.yaw_l = yaw;
        self.pitch_l = pitch;
    }

    /// Set yaw only for q_left, preserving current pitch
    pub fn set_yaw_l(&mut self, yaw: f32) {
        let pitch = self.pitch_l;
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.rotation_4d = Rotation4D::new(yaw_rot * pitch_rot, *self.rotation_4d.q_right());
        self.yaw_l = yaw;
    }

    /// Set pitch only for q_left, preserving current yaw
    pub fn set_pitch_l(&mut self, pitch: f32) {
        let yaw = self.yaw_l;
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.rotation_4d = Rotation4D::new(yaw_rot * pitch_rot, *self.rotation_4d.q_right());
        self.pitch_l = pitch;
    }

    /// Get yaw angle for q_right (4D rotation in XW plane)
    pub fn yaw_r(&self) -> f32 {
        self.yaw_r
    }

    /// Get pitch angle for q_right (4D rotation in YW plane)
    pub fn pitch_r(&self) -> f32 {
        self.pitch_r
    }

    /// Set yaw only for q_right, preserving current pitch
    pub fn set_yaw_r(&mut self, yaw: f32) {
        let pitch = self.pitch_r;
        self.rotation_4d.set_q_right_from_yaw_pitch(yaw, pitch);
        self.yaw_r = yaw;
    }

    /// Set pitch only for q_right, preserving current yaw
    pub fn set_pitch_r(&mut self, pitch: f32) {
        let yaw = self.yaw_r;
        self.rotation_4d.set_q_right_from_yaw_pitch(yaw, pitch);
        self.pitch_r = pitch;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    IncreaseW,
    DecreaseW,
    MoveSliceForward,
    MoveSliceBackward,
    MoveKata,
    MoveAna,
}

impl Camera {
    pub fn project_3d_to_4d(&self, v3: (f32, f32, f32)) -> [f32; 4] {
        let basis_4d = self.rotation_4d.basis_vectors();
        [
            v3.0 * basis_4d[0][0] + v3.1 * basis_4d[1][0] + v3.2 * basis_4d[2][0],
            v3.0 * basis_4d[0][1] + v3.1 * basis_4d[1][1] + v3.2 * basis_4d[2][1],
            v3.0 * basis_4d[0][2] + v3.1 * basis_4d[1][2] + v3.2 * basis_4d[2][2],
            v3.0 * basis_4d[0][3] + v3.1 * basis_4d[1][3] + v3.2 * basis_4d[2][3],
        ]
    }

    pub fn apply_action(&mut self, action: CameraAction, speed: f32) {
        let forward = self.forward_vector();
        let right = self.right_vector();
        let up = self.up_vector();
        let basis_4d = self.rotation_4d.basis_vectors();

        match action {
            CameraAction::MoveForward => {
                let v4 = self.project_3d_to_4d(forward);
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::MoveBackward => {
                let v4 = self.project_3d_to_4d(forward);
                self.x -= v4[0] * speed;
                self.y -= v4[1] * speed;
                self.z -= v4[2] * speed;
                self.w -= v4[3] * speed;
            }
            CameraAction::MoveLeft => {
                let v4 = self.project_3d_to_4d((-right.0, -right.1, -right.2));
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::MoveRight => {
                let v4 = self.project_3d_to_4d(right);
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::MoveUp => {
                let v4 = self.project_3d_to_4d(up);
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::MoveDown => {
                let v4 = self.project_3d_to_4d((-up.0, -up.1, -up.2));
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::IncreaseW => self.w += speed,
            CameraAction::DecreaseW => self.w -= speed,
            CameraAction::MoveSliceForward => {
                let v4 = self.project_3d_to_4d(forward);
                self.x += v4[0] * speed;
                self.y += v4[1] * speed;
                self.z += v4[2] * speed;
                self.w += v4[3] * speed;
            }
            CameraAction::MoveSliceBackward => {
                let v4 = self.project_3d_to_4d(forward);
                self.x -= v4[0] * speed;
                self.y -= v4[1] * speed;
                self.z -= v4[2] * speed;
                self.w -= v4[3] * speed;
            }
            CameraAction::MoveKata => {
                let w_dir = basis_4d[3];
                self.x += w_dir[0] * speed;
                self.y += w_dir[1] * speed;
                self.z += w_dir[2] * speed;
                self.w += w_dir[3] * speed;
            }
            CameraAction::MoveAna => {
                let w_dir = basis_4d[3];
                self.x -= w_dir[0] * speed;
                self.y -= w_dir[1] * speed;
                self.z -= w_dir[2] * speed;
                self.w -= w_dir[3] * speed;
            }
        }
    }
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
        assert_eq!(camera.w, 0.0);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_camera_reset() {
        let mut camera = Camera::new();
        camera.x = 10.0;
        camera.y = 20.0;
        camera.z = 30.0;
        camera.w = 3.0;
        camera.rotation_4d = Rotation4D::from_plane_angle(RotationPlane::XW, 0.5);

        camera.reset();

        assert_eq!(camera.x, 0.0);
        assert_eq!(camera.y, 0.0);
        assert_eq!(camera.z, -5.0);
        assert_eq!(camera.w, 0.0);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_forward_vector_identity() {
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let up = camera.up_vector();
        assert_approx_eq(up.0, 0.0, 1e-6);
        assert_approx_eq(up.1, 1.0, 1e-6);
        assert_approx_eq(up.2, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_yaw() {
        // Rotate 90° around Y (yaw right) - use XZ plane
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: -5.0,
            w: 0.0,
            rotation_4d: Rotation4D::from_plane_angle(RotationPlane::XZ, PI / 2.0),
            ..Camera::new()
        };

        let forward = camera.forward_vector();
        assert_approx_eq(forward.0, 1.0, 1e-6);
        assert_approx_eq(forward.1, 0.0, 1e-6);
        assert_approx_eq(forward.2, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_pitch() {
        // Rotate 45° around X (pitch up) - use YZ plane
        let camera = Camera {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
            rotation_4d: Rotation4D::from_plane_angle(RotationPlane::YZ, PI / 4.0),
            ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        camera.rotate(1.0, 0.0);
        let forward = camera.forward_vector();

        // After rotation around Y axis, forward should be rotated
        assert!(forward.0.abs() > 1e-6 || forward.2.abs() < 0.99);
    }

    #[test]
    fn test_orthonormal_basis() {
        // Test that forward, right, up form an orthonormal basis
        let rotations = vec![
            Rotation4D::identity(),
            Rotation4D::from_plane_angle(RotationPlane::XY, PI / 4.0),
            Rotation4D::from_plane_angle(RotationPlane::XY, PI / 2.0),
            Rotation4D::from_plane_angle(RotationPlane::XZ, PI / 6.0),
        ];

        for rotation_4d in rotations {
            let camera = Camera {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
                rotation_4d,
                ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
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

            w: 0.0,
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
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

    #[test]
    fn test_rotate_affects_q_left_only() {
        let mut camera = Camera::new();

        let initial_q_right = *camera.rotation_4d.q_right();

        camera.rotate(1.0, 0.5);

        let new_q_right = *camera.rotation_4d.q_right();
        assert_eq!(
            initial_q_right, new_q_right,
            "rotate() should not affect q_right"
        );

        let forward = camera.forward_vector();
        assert!(
            forward.0.abs() > 1e-6 || forward.2.abs() < 0.99,
            "rotate() should change forward vector"
        );
    }

    #[test]
    fn test_rotate_4d_affects_q_right_only() {
        let mut camera = Camera::new();

        let initial_q_left = *camera.rotation_4d.q_left();

        camera.rotate_4d(1.0, 0.5);

        let new_q_left = *camera.rotation_4d.q_left();
        assert_eq!(
            initial_q_left, new_q_left,
            "rotate_4d() should not affect q_left"
        );

        let basis_w = camera.rotation_4d.basis_w();
        assert!(
            basis_w[3] != 1.0 || basis_w[0].abs() > 1e-6,
            "rotate_4d() should change W axis"
        );
    }

    #[test]
    fn test_rotate_4d_changes_basis_w() {
        let mut camera = Camera::new();

        let initial_basis_w = camera.rotation_4d.basis_w();
        assert_approx_eq(initial_basis_w[3], 1.0, 1e-6);

        camera.rotate_4d(1.0, 0.5);

        let new_basis_w = camera.rotation_4d.basis_w();
        assert!(
            new_basis_w[3].abs() < 0.99 || new_basis_w[0].abs() > 1e-6,
            "rotate_4d() should tilt W axis"
        );
    }

    #[test]
    fn test_rotate_and_rotate_4d_independent() {
        let mut camera = Camera::new();

        camera.rotate(1.0, 0.5);
        let q_left_after_rotate = *camera.rotation_4d.q_left();
        let q_right_after_rotate = *camera.rotation_4d.q_right();

        camera.rotate_4d(0.5, 1.0);

        let q_left_after_both = *camera.rotation_4d.q_left();
        let q_right_after_both = *camera.rotation_4d.q_right();

        assert_eq!(
            q_left_after_rotate, q_left_after_both,
            "rotate_4d() should not change q_left"
        );
        assert!(
            q_right_after_both != q_right_after_rotate,
            "rotate_4d() should change q_right"
        );
    }

    #[test]
    fn test_yaw_pitch_preservation() {
        let mut camera = Camera::new();

        camera.set_yaw_l(PI / 4.0);
        let yaw1 = camera.yaw_l();

        camera.set_pitch_l(PI / 6.0);
        let yaw2 = camera.yaw_l();

        assert_approx_eq(yaw1, yaw2, 1e-6);
    }
}
