use nalgebra::{Matrix4, Rotation3, Vector3, Vector4};

use crate::camera::Camera;
use crate::rotation4d::Rotation4D;

pub(crate) struct CameraProjection {
    mat_4d: Matrix4<f32>,
    offset_4d: Vector4<f32>,
    mat_3d: Rotation3<f32>,
}

impl CameraProjection {
    pub(crate) fn new(camera: &Camera) -> Self {
        let inv = camera.rotation_4d.inverse_q_right_only();
        Self {
            mat_4d: inv.to_matrix(),
            offset_4d: inv.rotate_vector(camera.position),
            mat_3d: camera.rotation_4d.q_left().inverse().to_rotation_matrix(),
        }
    }

    pub(crate) fn with_object_rotation(camera: &Camera, object_rotation: &Rotation4D) -> Self {
        let camera_inv = camera.rotation_4d.inverse_q_right_only();
        let combined = object_rotation.then(&camera_inv);
        Self {
            mat_4d: combined.to_matrix(),
            offset_4d: camera_inv.rotate_vector(camera.position),
            mat_3d: camera.rotation_4d.q_left().inverse().to_rotation_matrix(),
        }
    }

    pub(crate) fn project(&self, pos_4d: Vector4<f32>) -> (Vector3<f32>, f32) {
        let r = self.mat_4d * pos_4d - self.offset_4d;
        let xyz = self.mat_3d * Vector3::new(r.x, r.y, r.z);
        (xyz, r.w)
    }

    pub(crate) fn project_direction(&self, dir_4d: Vector4<f32>) -> Vector3<f32> {
        let r = self.mat_4d * dir_4d;
        self.mat_3d * Vector3::new(r.x, r.y, r.z)
    }
}
