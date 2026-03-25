//! 4D Geometry and mathematical operations

use nalgebra::Vector4;

pub use crate::polytopes::Vertex4D;

/// Applies SO(4) rotations to a 4D point
#[allow(clippy::too_many_arguments)]
pub fn apply_so4_rotation(
    pos: [f32; 4],
    sin_xy: f32,
    cos_xy: f32,
    sin_xz: f32,
    cos_xz: f32,
    sin_yz: f32,
    cos_yz: f32,
    sin_xw: f32,
    cos_xw: f32,
    sin_yw: f32,
    cos_yw: f32,
    sin_zw: f32,
    cos_zw: f32,
) -> Vector4<f32> {
    let mut p = Vector4::new(pos[0], pos[1], pos[2], pos[3]);

    // Apply rotations in sequence for simplicity
    // XY rotation
    let x = p.x;
    let y = p.y;
    p.x = x * cos_xy - y * sin_xy;
    p.y = x * sin_xy + y * cos_xy;

    // XZ rotation
    let x = p.x;
    let z = p.z;
    p.x = x * cos_xz - z * sin_xz;
    p.z = x * sin_xz + z * cos_xz;

    // YZ rotation
    let y = p.y;
    let z = p.z;
    p.y = y * cos_yz - z * sin_yz;
    p.z = y * sin_yz + z * cos_yz;

    // XW rotation
    let x = p.x;
    let w = p.w;
    p.x = x * cos_xw - w * sin_xw;
    p.w = x * sin_xw + w * cos_xw;

    // YW rotation
    let y = p.y;
    let w = p.w;
    p.y = y * cos_yw - w * sin_yw;
    p.w = y * sin_yw + w * cos_yw;

    // ZW rotation
    let z = p.z;
    let w = p.w;
    p.z = z * cos_zw - w * sin_zw;
    p.w = z * sin_zw + w * cos_zw;

    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_identity_rotation() {
        let pos = [1.0, 2.0, 3.0, 4.0];
        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!(
            (result.x - pos[0]).abs() < 1e-6,
            "X should remain unchanged"
        );
        assert!(
            (result.y - pos[1]).abs() < 1e-6,
            "Y should remain unchanged"
        );
        assert!(
            (result.z - pos[2]).abs() < 1e-6,
            "Z should remain unchanged"
        );
        assert!(
            (result.w - pos[3]).abs() < 1e-6,
            "W should remain unchanged"
        );
    }

    #[test]
    fn test_xy_rotation_90_degrees() {
        let pos = [1.0, 0.0, 0.0, 0.0];
        let result = apply_so4_rotation(
            pos, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - 0.0).abs() < 1e-6, "X should be 0");
        assert!((result.y - 1.0).abs() < 1e-6, "Y should be 1");
        assert!((result.z - 0.0).abs() < 1e-6, "Z should remain 0");
        assert!((result.w - 0.0).abs() < 1e-6, "W should remain 0");
    }

    #[test]
    fn test_xy_rotation_180_degrees() {
        let pos = [1.0, 2.0, 0.0, 0.0];
        let result = apply_so4_rotation(
            pos, 0.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - (-1.0)).abs() < 1e-6, "X should be negated");
        assert!((result.y - (-2.0)).abs() < 1e-6, "Y should be negated");
    }

    #[test]
    fn test_xz_rotation() {
        let pos = [0.0, 0.0, 1.0, 0.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, 0.0, 1.0, sqrt2_2, sqrt2_2, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - (-sqrt2_2)).abs() < 1e-6);
        assert!((result.z - sqrt2_2).abs() < 1e-6);
    }

    #[test]
    fn test_yz_rotation() {
        let pos = [0.0, 1.0, 0.0, 0.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, sqrt2_2, sqrt2_2, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.y - sqrt2_2).abs() < 1e-6);
        assert!((result.z - sqrt2_2).abs() < 1e-6);
    }

    #[test]
    fn test_xw_rotation() {
        let pos = [0.0, 0.0, 0.0, 1.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, sqrt2_2, sqrt2_2, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - (-sqrt2_2)).abs() < 1e-6);
        assert!((result.w - sqrt2_2).abs() < 1e-6);
    }

    #[test]
    fn test_yw_rotation() {
        let pos = [0.0, 0.0, 0.0, 1.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, sqrt2_2, sqrt2_2, 0.0, 1.0,
        );

        assert!((result.y - (-sqrt2_2)).abs() < 1e-6);
        assert!((result.w - sqrt2_2).abs() < 1e-6);
    }

    #[test]
    fn test_zw_rotation() {
        let pos = [0.0, 0.0, 0.0, 1.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, sqrt2_2, sqrt2_2,
        );

        assert!((result.z - (-sqrt2_2)).abs() < 1e-6);
        assert!((result.w - sqrt2_2).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_preserves_distance() {
        let pos = [1.0f32, 2.0, 3.0, 4.0];
        let original_length =
            (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2] + pos[3] * pos[3]).sqrt();

        let result = apply_so4_rotation(
            pos,
            0.5,
            (1.0f32 - 0.5 * 0.5).sqrt(),
            0.3,
            (1.0f32 - 0.3 * 0.3).sqrt(),
            0.7,
            (1.0f32 - 0.7 * 0.7).sqrt(),
            0.2,
            (1.0f32 - 0.2 * 0.2).sqrt(),
            0.4,
            (1.0f32 - 0.4 * 0.4).sqrt(),
            0.6,
            (1.0f32 - 0.6 * 0.6).sqrt(),
        );

        let new_length =
            (result.x * result.x + result.y * result.y + result.z * result.z + result.w * result.w)
                .sqrt();

        assert!(
            (original_length - new_length).abs() < 1e-5,
            "Rotation should preserve distance"
        );
    }

    #[test]
    fn test_rotation_normalized_sin_cos() {
        let pos = [1.0, 0.0, 0.0, 0.0];
        let angle = PI / 8.0;
        let sin = angle.sin();
        let cos = angle.cos();

        let result = apply_so4_rotation(
            pos, sin, cos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        let length =
            (result.x * result.x + result.y * result.y + result.z * result.z + result.w * result.w)
                .sqrt();
        let original_length = 1.0_f32;

        assert!(
            (length - original_length).abs() < 1e-6,
            "Length should be preserved with normalized rotations"
        );

        assert!((result.x - cos).abs() < 1e-6);
        assert!((result.y - sin).abs() < 1e-6);
    }

    #[test]
    fn test_zero_rotation() {
        let pos = [5.0, -3.0, 2.0, -1.0];
        let result = apply_so4_rotation(
            pos, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - pos[0]).abs() < 1e-6);
        assert!((result.y - pos[1]).abs() < 1e-6);
        assert!((result.z - pos[2]).abs() < 1e-6);
        assert!((result.w - pos[3]).abs() < 1e-6);
    }

    #[test]
    fn test_multiple_rotations_composition() {
        let pos = [1.0, 0.0, 0.0, 0.0];
        let sqrt2_2 = (2.0_f32).sqrt() / 2.0;

        let result = apply_so4_rotation(
            pos, sqrt2_2, sqrt2_2, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        );

        assert!((result.x - sqrt2_2).abs() < 1e-6);
        assert!((result.y - sqrt2_2).abs() < 1e-6);
        assert!((result.z - 0.0).abs() < 1e-6);

        let result2 = apply_so4_rotation(
            [result.x, result.y, result.z, result.w],
            0.0,
            1.0,
            0.0,
            1.0,
            sqrt2_2,
            sqrt2_2,
            0.0,
            1.0,
            0.0,
            1.0,
            0.0,
            1.0,
        );

        assert!((result2.x - sqrt2_2).abs() < 1e-6);
        let half = sqrt2_2 * sqrt2_2;
        assert!((result2.y - half).abs() < 1e-6);
        assert!((result2.z - half).abs() < 1e-6);
    }

    #[test]
    fn test_rotation_preserves_origin() {
        let pos = [0.0f32, 0.0, 0.0, 0.0];
        let result = apply_so4_rotation(
            pos,
            0.5,
            (1.0f32 - 0.5 * 0.5).sqrt(),
            0.3,
            (1.0f32 - 0.3 * 0.3).sqrt(),
            0.7,
            (1.0f32 - 0.7 * 0.7).sqrt(),
            0.2,
            (1.0f32 - 0.2 * 0.2).sqrt(),
            0.4,
            (1.0f32 - 0.4 * 0.4).sqrt(),
            0.6,
            (1.0f32 - 0.6 * 0.6).sqrt(),
        );

        assert!((result.x - 0.0).abs() < 1e-6);
        assert!((result.y - 0.0).abs() < 1e-6);
        assert!((result.z - 0.0).abs() < 1e-6);
        assert!((result.w - 0.0).abs() < 1e-6);
    }
}
