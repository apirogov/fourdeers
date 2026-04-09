use nalgebra::{Vector3, Vector4};

use crate::camera::Camera;

pub(crate) struct MapViewTransform {
    mat_4d: nalgebra::Matrix4<f32>,
    offset_4d: Vector4<f32>,
    mat_3d: nalgebra::Rotation3<f32>,
}

impl MapViewTransform {
    pub(crate) fn new(map_camera: &Camera) -> Self {
        let map_inv = map_camera.rotation_4d.inverse_q_right_only();
        let mat_4d = map_inv.to_matrix();
        let offset_4d = map_inv.rotate_vector(map_camera.position);
        let mat_3d = map_camera
            .rotation_4d
            .q_left()
            .inverse()
            .to_rotation_matrix();
        Self {
            mat_4d,
            offset_4d,
            mat_3d,
        }
    }

    pub(crate) fn project_to_3d(&self, pos_4d: Vector4<f32>) -> Vector3<f32> {
        let r = self.mat_4d * pos_4d - self.offset_4d;
        self.mat_3d * Vector3::new(r.x, r.y, r.z)
    }

    /// Transform a **direction vector** (not a position) from tesseract 4D space to map 3D space.
    ///
    /// Unlike `project_to_3d`, this does NOT subtract the camera offset. This is essential for
    /// transforming frustum edge directions: subtracting the offset would corrupt direction-only
    /// transforms, since directions are not rooted at any position.
    ///
    /// The rotation matrices (`mat_4d` and `mat_3d`) are still applied — only the translation
    /// component is omitted.
    pub(crate) fn direction_to_3d(&self, dir_4d: Vector4<f32>) -> Vector3<f32> {
        let r = self.mat_4d * dir_4d;
        self.mat_3d * Vector3::new(r.x, r.y, r.z)
    }
}

#[cfg(test)]
mod tests {
    use crate::camera::Camera;
    use crate::geometry::Bounds4D;
    use crate::test_utils::assert_approx_eq;
    use crate::toy::CompassWaypoint;
    use nalgebra::Vector4;

    use super::MapViewTransform;
    use crate::map::bounds::{compute_bounds, direction_to_tesseract, normalize_to_tesseract};

    #[test]
    fn test_direction_to_3d_no_offset() {
        let mut cam = Camera::new();
        cam.position = Vector4::zeros();
        let mt = MapViewTransform::new(&cam);
        let dir = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let pos_result = mt.project_to_3d(dir);
        let dir_result = mt.direction_to_3d(dir);
        assert_approx_eq(pos_result.x, dir_result.x, 1e-6);
        assert_approx_eq(pos_result.y, dir_result.y, 1e-6);
        assert_approx_eq(pos_result.z, dir_result.z, 1e-6);
    }

    #[test]
    fn test_forward_direction_points_at_origin_while_orbiting() {
        let geometry_bounds = Some(Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ));
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);

        let orbit_radius = 5.0f32;
        let steps = 12;
        for step in 0..steps {
            let angle = 2.0 * std::f32::consts::PI * step as f32 / steps as f32;
            let cam_x = orbit_radius * angle.sin();
            let cam_z = -orbit_radius * angle.cos();

            let mut scene_camera = Camera::new();
            scene_camera.position = Vector4::new(cam_x, 0.0, cam_z, 0.0);
            let yaw_to_origin = (-cam_x).atan2(-cam_z);
            scene_camera.set_yaw_pitch_l(yaw_to_origin, 0.0);

            let waypoints: Vec<CompassWaypoint> = vec![];
            let bounds = compute_bounds(&scene_camera, &waypoints, geometry_bounds);

            let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
            let cam_3d = map_transform.project_to_3d(norm_cam);

            let norm_origin = normalize_to_tesseract(Vector4::zeros(), &bounds);
            let origin_3d = map_transform.project_to_3d(norm_origin);

            let to_origin = origin_3d - cam_3d;
            let to_origin_len = to_origin.norm();
            if to_origin_len < 1e-6 {
                continue;
            }
            let to_origin_dir = to_origin / to_origin_len;

            let forward_4d =
                scene_camera.project_camera_3d_to_world_4d(scene_camera.forward_vector());
            let forward_tess = direction_to_tesseract(forward_4d, &bounds);
            let forward_3d = map_transform.direction_to_3d(forward_tess);
            let forward_len = forward_3d.norm();
            assert!(
                forward_len > 1e-10,
                "forward direction should be non-zero at step {}",
                step
            );
            let forward_dir = forward_3d / forward_len;

            let dot = to_origin_dir.dot(&forward_dir);
            assert!(
                dot > 0.99,
                "forward arrow should point at origin at angle {:.1}° (step {}): \
                 dot={:.6}, to_origin_dir={:?}, forward_dir={:?}, cam=({:.2},{:.2}), \
                 cam_3d={:?}, origin_3d={:?}, bounds_z=[{:.2},{:.2}]",
                angle.to_degrees(),
                step,
                dot,
                to_origin_dir,
                forward_dir,
                cam_x,
                cam_z,
                cam_3d,
                origin_3d,
                bounds.min[2],
                bounds.max[2],
            );
        }
    }
}
