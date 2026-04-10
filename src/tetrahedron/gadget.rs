use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::input::Zone;

use super::geometry::{
    component_weights, compute_weighted_direction_3d, magnitude_4d, tetrahedron_vertices,
};
use super::types::{TetrahedronEdge, TetrahedronGadget, TetrahedronVertex, VectorArrow};

impl TetrahedronGadget {
    #[must_use]
    pub fn from_4d_vector(vector_4d: Vector4<f32>) -> Self {
        Self::from_4d_vector_with_quaternion(vector_4d, UnitQuaternion::identity(), 1.0)
    }

    #[must_use]
    pub fn from_4d_vector_with_scale(vector_4d: Vector4<f32>, scale: f32) -> Self {
        Self::from_4d_vector_with_quaternion(vector_4d, UnitQuaternion::identity(), scale)
    }

    #[must_use]
    pub fn from_4d_vector_with_quaternion(
        vector_4d: Vector4<f32>,
        rotation: UnitQuaternion<f32>,
        scale: f32,
    ) -> Self {
        let center = Vector3::zeros();
        let vertices = Self::compute_vertices_with_rotation(scale, &rotation);
        let edges = Self::compute_edges();
        let vector_arrow = Self::compute_vector_arrow(vector_4d, scale, &rotation);

        let component_values = [vector_4d.x, vector_4d.y, vector_4d.z, vector_4d.w];
        let vector_magnitude = magnitude_4d(vector_4d);

        Self {
            vertices,
            edges,
            vector_arrow,
            center,
            scale,
            tip_label: None,
            base_label: None,
            component_values,
            vector_magnitude,
        }
    }

    #[must_use]
    pub fn with_tip_label(mut self, label: impl Into<String>) -> Self {
        self.tip_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn with_base_label(mut self, label: impl Into<String>) -> Self {
        self.base_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn for_zone(
        vector_4d: Vector4<f32>,
        zone: Zone,
        user_rotation: UnitQuaternion<f32>,
        scale: f32,
    ) -> Self {
        let base_rotation = Self::compute_base_rotation_for_zone(&vector_4d, zone);
        let total_rotation = user_rotation * base_rotation;
        Self::from_4d_vector_with_quaternion(vector_4d, total_rotation, scale)
    }

    fn compute_base_rotation_for_zone(vector_4d: &Vector4<f32>, zone: Zone) -> UnitQuaternion<f32> {
        let arrow_dir = compute_weighted_direction_3d(*vector_4d);
        let target = match zone {
            Zone::North | Zone::Center => Vector3::new(0.0, 1.0, 0.0),
            Zone::South => Vector3::new(0.0, -1.0, 0.0),
            Zone::East => Vector3::new(1.0, 0.0, 0.0),
            Zone::West => Vector3::new(-1.0, 0.0, 0.0),
            Zone::NorthEast => Vector3::new(1.0, 1.0, 0.0).normalize(),
            Zone::NorthWest => Vector3::new(-1.0, 1.0, 0.0).normalize(),
            Zone::SouthEast => Vector3::new(1.0, -1.0, 0.0).normalize(),
            Zone::SouthWest => Vector3::new(-1.0, -1.0, 0.0).normalize(),
        };

        let current = arrow_dir;
        let current_len = current.magnitude();
        if current_len < 1e-6 {
            return UnitQuaternion::identity();
        }

        let current_normalized = current / current_len;
        let dot = current_normalized.dot(&target);

        if dot > 0.9999 {
            return UnitQuaternion::identity();
        }
        if dot < -0.9999 {
            let perp = if current_normalized.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            let axis = current_normalized.cross(&perp).normalize();
            return UnitQuaternion::from_axis_angle(
                &nalgebra::Unit::new_normalize(axis),
                std::f32::consts::PI,
            );
        }

        let axis = current_normalized.cross(&target);
        let axis_len = axis.magnitude();
        if axis_len < 1e-6 {
            return UnitQuaternion::identity();
        }

        let axis_normalized = nalgebra::Unit::new_normalize(axis);
        let angle = dot.clamp(-1.0, 1.0).acos();
        UnitQuaternion::from_axis_angle(&axis_normalized, angle)
    }

    fn compute_vertices_with_rotation(
        scale: f32,
        rotation: &UnitQuaternion<f32>,
    ) -> [TetrahedronVertex; 4] {
        let base_positions = tetrahedron_vertices(scale);
        let labels = ["X", "Y", "Z", "W"];
        let axes = ['X', 'Y', 'Z', 'W'];

        std::array::from_fn(|i| {
            let pos = base_positions[i];
            let rotated_pos = rotation.transform_vector(&pos);
            let normal = rotation.transform_vector(&pos.normalize());
            TetrahedronVertex {
                position: rotated_pos,
                normal,
                label: labels[i].to_string(),
                axis_4d: axes[i],
            }
        })
    }

    fn compute_edges() -> [TetrahedronEdge; 6] {
        [
            TetrahedronEdge {
                vertex_indices: [0, 1],
            },
            TetrahedronEdge {
                vertex_indices: [0, 2],
            },
            TetrahedronEdge {
                vertex_indices: [0, 3],
            },
            TetrahedronEdge {
                vertex_indices: [1, 2],
            },
            TetrahedronEdge {
                vertex_indices: [1, 3],
            },
            TetrahedronEdge {
                vertex_indices: [2, 3],
            },
        ]
    }

    fn compute_vector_arrow(
        vector_4d: Vector4<f32>,
        scale: f32,
        rotation: &UnitQuaternion<f32>,
    ) -> VectorArrow {
        let arrow_head_size = scale * 0.15;

        let Some(weights) = component_weights(vector_4d) else {
            return VectorArrow {
                end_position: Vector3::zeros(),
                arrow_head_size: 0.0,
            };
        };

        let abs_sum: f32 = weights.iter().sum();

        if abs_sum < 1e-6 {
            return VectorArrow {
                end_position: Vector3::zeros(),
                arrow_head_size,
            };
        }

        let base_vertices = tetrahedron_vertices(scale);

        let mut end = Vector3::zeros();
        for (i, &weight) in weights.iter().enumerate() {
            let rotated = rotation.transform_vector(&base_vertices[i]);
            end += rotated * weight;
        }

        VectorArrow {
            end_position: end,
            arrow_head_size,
        }
    }

    #[must_use]
    pub fn vertex_3d(&self, vertex_index: usize) -> Option<&Vector3<f32>> {
        self.vertices.get(vertex_index).map(|v| &v.position)
    }

    #[must_use]
    pub fn vertex_normal(&self, vertex_index: usize) -> Option<&Vector3<f32>> {
        self.vertices.get(vertex_index).map(|v| &v.normal)
    }

    #[must_use]
    pub fn arrow_position(&self) -> &Vector3<f32> {
        &self.vector_arrow.end_position
    }

    #[must_use]
    pub const fn arrow_head_size(&self) -> f32 {
        self.vector_arrow.arrow_head_size
    }

    #[must_use]
    pub const fn base_label(&self) -> Option<&String> {
        self.base_label.as_ref()
    }
}
