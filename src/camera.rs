//! Camera model and movement semantics for split 3D/4D control.
//!
//! # Architecture contract
//!
//! The camera stores one `Rotation4D` but uses it in two semantic parts:
//!
//! - `q_left`: orientation *inside* the current 3D slice (look direction, left/right/up frame)
//! - `q_right`: orientation of the slice itself in 4D (slice tilt and slice normal)
//!
//! This split is intentional and must stay consistent with render code.
//!
//! # Why controls are implemented this way
//!
//! - 3D movement (`forward/backward/left/right/up/down`) should follow what the camera sees in the
//!   current slice. So we compute camera-frame directions from `q_left` and then project those into
//!   world 4D using the basis induced by `q_right` only.
//! - 4D movement (`kata/ana`) should move along the slice normal, so it uses `basis_w` from a
//!   rotation built as `(identity, q_right)`.
//!
//! If `kata/ana` are derived from the full `(q_left, q_right)` basis, horizontal 3D look changes
//! can incorrectly introduce vertical drift.
//!
//! # Refactor guardrails
//!
//! - Keep `rotate` affecting `q_left` only.
//! - Keep `rotate_4d` affecting `q_right` only.
//! - Keep `apply_action` frame split exactly as described above.
//! - If changed, run and verify camera tests around:
//!   - `test_apply_action_moves_follow_3d_camera_frame_with_tilted_slice`
//!   - `test_kata_ana_independent_of_q_left_yaw_pitch`
//!   - `test_kata_ana_do_not_change_xyz_in_pure_3d_slice`

use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::rotation4d::{Rotation4D, RotationPlane};

const ROTATION_SENSITIVITY: f32 = 0.005;
const DEFAULT_CAMERA_POSITION: Vector4<f32> = Vector4::new(0.0, 0.0, -5.0, 0.0);

/// First-person camera state with 4D orientation
#[derive(Clone)]
pub struct Camera {
    pub position: Vector4<f32>,

    pub rotation_4d: Rotation4D,

    /// Cached yaw angle (rotation around Y axis) for `q_left` - in radians.
    /// Cached to avoid quaternion-to-Euler conversion instability that causes UI slider glitching.
    yaw_l: f32,
    pitch_l: f32,
    yaw_r: f32,
    pitch_r: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: DEFAULT_CAMERA_POSITION,
            rotation_4d: Rotation4D::identity(),
            yaw_l: 0.0,
            pitch_l: 0.0,
            yaw_r: 0.0,
            pitch_r: 0.0,
        }
    }
}

impl Camera {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.position = DEFAULT_CAMERA_POSITION;
        self.rotation_4d = Rotation4D::identity();
        self.yaw_l = 0.0;
        self.pitch_l = 0.0;
        self.yaw_r = 0.0;
        self.pitch_r = 0.0;
    }

    /// Forward vector in world space (direction camera is looking)
    /// +Z is forward, +X is right, +Y is up
    #[must_use]
    pub fn forward_vector(&self) -> Vector3<f32> {
        self.rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(0.0, 0.0, 1.0))
    }

    /// Right vector in world space
    #[must_use]
    pub fn right_vector(&self) -> Vector3<f32> {
        self.rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(1.0, 0.0, 0.0))
    }

    /// Up vector in world space
    #[must_use]
    pub fn up_vector(&self) -> Vector3<f32> {
        self.rotation_4d
            .q_left()
            .transform_vector(&Vector3::new(0.0, 1.0, 0.0))
    }

    /// Move camera along a direction vector
    pub fn move_along(&mut self, dir: Vector3<f32>, speed: f32) {
        let movement_4d = self.project_3d_to_4d(dir);
        self.position += Vector4::new(
            movement_4d[0],
            movement_4d[1],
            movement_4d[2],
            movement_4d[3],
        ) * speed;
    }

    /// Rotate camera by delta mouse movement (3D mode - affects `q_left`)
    /// `delta_x`: horizontal movement (positive = drag right)
    /// `delta_y`: vertical movement (positive = drag down)
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

    /// Rotate 4D camera slice orientation (affects `q_right` only).
    ///
    /// This tilts the 3D slice in 4D. It should not change in-slice look frame (`q_left`).
    #[allow(clippy::similar_names)]
    pub fn rotate_4d(&mut self, delta_x: f32, delta_y: f32) {
        // XW plane for horizontal (like XZ in 3D), YW plane for vertical (like YZ in 3D)
        // Match 3D pattern: yaw * old * pitch
        let tilt_xw =
            Rotation4D::from_plane_angle(RotationPlane::XW, -delta_x * ROTATION_SENSITIVITY);
        let tilt_yw =
            Rotation4D::from_plane_angle(RotationPlane::YW, delta_y * ROTATION_SENSITIVITY);

        // Apply in same order as 3D: new_xw * old * new_yw
        let new_q_right = *tilt_xw.q_left() * *self.rotation_4d.q_right() * *tilt_yw.q_left();
        self.rotation_4d = Rotation4D::new(*self.rotation_4d.q_left(), new_q_right);

        // Update cached values
        self.yaw_r += -delta_x * ROTATION_SENSITIVITY;
        self.pitch_r += delta_y * ROTATION_SENSITIVITY;
    }

    /// Get yaw angle (rotation around Y axis) in radians - for `q_left`
    #[must_use]
    pub const fn yaw_l(&self) -> f32 {
        self.yaw_l
    }

    /// Get pitch angle (rotation around X axis) in radians - for `q_left`
    #[must_use]
    pub const fn pitch_l(&self) -> f32 {
        self.pitch_l
    }

    /// Set `q_left` (3D orientation) from yaw and pitch angles
    pub fn set_yaw_pitch_l(&mut self, yaw: f32, pitch: f32) {
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.rotation_4d = Rotation4D::new(yaw_rot * pitch_rot, *self.rotation_4d.q_right());
        self.yaw_l = yaw;
        self.pitch_l = pitch;
    }

    /// Set yaw only for `q_left`, preserving current pitch
    pub fn set_yaw_l(&mut self, yaw: f32) {
        self.set_yaw_pitch_l_internal(yaw, self.pitch_l);
        self.yaw_l = yaw;
    }

    /// Set pitch only for `q_left`, preserving current yaw
    pub fn set_pitch_l(&mut self, pitch: f32) {
        self.set_yaw_pitch_l_internal(self.yaw_l, pitch);
        self.pitch_l = pitch;
    }

    fn set_yaw_pitch_l_internal(&mut self, yaw: f32, pitch: f32) {
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.rotation_4d = Rotation4D::new(yaw_rot * pitch_rot, *self.rotation_4d.q_right());
    }

    /// Get yaw angle for `q_right` (4D rotation in XW plane)
    #[must_use]
    pub const fn yaw_r(&self) -> f32 {
        self.yaw_r
    }

    /// Get pitch angle for `q_right` (4D rotation in YW plane)
    #[must_use]
    pub const fn pitch_r(&self) -> f32 {
        self.pitch_r
    }

    /// Set yaw only for `q_right`, preserving current pitch
    pub fn set_yaw_r(&mut self, yaw: f32) {
        let pitch = self.pitch_r;
        self.rotation_4d.set_q_right_from_yaw_pitch(yaw, pitch);
        self.yaw_r = yaw;
    }

    /// Set pitch only for `q_right`, preserving current yaw
    pub fn set_pitch_r(&mut self, pitch: f32) {
        let yaw = self.yaw_r;
        self.rotation_4d.set_q_right_from_yaw_pitch(yaw, pitch);
        self.pitch_r = pitch;
    }

    #[must_use]
    pub fn basis_4d(&self) -> [[f32; 4]; 4] {
        self.rotation_4d.basis_vectors()
    }

    /// Returns the slice-only rotation (identity q_left, actual q_right).
    ///
    /// Used to derive slice-normal direction and to project in-slice camera
    /// directions into world 4D without including the camera's 3D look rotation.
    #[must_use]
    pub fn slice_rotation(&self) -> Rotation4D {
        Rotation4D::new(UnitQuaternion::identity(), *self.rotation_4d.q_right())
    }

    #[must_use]
    pub fn slice_w_axis(&self) -> [f32; 4] {
        self.rotation_4d.basis_w()
    }

    #[must_use]
    pub fn is_slice_tilted(&self) -> bool {
        !self.rotation_4d.is_pure_3d()
    }

    #[must_use]
    pub fn direction_label_4d(&self, direction: SliceDirection) -> String {
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
        format_4d_vector(v, 0.01, 2)
    }

    fn project_3d_to_4d_with_basis(v3: Vector3<f32>, basis: &[[f32; 4]; 4]) -> Vector4<f32> {
        Vector4::new(
            v3.x * basis[0][0] + v3.y * basis[1][0] + v3.z * basis[2][0],
            v3.x * basis[0][1] + v3.y * basis[1][1] + v3.z * basis[2][1],
            v3.x * basis[0][2] + v3.y * basis[1][2] + v3.z * basis[2][2],
            v3.x * basis[0][3] + v3.y * basis[1][3] + v3.z * basis[2][3],
        )
    }

    #[must_use]
    pub fn project_3d_to_4d(&self, v3: Vector3<f32>) -> Vector4<f32> {
        Self::project_3d_to_4d_with_basis(v3, &self.rotation_4d.basis_vectors())
    }

    /// Projects a camera-local 3D direction into world 4D using only `q_right` slice orientation.
    ///
    /// This is the key bridge between in-slice movement (`q_left`) and tilted-slice world motion
    /// (`q_right`).
    #[must_use]
    pub fn project_camera_3d_to_world_4d(&self, v3: Vector3<f32>) -> Vector4<f32> {
        Self::project_3d_to_4d_with_basis(v3, &self.slice_rotation().basis_vectors())
    }

    fn camera_world_axes(&self) -> (Vector4<f32>, Vector4<f32>, Vector4<f32>, Vector4<f32>) {
        let right3 = self.right_vector();
        let up3 = self.up_vector();
        let forward3 = self.forward_vector();
        let right = self.project_camera_3d_to_world_4d(right3);
        let up = self.project_camera_3d_to_world_4d(up3);
        let forward = self.project_camera_3d_to_world_4d(forward3);

        let w_basis = self.slice_rotation().basis_w();
        let w_axis = Vector4::new(w_basis[0], w_basis[1], w_basis[2], w_basis[3]);

        (right, up, forward, w_axis)
    }

    /// Converts a world-space 4D direction into camera-frame components (R/U/F/K).
    #[must_use]
    pub fn world_vector_to_camera_frame(&self, world_vector: Vector4<f32>) -> Vector4<f32> {
        let (right, up, forward, w_axis) = self.camera_world_axes();
        Vector4::new(
            world_vector.dot(&right),
            world_vector.dot(&up),
            world_vector.dot(&forward),
            world_vector.dot(&w_axis),
        )
    }

    /// Applies one camera movement action in the mathematically split frame model.
    ///
    /// - 3D actions: derive direction from `q_left`, project through `q_right` slice basis.
    /// - Kata/Ana: move along slice normal from `(identity, q_right).basis_w()`.
    ///
    /// Do not collapse this to full `rotation_4d.basis_*` without updating camera semantics.
    pub fn apply_action(&mut self, action: CameraAction, speed: f32) {
        let (right, up, forward, w_axis) = self.camera_world_axes();
        match action {
            CameraAction::MoveForward => {
                self.position += forward * speed;
            }
            CameraAction::MoveBackward => {
                self.position -= forward * speed;
            }
            CameraAction::MoveLeft => {
                self.position -= right * speed;
            }
            CameraAction::MoveRight => {
                self.position += right * speed;
            }
            CameraAction::MoveUp => {
                self.position += up * speed;
            }
            CameraAction::MoveDown => {
                self.position -= up * speed;
            }
            CameraAction::MoveKata => {
                self.position += w_axis * speed;
            }
            CameraAction::MoveAna => {
                self.position -= w_axis * speed;
            }
        }
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

/// Camera movement actions for 4D navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAction {
    /// Move forward in the direction the camera is facing
    MoveForward,
    /// Move backward (opposite to camera facing direction)
    MoveBackward,
    /// Move left (relative to camera)
    MoveLeft,
    /// Move right (relative to camera)
    MoveRight,
    /// Move up (relative to camera)
    MoveUp,
    /// Move down (relative to camera)
    MoveDown,
    /// Move along slice-normal positive W direction (kata)
    MoveKata,
    /// Move along slice-normal negative W direction (ana)
    MoveAna,
}

impl std::fmt::Display for CameraAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraAction::MoveForward => write!(f, "MoveForward"),
            CameraAction::MoveBackward => write!(f, "MoveBackward"),
            CameraAction::MoveLeft => write!(f, "MoveLeft"),
            CameraAction::MoveRight => write!(f, "MoveRight"),
            CameraAction::MoveUp => write!(f, "MoveUp"),
            CameraAction::MoveDown => write!(f, "MoveDown"),
            CameraAction::MoveKata => write!(f, "MoveKata"),
            CameraAction::MoveAna => write!(f, "MoveAna"),
        }
    }
}

#[must_use]
pub fn format_4d_vector(v: [f32; 4], threshold: f32, precision: usize) -> String {
    let components: [(f32, &str); 4] = [(v[0], "X"), (v[1], "Y"), (v[2], "Z"), (v[3], "W")];

    let parts: Vec<String> = components
        .iter()
        .filter(|(val, _)| val.abs() >= threshold)
        .map(|(val, axis)| {
            if (val - 1.0).abs() < threshold {
                format!("+{axis}")
            } else if (val + 1.0).abs() < threshold {
                format!("-{axis}")
            } else {
                format!("{val:+.precision$}{axis}")
            }
        })
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
    use crate::test_utils::assert_approx_eq;
    use std::f32::consts::PI;

    fn vec4_from_arr(v: [f32; 4]) -> Vector4<f32> {
        Vector4::new(v[0], v[1], v[2], v[3])
    }

    #[test]
    fn test_camera_new() {
        let camera = Camera::new();
        assert_approx_eq(camera.position.x, 0.0, 1e-6);
        assert_approx_eq(camera.position.y, 0.0, 1e-6);
        assert_approx_eq(camera.position.z, -5.0, 1e-6);
        assert_approx_eq(camera.position.w, 0.0, 1e-6);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_camera_reset() {
        let mut camera = Camera::new();
        camera.position = Vector4::new(10.0, 20.0, 30.0, 3.0);
        camera.rotation_4d = Rotation4D::from_plane_angle(RotationPlane::XW, 0.5);

        camera.reset();

        assert_approx_eq(camera.position.x, 0.0, 1e-6);
        assert_approx_eq(camera.position.y, 0.0, 1e-6);
        assert_approx_eq(camera.position.z, -5.0, 1e-6);
        assert_approx_eq(camera.position.w, 0.0, 1e-6);
        assert!(camera.rotation_4d.is_pure_3d());
    }

    #[test]
    fn test_forward_vector_identity() {
        let camera = Camera {
            position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let forward = camera.forward_vector();
        assert_approx_eq(forward.x, 0.0, 1e-6);
        assert_approx_eq(forward.y, 0.0, 1e-6);
        assert_approx_eq(forward.z, 1.0, 1e-6);
    }

    #[test]
    fn test_right_vector_identity() {
        let camera = Camera {
            position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let right = camera.right_vector();
        assert_approx_eq(right.x, 1.0, 1e-6);
        assert_approx_eq(right.y, 0.0, 1e-6);
        assert_approx_eq(right.z, 0.0, 1e-6);
    }

    #[test]
    fn test_up_vector_identity() {
        let camera = Camera {
            position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let up = camera.up_vector();
        assert_approx_eq(up.x, 0.0, 1e-6);
        assert_approx_eq(up.y, 1.0, 1e-6);
        assert_approx_eq(up.z, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_yaw() {
        let camera = Camera {
            position: Vector4::new(0.0, 0.0, -5.0, 0.0),
            rotation_4d: Rotation4D::from_plane_angle(RotationPlane::XZ, PI / 2.0),
            ..Camera::new()
        };

        let forward = camera.forward_vector();
        assert_approx_eq(forward.x, 1.0, 1e-6);
        assert_approx_eq(forward.y, 0.0, 1e-6);
        assert_approx_eq(forward.z, 0.0, 1e-6);
    }

    #[test]
    fn test_forward_vector_pitch() {
        let camera = Camera {
            position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation_4d: Rotation4D::from_plane_angle(RotationPlane::YZ, PI / 4.0),
            ..Camera::new()
        };

        let forward = camera.forward_vector();
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;
        assert_approx_eq(forward.x, 0.0, 1e-6);
        assert_approx_eq(forward.y, -sqrt2_2, 1e-6);
        assert_approx_eq(forward.z, sqrt2_2, 1e-6);
    }

    #[test]
    fn test_rotate() {
        let mut camera = Camera {
            position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        camera.rotate(1.0, 0.0);
        let forward = camera.forward_vector();

        assert!(forward.x.abs() > 1e-6 || forward.z.abs() < 0.99);
    }

    #[test]
    fn test_orthonormal_basis() {
        let rotations = vec![
            Rotation4D::identity(),
            Rotation4D::from_plane_angle(RotationPlane::XY, PI / 4.0),
            Rotation4D::from_plane_angle(RotationPlane::XY, PI / 2.0),
            Rotation4D::from_plane_angle(RotationPlane::XZ, PI / 6.0),
        ];

        for rotation_4d in rotations {
            let camera = Camera {
                position: Vector4::zeros(),
                rotation_4d,
                ..Camera::new()
            };

            let forward = camera.forward_vector();
            let right = camera.right_vector();
            let up = camera.up_vector();

            assert_approx_eq(forward.dot(&right), 0.0, 1e-6);
            assert_approx_eq(forward.dot(&up), 0.0, 1e-6);
            assert_approx_eq(right.dot(&up), 0.0, 1e-6);

            assert_approx_eq(forward.norm(), 1.0, 1e-6);
            assert_approx_eq(right.norm(), 1.0, 1e-6);
            assert_approx_eq(up.norm(), 1.0, 1e-6);

            let cross = forward.cross(&right);
            assert_approx_eq(cross.x, up.x, 1e-6);
            assert_approx_eq(cross.y, up.y, 1e-6);
            assert_approx_eq(cross.z, up.z, 1e-6);
        }
    }

    #[test]
    fn test_coordinate_consistency_forward() {
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let initial_z = camera.position.z;
        let forward = camera.forward_vector();

        assert_approx_eq(forward.z, 1.0, 1e-6);

        camera.move_along(forward, 1.0);
        assert_approx_eq(camera.position.z - initial_z, 1.0, 1e-6);
    }

    #[test]
    fn test_coordinate_consistency_right() {
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let initial_x = camera.position.x;
        let right = camera.right_vector();

        assert_approx_eq(right.x, 1.0, 1e-6);

        camera.move_along(right, 1.0);
        assert_approx_eq(camera.position.x - initial_x, 1.0, 1e-6);
    }

    #[test]
    fn test_world_vector_to_camera_frame_identity() {
        let camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let v = Vector4::new(1.0, -2.0, 3.0, -4.0);
        let local = camera.world_vector_to_camera_frame(v);
        assert_approx_eq(local.x, 1.0, 1e-6);
        assert_approx_eq(local.y, -2.0, 1e-6);
        assert_approx_eq(local.z, 3.0, 1e-6);
        assert_approx_eq(local.w, -4.0, 1e-6);
    }

    #[test]
    fn test_world_vector_to_camera_frame_uses_camera_axes() {
        let camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::from_6_plane_angles(0.31, -0.2, 0.18, 0.42, -0.27, 0.15),
            ..Camera::new()
        };

        let right_world = camera.project_camera_3d_to_world_4d(camera.right_vector());
        let local = camera.world_vector_to_camera_frame(right_world);
        assert_approx_eq(local.x, 1.0, 1e-6);
        assert_approx_eq(local.y, 0.0, 1e-6);
        assert_approx_eq(local.z, 0.0, 1e-6);
        assert_approx_eq(local.w, 0.0, 1e-6);
    }

    #[test]
    fn test_backward_movement_inverts_forward() {
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        let initial_z = camera.position.z;
        let forward = camera.forward_vector();

        camera.move_along(forward, 1.0);
        let after_forward_z = camera.position.z;

        camera.position.z = initial_z;
        camera.move_along(-forward, 1.0);
        let after_backward_z = camera.position.z;

        assert_approx_eq(after_forward_z - initial_z, 1.0, 1e-6);
        assert_approx_eq(after_backward_z - initial_z, -1.0, 1e-6);
    }

    #[test]
    fn test_move_along() {
        let mut camera = Camera::new();
        camera.move_along(Vector3::new(1.0, 2.0, 3.0), 0.5);

        assert_approx_eq(camera.position.x, 0.5, 1e-6);
        assert_approx_eq(camera.position.y, 1.0, 1e-6);
        assert_approx_eq(camera.position.z, -5.0 + 1.5, 1e-6);
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
            forward.x.abs() > 1e-6 || forward.z.abs() < 0.99,
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
    fn test_apply_action_move_forward() {
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::identity(),
            ..Camera::new()
        };

        camera.apply_action(CameraAction::MoveForward, 1.0);

        assert_approx_eq(camera.position.z, 1.0, 1e-6);
    }

    #[test]
    fn test_apply_action_move_forward_uses_camera_basis() {
        let yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 2.0);
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::new(yaw, UnitQuaternion::identity()),
            ..Camera::new()
        };

        camera.apply_action(CameraAction::MoveForward, 1.0);

        assert_approx_eq(camera.position.x, 1.0, 1e-6);
        assert_approx_eq(camera.position.y, 0.0, 1e-6);
        assert_approx_eq(camera.position.z, 0.0, 1e-6);
        assert_approx_eq(camera.position.w, 0.0, 1e-6);
    }

    #[test]
    fn test_apply_action_moves_follow_3d_camera_frame_with_tilted_slice() {
        let base_camera = Camera {
            position: Vector4::new(0.3, -0.4, 1.1, -0.7),
            rotation_4d: Rotation4D::from_6_plane_angles(0.37, -0.21, 0.44, 0.29, -0.18, 0.53),
            ..Camera::new()
        };

        let right = base_camera.project_camera_3d_to_world_4d(base_camera.right_vector());
        let up = base_camera.project_camera_3d_to_world_4d(base_camera.up_vector());
        let forward = base_camera.project_camera_3d_to_world_4d(base_camera.forward_vector());
        let step = 0.75;
        let cases = [
            (CameraAction::MoveRight, right),
            (CameraAction::MoveLeft, -right),
            (CameraAction::MoveUp, up),
            (CameraAction::MoveDown, -up),
            (CameraAction::MoveForward, forward),
            (CameraAction::MoveBackward, -forward),
        ];

        for (action, expected_dir) in cases {
            let mut camera = base_camera.clone();
            let before = camera.position;
            camera.apply_action(action, step);
            let delta = camera.position - before;

            assert_approx_eq(delta.x, expected_dir.x * step, 1e-6);
            assert_approx_eq(delta.y, expected_dir.y * step, 1e-6);
            assert_approx_eq(delta.z, expected_dir.z * step, 1e-6);
            assert_approx_eq(delta.w, expected_dir.w * step, 1e-6);
        }
    }

    #[test]
    fn test_3d_moves_do_not_change_world_w_when_slice_not_tilted() {
        let yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.7);
        let pitch = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -0.3);
        let base_camera = Camera {
            position: Vector4::new(-0.2, 0.9, -1.3, 0.4),
            rotation_4d: Rotation4D::new(yaw * pitch, UnitQuaternion::identity()),
            ..Camera::new()
        };

        let actions = [
            CameraAction::MoveRight,
            CameraAction::MoveLeft,
            CameraAction::MoveUp,
            CameraAction::MoveDown,
            CameraAction::MoveForward,
            CameraAction::MoveBackward,
        ];

        for action in actions {
            let mut camera = base_camera.clone();
            let before = camera.position;
            camera.apply_action(action, 1.0);
            let delta = camera.position - before;
            assert_approx_eq(delta.w, 0.0, 1e-6);
        }
    }

    #[test]
    fn test_3d_moves_can_change_world_w_when_slice_tilted() {
        let mut camera = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::from_6_plane_angles(0.4, -0.2, 0.1, 0.6, 0.0, 0.0),
            ..Camera::new()
        };

        let before = camera.position;
        camera.apply_action(CameraAction::MoveForward, 1.0);
        let delta = camera.position - before;
        assert!(delta.w.abs() > 1e-6);
    }

    #[test]
    fn test_kata_ana_move_along_slice_normal() {
        let base_camera = Camera {
            position: Vector4::new(0.1, -0.3, 0.7, -0.9),
            rotation_4d: Rotation4D::from_6_plane_angles(-0.19, 0.33, 0.47, -0.55, 0.26, -0.31),
            ..Camera::new()
        };

        let w_axis = vec4_from_arr(base_camera.slice_rotation().basis_w());

        let mut kata = base_camera.clone();
        let kata_before = kata.position;
        kata.apply_action(CameraAction::MoveKata, 0.8);
        let kata_delta = kata.position - kata_before;
        assert_approx_eq(kata_delta.x, w_axis.x * 0.8, 1e-6);
        assert_approx_eq(kata_delta.y, w_axis.y * 0.8, 1e-6);
        assert_approx_eq(kata_delta.z, w_axis.z * 0.8, 1e-6);
        assert_approx_eq(kata_delta.w, w_axis.w * 0.8, 1e-6);

        let mut ana = base_camera.clone();
        let ana_before = ana.position;
        ana.apply_action(CameraAction::MoveAna, 0.8);
        let ana_delta = ana.position - ana_before;
        assert_approx_eq(ana_delta.x, -w_axis.x * 0.8, 1e-6);
        assert_approx_eq(ana_delta.y, -w_axis.y * 0.8, 1e-6);
        assert_approx_eq(ana_delta.z, -w_axis.z * 0.8, 1e-6);
        assert_approx_eq(ana_delta.w, -w_axis.w * 0.8, 1e-6);
    }

    #[test]
    fn test_kata_ana_independent_of_q_left_yaw_pitch() {
        let q_right_tilt =
            *Rotation4D::from_6_plane_angles(0.0, 0.0, 0.0, 0.41, -0.27, 0.18).q_right();

        let mut camera_a = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::new(UnitQuaternion::identity(), q_right_tilt),
            ..Camera::new()
        };
        let mut camera_b = Camera {
            position: Vector4::zeros(),
            rotation_4d: Rotation4D::new(
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.73)
                    * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -0.38),
                q_right_tilt,
            ),
            ..Camera::new()
        };

        let speed = 0.6;
        let before_a = camera_a.position;
        camera_a.apply_action(CameraAction::MoveKata, speed);
        let delta_a = camera_a.position - before_a;

        let before_b = camera_b.position;
        camera_b.apply_action(CameraAction::MoveKata, speed);
        let delta_b = camera_b.position - before_b;

        assert_approx_eq(delta_a.x, delta_b.x, 1e-6);
        assert_approx_eq(delta_a.y, delta_b.y, 1e-6);
        assert_approx_eq(delta_a.z, delta_b.z, 1e-6);
        assert_approx_eq(delta_a.w, delta_b.w, 1e-6);
    }

    #[test]
    fn test_kata_ana_do_not_change_xyz_in_pure_3d_slice() {
        let yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 3.0);
        let base_camera = Camera {
            position: Vector4::new(0.2, -0.1, 0.4, 1.7),
            rotation_4d: Rotation4D::new(yaw, UnitQuaternion::identity()),
            ..Camera::new()
        };

        let mut kata = base_camera.clone();
        let before_kata = kata.position;
        kata.apply_action(CameraAction::MoveKata, 0.9);
        let kata_delta = kata.position - before_kata;
        assert_approx_eq(kata_delta.x, 0.0, 1e-6);
        assert_approx_eq(kata_delta.y, 0.0, 1e-6);
        assert_approx_eq(kata_delta.z, 0.0, 1e-6);
        assert_approx_eq(kata_delta.w, 0.9, 1e-6);

        let mut ana = base_camera.clone();
        let before_ana = ana.position;
        ana.apply_action(CameraAction::MoveAna, 0.9);
        let ana_delta = ana.position - before_ana;
        assert_approx_eq(ana_delta.x, 0.0, 1e-6);
        assert_approx_eq(ana_delta.y, 0.0, 1e-6);
        assert_approx_eq(ana_delta.z, 0.0, 1e-6);
        assert_approx_eq(ana_delta.w, -0.9, 1e-6);
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

    #[test]
    fn test_rotate_4d_circular_drag_returns_to_start() {
        // Simulate circular drag with larger values (like actual mouse drag)
        let mut camera = Camera::new();

        // Right, Down, Left, Up - larger values like real drag
        camera.rotate_4d(100.0, 0.0);
        camera.rotate_4d(0.0, 100.0);
        camera.rotate_4d(-100.0, 0.0);
        camera.rotate_4d(0.0, -100.0);

        let final_q_right = *camera.rotation_4d.q_right();

        // Should be back to identity
        let expected = UnitQuaternion::identity();
        assert_approx_eq(final_q_right.w, expected.w, 1e-3);
        assert_approx_eq(final_q_right.i, expected.i, 1e-3);
        assert_approx_eq(final_q_right.j, expected.j, 1e-3);
        assert_approx_eq(final_q_right.k, expected.k, 1e-3);
    }

    #[test]
    fn test_rotate_4d_horizontal_then_back_returns_to_start() {
        // Drag right then left by same amount should cancel out
        let mut camera = Camera::new();

        let initial_q_right = *camera.rotation_4d.q_right();

        // Right then left - larger values
        camera.rotate_4d(100.0, 0.0);
        camera.rotate_4d(-100.0, 0.0);

        let final_q_right = *camera.rotation_4d.q_right();

        assert_approx_eq(final_q_right.w, initial_q_right.w, 1e-3);
        assert_approx_eq(final_q_right.i, initial_q_right.i, 1e-3);
        assert_approx_eq(final_q_right.j, initial_q_right.j, 1e-3);
        assert_approx_eq(final_q_right.k, initial_q_right.k, 1e-3);
    }

    #[test]
    fn test_rotate_4d_vertical_then_back_returns_to_start() {
        // Drag down then up by same amount should cancel out
        let mut camera = Camera::new();

        let initial_q_right = *camera.rotation_4d.q_right();

        // Down then up
        camera.rotate_4d(0.0, 100.0);
        camera.rotate_4d(0.0, -100.0);

        let final_q_right = *camera.rotation_4d.q_right();

        assert_approx_eq(final_q_right.w, initial_q_right.w, 1e-3);
        assert_approx_eq(final_q_right.i, initial_q_right.i, 1e-3);
        assert_approx_eq(final_q_right.j, initial_q_right.j, 1e-3);
        assert_approx_eq(final_q_right.k, initial_q_right.k, 1e-3);
    }

    #[test]
    fn test_slice_w_axis_identity() {
        let camera = Camera::new();
        let w_axis = camera.slice_w_axis();
        assert_approx_eq(w_axis[0], 0.0, 1e-6);
        assert_approx_eq(w_axis[1], 0.0, 1e-6);
        assert_approx_eq(w_axis[2], 0.0, 1e-6);
        assert_approx_eq(w_axis[3], 1.0, 1e-6);
    }

    #[test]
    fn test_slice_w_axis_after_4d_rotation() {
        let mut camera = Camera::new();
        camera.rotate_4d(1.0, 0.0);
        let w_axis = camera.slice_w_axis();
        let norm = (w_axis[0] * w_axis[0]
            + w_axis[1] * w_axis[1]
            + w_axis[2] * w_axis[2]
            + w_axis[3] * w_axis[3])
            .sqrt();
        assert_approx_eq(norm, 1.0, 1e-4);
    }

    #[test]
    fn test_project_3d_to_4d_identity() {
        let camera = Camera::new();
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let v4 = camera.project_3d_to_4d(v3);
        assert_approx_eq(v4.x, 1.0, 1e-6);
        assert_approx_eq(v4.y, 2.0, 1e-6);
        assert_approx_eq(v4.z, 3.0, 1e-6);
        assert_approx_eq(v4.w, 0.0, 1e-6);
    }

    #[test]
    fn test_project_3d_to_4d_after_4d_rotation() {
        let mut camera = Camera::new();
        camera.rotate_4d(200.0, 0.0);
        let v3 = Vector3::new(1.0, 0.0, 0.0);
        let v4 = camera.project_3d_to_4d(v3);
        let norm = (v4.x * v4.x + v4.y * v4.y + v4.z * v4.z + v4.w * v4.w).sqrt();
        assert_approx_eq(norm, 1.0, 1e-4);
        assert!(
            v4.w.abs() > 0.1,
            "4D rotation should mix x into w, got w={}",
            v4.w
        );
    }

    #[test]
    fn test_is_slice_tilted_identity() {
        let camera = Camera::new();
        assert!(!camera.is_slice_tilted());
    }

    #[test]
    fn test_is_slice_tilted_after_4d_rotation() {
        let mut camera = Camera::new();
        camera.rotate_4d(1.0, 0.0);
        assert!(camera.is_slice_tilted());
    }

    #[test]
    fn test_direction_label_4d_forward_identity() {
        let camera = Camera::new();
        let label = camera.direction_label_4d(SliceDirection::Forward);
        assert_eq!(label, "+Z");
    }

    #[test]
    fn test_direction_label_4d_right_identity() {
        let camera = Camera::new();
        let label = camera.direction_label_4d(SliceDirection::Right);
        assert_eq!(label, "+X");
    }

    #[test]
    fn test_direction_label_4d_up_identity() {
        let camera = Camera::new();
        let label = camera.direction_label_4d(SliceDirection::Up);
        assert_eq!(label, "+Y");
    }

    #[test]
    fn test_direction_label_4d_w_positive_identity() {
        let camera = Camera::new();
        let label = camera.direction_label_4d(SliceDirection::WPositive);
        assert_eq!(label, "+W");
    }

    #[test]
    fn test_camera_action_display() {
        assert_eq!(CameraAction::MoveForward.to_string(), "MoveForward");
        assert_eq!(CameraAction::MoveUp.to_string(), "MoveUp");
        assert_eq!(CameraAction::MoveKata.to_string(), "MoveKata");
    }
}
