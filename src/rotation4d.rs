//! 4D rotation using dual quaternion representation
//!
//! SO(4) rotations can be represented by a pair of unit quaternions (q_L, q_R).
//! A 4D point v (represented as a quaternion) is rotated as:
//!   v' = q_L * v * q_R^(-1)
//!
//! This is the double-cover of SO(4) by S^3 × S^3.

use nalgebra::{UnitQuaternion, Vector3, Vector4};

#[derive(Clone, Debug)]
pub struct Rotation4D {
    q_left: UnitQuaternion<f32>,
    q_right: UnitQuaternion<f32>,
}

impl Default for Rotation4D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Rotation4D {
    pub fn identity() -> Self {
        Self {
            q_left: UnitQuaternion::identity(),
            q_right: UnitQuaternion::identity(),
        }
    }

    pub fn new(q_left: UnitQuaternion<f32>, q_right: UnitQuaternion<f32>) -> Self {
        Self { q_left, q_right }
    }

    pub fn q_left(&self) -> &UnitQuaternion<f32> {
        &self.q_left
    }

    pub fn q_right(&self) -> &UnitQuaternion<f32> {
        &self.q_right
    }

    pub fn get_q_left_as_yaw_pitch(&self) -> (f32, f32) {
        quaternion_to_yaw_pitch(self.q_left())
    }

    pub fn set_q_left_from_yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        self.q_left = quaternion_from_yaw_pitch(yaw, pitch);
    }

    pub fn get_q_right_as_yaw_pitch(&self) -> (f32, f32) {
        quaternion_to_yaw_pitch_4d(self.q_right())
    }

    pub fn set_q_right_from_yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        self.q_right = quaternion_from_yaw_pitch_4d(yaw, pitch);
    }

    pub fn set_q_left_from_yaw_pitch_preserving_right(&mut self, yaw: f32, pitch: f32) {
        self.q_left = quaternion_from_yaw_pitch(yaw, pitch);
    }

    pub fn set_q_right_from_yaw_pitch_preserving_left(&mut self, yaw: f32, pitch: f32) {
        self.q_right = quaternion_from_yaw_pitch_4d(yaw, pitch);
    }

    /// Returns the inverse rotation.
    pub fn inverse(&self) -> Self {
        Self {
            q_left: self.q_left.inverse(),
            q_right: self.q_right.inverse(),
        }
    }

    /// Returns inverse of q_right only, with identity for q_left.
    /// Used for camera transformations.
    pub fn inverse_q_right_only(&self) -> Self {
        Self {
            q_left: UnitQuaternion::identity(),
            q_right: self.q_right.inverse(),
        }
    }

    /// Composes this rotation with another: other.then(self).
    pub fn then(&self, other: &Self) -> Self {
        Self {
            q_left: other.q_left * self.q_left,
            q_right: self.q_right * other.q_right,
        }
    }

    pub fn rotate_point(&self, p: [f32; 4]) -> [f32; 4] {
        let v_quat = quat_from_4d(p);
        let q_right_inv = self.q_right.inverse();
        let rotated = *self.q_left * v_quat * *q_right_inv;
        quat_to_4d(&rotated)
    }

    pub fn rotate_vector(&self, v: Vector4<f32>) -> Vector4<f32> {
        let arr = [v.x, v.y, v.z, v.w];
        let rotated = self.rotate_point(arr);
        Vector4::new(rotated[0], rotated[1], rotated[2], rotated[3])
    }

    pub fn basis_vectors(&self) -> [[f32; 4]; 4] {
        [
            self.rotate_point([1.0, 0.0, 0.0, 0.0]),
            self.rotate_point([0.0, 1.0, 0.0, 0.0]),
            self.rotate_point([0.0, 0.0, 1.0, 0.0]),
            self.rotate_point([0.0, 0.0, 0.0, 1.0]),
        ]
    }

    pub fn basis_x(&self) -> [f32; 4] {
        self.rotate_point([1.0, 0.0, 0.0, 0.0])
    }

    pub fn basis_y(&self) -> [f32; 4] {
        self.rotate_point([0.0, 1.0, 0.0, 0.0])
    }

    pub fn basis_z(&self) -> [f32; 4] {
        self.rotate_point([0.0, 0.0, 1.0, 0.0])
    }

    pub fn basis_w(&self) -> [f32; 4] {
        self.rotate_point([0.0, 0.0, 0.0, 1.0])
    }

    pub fn from_plane_angle(plane: RotationPlane, angle: f32) -> Self {
        let (sin_a, cos_a) = angle.sin_cos();

        let q = UnitQuaternion::from_quaternion(nalgebra::Quaternion::from_parts(
            cos_a,
            match plane {
                RotationPlane::XY => Vector3::new(0.0, 0.0, sin_a),
                RotationPlane::XZ => Vector3::new(0.0, sin_a, 0.0),
                RotationPlane::YZ => Vector3::new(sin_a, 0.0, 0.0),
                RotationPlane::XW => Vector3::new(-sin_a, 0.0, 0.0),
                RotationPlane::YW => Vector3::new(0.0, -sin_a, 0.0),
                RotationPlane::ZW => Vector3::new(0.0, 0.0, -sin_a),
            },
        ));

        match plane {
            RotationPlane::XY | RotationPlane::XZ | RotationPlane::YZ => {
                let half_angle = angle * 0.5;
                let (sin_h, cos_h) = half_angle.sin_cos();
                let q_half = UnitQuaternion::from_quaternion(nalgebra::Quaternion::from_parts(
                    cos_h,
                    match plane {
                        RotationPlane::XY => Vector3::new(0.0, 0.0, sin_h),
                        RotationPlane::XZ => Vector3::new(0.0, sin_h, 0.0),
                        RotationPlane::YZ => Vector3::new(sin_h, 0.0, 0.0),
                        _ => Vector3::zeros(),
                    },
                ));
                Self {
                    q_left: q_half,
                    q_right: q_half,
                }
            }
            RotationPlane::XW | RotationPlane::YW | RotationPlane::ZW => Self {
                q_left: q,
                q_right: UnitQuaternion::identity(),
            },
        }
    }

    pub fn from_axis_angle_3d(axis: Vector3<f32>, angle: f32) -> Self {
        let axis_normalized = axis.normalize();
        let q =
            UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(axis_normalized), angle);
        Self {
            q_left: q,
            q_right: q,
        }
    }

    pub fn from_3d_rotation(q: &UnitQuaternion<f32>) -> Self {
        Self {
            q_left: *q,
            q_right: UnitQuaternion::identity(),
        }
    }

    pub fn get_w_component_of_basis(&self) -> [f32; 4] {
        let basis = self.basis_vectors();
        [basis[0][3], basis[1][3], basis[2][3], basis[3][3]]
    }

    pub fn is_pure_3d(&self) -> bool {
        let w_components = self.get_w_component_of_basis();
        let eps = 1e-6;
        w_components[0].abs() < eps
            && w_components[1].abs() < eps
            && w_components[2].abs() < eps
            && (w_components[3] - 1.0).abs() < eps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationPlane {
    XY,
    XZ,
    YZ,
    XW,
    YW,
    ZW,
}

fn quat_from_4d(p: [f32; 4]) -> nalgebra::Quaternion<f32> {
    nalgebra::Quaternion::new(p[3], p[0], p[1], p[2])
}

fn quat_to_4d(q: &nalgebra::Quaternion<f32>) -> [f32; 4] {
    [q.i, q.j, q.k, q.w]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_approx_eq, assert_vec_approx_eq};
    use std::f32::consts::PI;

    #[test]
    fn test_identity() {
        let rot = Rotation4D::identity();
        let p = [1.0, 2.0, 3.0, 4.0];
        let result = rot.rotate_point(p);
        assert_vec_approx_eq(result, p, 1e-6);
    }

    #[test]
    fn test_inverse() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        let p = [1.0, 2.0, 3.0, 4.0];
        let rotated = rot.rotate_point(p);
        let back = rot.inverse().rotate_point(rotated);
        assert_vec_approx_eq(back, p, 1e-5);
    }

    #[test]
    fn test_xy_rotation() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::XY, PI / 2.0);
        let result = rot.rotate_point([1.0, 0.0, 0.0, 0.0]);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], 1.0, 1e-6);
        assert_approx_eq(result[2], 0.0, 1e-6);
        assert_approx_eq(result[3], 0.0, 1e-6);
    }

    #[test]
    fn test_xw_rotation() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 2.0);
        let result = rot.rotate_point([1.0, 0.0, 0.0, 0.0]);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], 0.0, 1e-6);
        assert_approx_eq(result[2], 0.0, 1e-6);
        assert_approx_eq(result[3], 1.0, 1e-6);
    }

    #[test]
    fn test_zw_rotation() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::ZW, PI / 2.0);
        let result = rot.rotate_point([0.0, 0.0, 1.0, 0.0]);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], 0.0, 1e-6);
        assert_approx_eq(result[2], 0.0, 1e-6);
        assert_approx_eq(result[3], 1.0, 1e-6);
    }

    #[test]
    fn test_yw_rotation() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::YW, PI / 2.0);
        let result = rot.rotate_point([0.0, 1.0, 0.0, 0.0]);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], 0.0, 1e-6);
        assert_approx_eq(result[2], 0.0, 1e-6);
        assert_approx_eq(result[3], 1.0, 1e-6);
    }

    #[test]
    fn test_basis_vectors_identity() {
        let rot = Rotation4D::identity();
        let basis = rot.basis_vectors();

        assert_vec_approx_eq(basis[0], [1.0, 0.0, 0.0, 0.0], 1e-6);
        assert_vec_approx_eq(basis[1], [0.0, 1.0, 0.0, 0.0], 1e-6);
        assert_vec_approx_eq(basis[2], [0.0, 0.0, 1.0, 0.0], 1e-6);
        assert_vec_approx_eq(basis[3], [0.0, 0.0, 0.0, 1.0], 1e-6);
    }

    #[test]
    fn test_basis_vectors_after_xw_rotation() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        let basis = rot.basis_vectors();

        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        // XW rotation by 45°:
        // X axis should rotate into XW plane (have W component)
        // W axis should rotate into XW plane (have X component)
        assert_approx_eq(basis[0][0].abs(), sqrt2_2, 1e-5);
        assert_approx_eq(basis[0][3].abs(), sqrt2_2, 1e-5);
        assert_approx_eq(basis[3][0].abs(), sqrt2_2, 1e-5);
        assert_approx_eq(basis[3][3].abs(), sqrt2_2, 1e-5);

        // Verify orthonormality
        for i in 0..4 {
            let len: f32 = basis[i].iter().map(|x| x.powi(2)).sum();
            assert_approx_eq(len, 1.0, 1e-5);
        }

        for i in 0..4 {
            for j in (i + 1)..4 {
                let dot: f32 = basis[i]
                    .iter()
                    .zip(basis[j].iter())
                    .map(|(a, b)| a * b)
                    .sum();
                assert_approx_eq(dot, 0.0, 1e-5);
            }
        }
    }

    #[test]
    fn test_rotation_preserves_length() {
        let p: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let original_len = (p[0].powi(2) + p[1].powi(2) + p[2].powi(2) + p[3].powi(2)).sqrt();

        let planes = [
            RotationPlane::XY,
            RotationPlane::XZ,
            RotationPlane::YZ,
            RotationPlane::XW,
            RotationPlane::YW,
            RotationPlane::ZW,
        ];

        for plane in planes {
            for angle in [0.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0] {
                let rot = Rotation4D::from_plane_angle(plane, angle);
                let result = rot.rotate_point(p);
                let new_len =
                    (result[0].powi(2) + result[1].powi(2) + result[2].powi(2) + result[3].powi(2))
                        .sqrt();
                assert_approx_eq(new_len, original_len, 1e-5);
            }
        }
    }

    #[test]
    fn test_composition() {
        let rot1 = Rotation4D::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let rot2 = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        let combined = rot1.then(&rot2);

        let p = [1.0, 0.0, 0.0, 0.0];
        let step1 = rot1.rotate_point(p);
        let step2 = rot2.rotate_point(step1);
        let direct = combined.rotate_point(p);

        assert_vec_approx_eq(step2, direct, 1e-5);
    }

    #[test]
    fn test_composition_identity() {
        let rot = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        let identity = rot.then(&rot.inverse());

        let p = [1.0, 2.0, 3.0, 4.0];
        let result = identity.rotate_point(p);
        assert_vec_approx_eq(result, p, 1e-5);
    }

    #[test]
    fn test_is_pure_3d() {
        let identity = Rotation4D::identity();
        assert!(identity.is_pure_3d());

        let rot_3d = Rotation4D::from_plane_angle(RotationPlane::XY, PI / 4.0);
        assert!(rot_3d.is_pure_3d());

        let rot_4d = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        assert!(!rot_4d.is_pure_3d());
    }

    #[test]
    fn test_double_rotation() {
        let rot_xy = Rotation4D::from_plane_angle(RotationPlane::XY, PI / 4.0);
        let rot_xw = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 4.0);
        let combined = rot_xy.then(&rot_xw);

        let basis = combined.basis_vectors();

        for i in 0..4 {
            let len = basis[i].iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
            assert_approx_eq(len, 1.0, 1e-6);
        }

        for i in 0..4 {
            for j in (i + 1)..4 {
                let dot: f32 = basis[i]
                    .iter()
                    .zip(basis[j].iter())
                    .map(|(a, b)| a * b)
                    .sum();
                assert_approx_eq(dot, 0.0, 1e-6);
            }
        }
    }

    #[test]
    fn test_orthonormal_basis_after_multiple_rotations() {
        let rot1 = Rotation4D::from_plane_angle(RotationPlane::XW, PI / 6.0);
        let rot2 = Rotation4D::from_plane_angle(RotationPlane::YW, PI / 4.0);
        let rot3 = Rotation4D::from_plane_angle(RotationPlane::ZW, PI / 3.0);
        let combined = rot1.then(&rot2).then(&rot3);

        let basis = combined.basis_vectors();

        for i in 0..4 {
            let len = basis[i].iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
            assert_approx_eq(len, 1.0, 1e-5);
        }

        for i in 0..4 {
            for j in (i + 1)..4 {
                let dot: f32 = basis[i]
                    .iter()
                    .zip(basis[j].iter())
                    .map(|(a, b)| a * b)
                    .sum();
                assert_approx_eq(dot, 0.0, 1e-5);
            }
        }
    }
}

pub fn quaternion_to_yaw_pitch(q: &UnitQuaternion<f32>) -> (f32, f32) {
    let forward = q * Vector3::new(0.0, 0.0, 1.0);
    let yaw = forward.x.atan2(forward.z);
    let horizontal_len = (forward.x * forward.x + forward.z * forward.z).sqrt();
    let pitch = forward.y.atan2(horizontal_len);
    (yaw, pitch)
}

pub fn quaternion_from_yaw_pitch(yaw: f32, pitch: f32) -> UnitQuaternion<f32> {
    let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
    let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
    yaw_rot * pitch_rot
}

pub fn quaternion_to_yaw_pitch_4d(q: &UnitQuaternion<f32>) -> (f32, f32) {
    // For XW plane rotation: how much has X rotated toward W?
    // For YW plane rotation: how much has Y rotated toward W?
    // Using the same pattern as q_left but adapted for 4D
    let x_axis = q * Vector3::new(1.0, 0.0, 0.0);
    let y_axis = q * Vector3::new(0.0, 1.0, 0.0);

    // XW plane: X component indicates rotation toward W
    let yaw = x_axis.z.atan2(x_axis.x);
    // YW plane: Y component indicates rotation toward W
    let pitch = y_axis.z.atan2(y_axis.y);

    (yaw, pitch)
}

pub fn quaternion_from_yaw_pitch_4d(yaw: f32, pitch: f32) -> UnitQuaternion<f32> {
    // XW plane for yaw (horizontal), YW plane for pitch (vertical)
    let yaw_rot = Rotation4D::from_plane_angle(RotationPlane::XW, yaw);
    let pitch_rot = Rotation4D::from_plane_angle(RotationPlane::YW, pitch);
    // Combine in same order as rotate_4d: tilt_xw * tilt_yw (XW applied last)
    *yaw_rot.q_left() * *pitch_rot.q_left()
}

#[cfg(test)]
mod yaw_pitch_tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;
    use std::f32::consts::PI;

    #[test]
    fn test_quaternion_from_yaw_pitch_identity() {
        let q = quaternion_from_yaw_pitch(0.0, 0.0);
        let forward = q * Vector3::new(0.0, 0.0, 1.0);
        assert_approx_eq(forward.x, 0.0, 1e-6);
        assert_approx_eq(forward.y, 0.0, 1e-6);
        assert_approx_eq(forward.z, 1.0, 1e-6);
    }

    #[test]
    fn test_quaternion_from_yaw_pitch_nonzero() {
        let q = quaternion_from_yaw_pitch(PI / 4.0, PI / 6.0);
        let forward = q * Vector3::new(0.0, 0.0, 1.0);
        assert!(forward.x.abs() > 0.1, "yaw should rotate forward.x");
        assert!(forward.y.abs() > 0.1, "pitch should rotate forward.y");
    }

    #[test]
    fn test_quaternion_to_yaw_pitch_reasonable_range() {
        let q = quaternion_from_yaw_pitch(PI / 4.0, PI / 6.0);
        let (yaw, pitch) = quaternion_to_yaw_pitch(&q);
        assert!(yaw.abs() < PI + 1.0, "yaw should be in reasonable range");
        assert!(
            pitch.abs() < PI / 2.0 + 0.1,
            "pitch should be in reasonable range"
        );
    }

    #[test]
    fn test_quaternion_from_yaw_pitch_4d_identity() {
        let q = quaternion_from_yaw_pitch_4d(0.0, 0.0);
        let x_axis = q * Vector3::new(1.0, 0.0, 0.0);
        assert_approx_eq(x_axis.x, 1.0, 1e-6);
        assert_approx_eq(x_axis.y, 0.0, 1e-6);
        assert_approx_eq(x_axis.z, 0.0, 1e-6);
    }

    #[test]
    fn test_quaternion_to_yaw_pitch_4d_reasonable() {
        let q = quaternion_from_yaw_pitch_4d(PI / 4.0, PI / 6.0);
        let (yaw, pitch) = quaternion_to_yaw_pitch_4d(&q);
        assert!(yaw.abs() < PI + 1.0, "yaw should be in reasonable range");
        assert!(
            pitch.abs() < PI / 2.0 + 0.1,
            "pitch should be in reasonable range"
        );
    }

    #[test]
    fn test_rotation4d_q_left_set_get_consistent() {
        let mut rot = Rotation4D::identity();

        rot.set_q_left_from_yaw_pitch(PI / 4.0, PI / 6.0);

        let forward = rot.q_left() * Vector3::new(0.0, 0.0, 1.0);
        assert!(forward.x.abs() > 0.1, "yaw should rotate forward");
        assert!(forward.y.abs() > 0.1, "pitch should rotate forward");
    }

    #[test]
    fn test_rotation4d_q_right_set_get_consistent() {
        let mut rot = Rotation4D::identity();

        rot.set_q_right_from_yaw_pitch(PI / 4.0, PI / 6.0);

        let q_right = rot.q_right();
        let x_axis = q_right * Vector3::new(1.0, 0.0, 0.0);
        assert!(x_axis.x.abs() < 0.99, "q_right should rotate x_axis");
    }

    #[test]
    fn test_rotation4d_preserves_other_quaternion() {
        let mut rot = Rotation4D::identity();

        rot.set_q_left_from_yaw_pitch(PI / 4.0, PI / 6.0);
        let q_right_before = *rot.q_right();

        rot.set_q_right_from_yaw_pitch(PI / 3.0, PI / 8.0);

        assert!(*rot.q_right() != q_right_before, "q_right should change");

        let (yaw_l, pitch_l) = rot.get_q_left_as_yaw_pitch();
        assert!(yaw_l.abs() > 0.1, "yaw_l should be nonzero");
        assert!(pitch_l.abs() > 0.1, "pitch_l should be nonzero");
    }
}
