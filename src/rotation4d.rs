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

    pub fn from_left_right(q_left: UnitQuaternion<f32>, q_right: UnitQuaternion<f32>) -> Self {
        Self { q_left, q_right }
    }

    pub fn q_left(&self) -> &UnitQuaternion<f32> {
        &self.q_left
    }

    pub fn q_right(&self) -> &UnitQuaternion<f32> {
        &self.q_right
    }

    pub fn set_q_left_from_yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch);
        self.q_left = yaw_rot * pitch_rot;
    }

    pub fn get_q_left_as_yaw_pitch(&self) -> (f32, f32) {
        let forward = self.q_left * Vector3::new(0.0, 0.0, 1.0);
        let yaw = forward.x.atan2(forward.z);
        let horizontal_len = (forward.x * forward.x + forward.z * forward.z).sqrt();
        let pitch = forward.y.atan2(horizontal_len);
        (yaw, pitch)
    }

    pub fn inverse(&self) -> Self {
        Self {
            q_left: self.q_left.inverse(),
            q_right: self.q_right.inverse(),
        }
    }

    pub fn inverse_q_right_only(&self) -> Self {
        Self {
            q_left: UnitQuaternion::identity(),
            q_right: self.q_right.inverse(),
        }
    }

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

    pub fn tilt_slice_around_local_axis(
        _forward: [f32; 4],
        _right: [f32; 4],
        _up: [f32; 4],
        delta_x: f32,
        delta_y: f32,
    ) -> Self {
        let tilt_zw = Self::from_plane_angle(RotationPlane::ZW, delta_x * 0.005);
        let tilt_yw = Self::from_plane_angle(RotationPlane::YW, delta_y * 0.005);
        tilt_zw.then(&tilt_yw)
    }

    pub fn to_3d_rotation(&self) -> UnitQuaternion<f32> {
        let q_avg = self.q_left * self.q_right;
        UnitQuaternion::from_quaternion(q_avg.into_inner())
    }

    pub fn from_3d_rotation(q: &UnitQuaternion<f32>) -> Self {
        Self {
            q_left: *q,
            q_right: UnitQuaternion::identity(),
        }
    }

    pub fn as_3d_rotation_compatible(&self) -> UnitQuaternion<f32> {
        let basis = self.basis_vectors();
        let x = Vector3::new(basis[0][0], basis[0][1], basis[0][2]);
        let y = Vector3::new(basis[1][0], basis[1][1], basis[1][2]);
        let z = Vector3::new(basis[2][0], basis[2][1], basis[2][2]);

        let mut mat = nalgebra::Matrix3::identity();
        mat.set_column(0, &x);
        mat.set_column(1, &y);
        mat.set_column(2, &z);

        let rotation = nalgebra::Rotation3::from_matrix(&mat);
        UnitQuaternion::from_rotation_matrix(&rotation)
    }

    pub fn get_w_component_of_basis(&self) -> [f32; 4] {
        let basis = self.basis_vectors();
        [basis[0][3], basis[1][3], basis[2][3], basis[3][3]]
    }

    pub fn is_pure_3d(&self) -> bool {
        let w_components = self.get_w_component_of_basis();
        w_components[0].abs() < 1e-6
            && w_components[1].abs() < 1e-6
            && w_components[2].abs() < 1e-6
            && (w_components[3] - 1.0).abs() < 1e-6
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
    use std::f32::consts::PI;

    fn assert_approx_eq(a: f32, b: f32, epsilon: f32) {
        assert!((a - b).abs() < epsilon, "Expected {:.6}, got {:.6}", b, a);
    }

    fn assert_vec_approx_eq(a: [f32; 4], b: [f32; 4], epsilon: f32) {
        for i in 0..4 {
            assert_approx_eq(a[i], b[i], epsilon);
        }
    }

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
