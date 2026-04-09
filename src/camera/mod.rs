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

pub const ROTATION_SENSITIVITY: f32 = 0.005;
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
        Self::new()
    }
}

impl Camera {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: DEFAULT_CAMERA_POSITION,
            rotation_4d: Rotation4D::identity(),
            yaw_l: 0.0,
            pitch_l: 0.0,
            yaw_r: 0.0,
            pitch_r: 0.0,
        }
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

    /// Returns the slice-only rotation (identity q_left, actual q_right).
    ///
    /// Used to derive slice-normal direction and to project in-slice camera
    /// directions into world 4D without including the camera's 3D look rotation.
    #[must_use]
    pub fn slice_rotation(&self) -> Rotation4D {
        Rotation4D::new(UnitQuaternion::identity(), *self.rotation_4d.q_right())
    }

    #[cfg(test)]
    #[must_use]
    pub fn basis_4d(&self) -> [[f32; 4]; 4] {
        self.rotation_4d.basis_vectors()
    }

    #[cfg(test)]
    #[must_use]
    pub fn slice_w_axis(&self) -> [f32; 4] {
        self.rotation_4d.basis_w()
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_slice_tilted(&self) -> bool {
        !self.rotation_4d.is_pure_3d()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn direction_label_4d(&self, direction: Direction4D) -> String {
        let basis = self.rotation_4d.basis_vectors();
        let v = direction.to_basis_vector(basis);
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
    pub fn apply_action(&mut self, action: Direction4D, speed: f32) {
        let (right, up, forward, w_axis) = self.camera_world_axes();
        match action {
            Direction4D::Forward => {
                self.position += forward * speed;
            }
            Direction4D::Backward => {
                self.position -= forward * speed;
            }
            Direction4D::Left => {
                self.position -= right * speed;
            }
            Direction4D::Right => {
                self.position += right * speed;
            }
            Direction4D::Up => {
                self.position += up * speed;
            }
            Direction4D::Down => {
                self.position -= up * speed;
            }
            Direction4D::Kata => {
                self.position += w_axis * speed;
            }
            Direction4D::Ana => {
                self.position -= w_axis * speed;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction4D {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
    Kata,
    Ana,
}

impl Direction4D {
    #[cfg(test)]
    fn to_basis_vector(self, basis: [[f32; 4]; 4]) -> [f32; 4] {
        match self {
            Direction4D::Forward => basis[2],
            Direction4D::Backward => [-basis[2][0], -basis[2][1], -basis[2][2], -basis[2][3]],
            Direction4D::Left => [-basis[0][0], -basis[0][1], -basis[0][2], -basis[0][3]],
            Direction4D::Right => basis[0],
            Direction4D::Up => basis[1],
            Direction4D::Down => [-basis[1][0], -basis[1][1], -basis[1][2], -basis[1][3]],
            Direction4D::Kata => basis[3],
            Direction4D::Ana => [-basis[3][0], -basis[3][1], -basis[3][2], -basis[3][3]],
        }
    }
}

impl std::fmt::Display for Direction4D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction4D::Forward => write!(f, "Forward"),
            Direction4D::Backward => write!(f, "Backward"),
            Direction4D::Left => write!(f, "Left"),
            Direction4D::Right => write!(f, "Right"),
            Direction4D::Up => write!(f, "Up"),
            Direction4D::Down => write!(f, "Down"),
            Direction4D::Kata => write!(f, "Kata"),
            Direction4D::Ana => write!(f, "Ana"),
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
mod tests;
