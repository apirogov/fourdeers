use nalgebra::Vector4;

use crate::camera::Camera;
use crate::geometry::Bounds4D;
use crate::toy::CompassWaypoint;

use super::BOUNDS_PADDING_FACTOR;

#[must_use]
pub fn compute_bounds(
    scene_camera: &Camera,
    waypoints: &[CompassWaypoint],
    geometry_bounds: Option<Bounds4D>,
) -> Bounds4D {
    let mut bounds = Bounds4D::from_point(scene_camera.position);
    for wp in waypoints {
        bounds = bounds.expanded_to(wp.position);
    }
    if let Some(geo) = geometry_bounds {
        bounds = bounds.expanded_to(geo.min).expanded_to(geo.max);
    }
    bounds.padded(BOUNDS_PADDING_FACTOR)
}

#[must_use]
pub fn normalize_to_tesseract(pos: Vector4<f32>, bounds: &Bounds4D) -> Vector4<f32> {
    let mut result = Vector4::zeros();
    for i in 0..4 {
        let range = bounds.range(i);
        if range.abs() < 1e-6 {
            result[i] = 0.0;
        } else {
            result[i] = 2.0 * (pos[i] - bounds.min[i]) / range - 1.0;
        }
    }
    result
}

pub(super) fn direction_to_tesseract(dir_world: Vector4<f32>, bounds: &Bounds4D) -> Vector4<f32> {
    let mut result = Vector4::zeros();
    for i in 0..4 {
        let range = bounds.range(i);
        if range.abs() < 1e-6 {
            result[i] = dir_world[i];
        } else {
            result[i] = dir_world[i] * 2.0 / range;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;

    #[test]
    fn test_normalize_to_tesseract_center() {
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let center = normalize_to_tesseract(Vector4::new(0.0, 0.0, 0.0, 0.0), &bounds);
        for i in 0..4 {
            assert_approx_eq(center[i], 0.0, 1e-6);
        }
    }

    #[test]
    fn test_normalize_to_tesseract_corners() {
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let min_corner = normalize_to_tesseract(bounds.min, &bounds);
        let max_corner = normalize_to_tesseract(bounds.max, &bounds);
        for i in 0..4 {
            assert_approx_eq(min_corner[i], -1.0, 1e-6);
            assert_approx_eq(max_corner[i], 1.0, 1e-6);
        }
    }

    #[test]
    fn test_normalize_to_tesseract_asymmetric_bounds() {
        let bounds = Bounds4D::from_corners(
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(10.0, 10.0, 10.0, 10.0),
        );
        let result = normalize_to_tesseract(Vector4::new(5.0, 0.0, 10.0, 2.5), &bounds);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], -1.0, 1e-6);
        assert_approx_eq(result[2], 1.0, 1e-6);
        assert_approx_eq(result[3], -0.5, 1e-6);
    }

    #[test]
    fn test_direction_to_tesseract_identity_bounds() {
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let dir = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let result = direction_to_tesseract(dir, &bounds);
        assert_approx_eq(result[0], 1.0, 1e-6);
        assert_approx_eq(result[1], 2.0, 1e-6);
        assert_approx_eq(result[2], 3.0, 1e-6);
        assert_approx_eq(result[3], 4.0, 1e-6);
    }

    #[test]
    fn test_direction_to_tesseract_scaled() {
        let bounds = Bounds4D::from_corners(
            Vector4::new(-2.0, -2.0, -2.0, -2.0),
            Vector4::new(2.0, 2.0, 2.0, 2.0),
        );
        let dir = Vector4::new(1.0, 1.0, 1.0, 1.0);
        let result = direction_to_tesseract(dir, &bounds);
        assert_approx_eq(result[0], 0.5, 1e-6);
        assert_approx_eq(result[1], 0.5, 1e-6);
        assert_approx_eq(result[2], 0.5, 1e-6);
        assert_approx_eq(result[3], 0.5, 1e-6);
    }

    #[test]
    fn test_compute_bounds_includes_geometry() {
        let mut camera = Camera::new();
        camera.position = Vector4::new(5.0, 5.0, 5.0, 5.0);
        let waypoints: Vec<CompassWaypoint> = vec![];
        let bounds = compute_bounds(&camera, &waypoints, None);
        assert!(
            bounds.min[0] > 0.0,
            "without geometry, bounds should be near camera"
        );
        assert!(
            bounds.max[0] > 0.0,
            "without geometry, bounds should be near camera"
        );

        let geometry_bounds = Some(Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ));
        let bounds_g = compute_bounds(&camera, &waypoints, geometry_bounds);
        assert!(
            bounds_g.min[0] < bounds.min[0],
            "with geometry, min should extend to include geometry"
        );
        assert!(bounds_g.min[0] <= -1.0, "geometry min should be included");
        assert!(bounds_g.max[0] >= 5.0, "camera should still be included");
    }
}
