use nalgebra::{Matrix4, Rotation3, Vector3, Vector4};

use crate::camera::Camera;

#[derive(Clone)]
pub(crate) struct CameraProjection {
    mat_4d: Matrix4<f32>,
    offset_4d: Vector4<f32>,
    mat_3d: Rotation3<f32>,
}

impl CameraProjection {
    pub(crate) fn new(camera: &Camera) -> Self {
        let inv = camera.rotation_4d().inverse_q_right_only();
        let rotated: Vector4<f32> = inv.rotate_vector(camera.position);
        CameraProjection {
            mat_4d: inv.to_matrix(),
            offset_4d: rotated,
            mat_3d: camera.rotation_4d().q_left().inverse().to_rotation_matrix(),
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
