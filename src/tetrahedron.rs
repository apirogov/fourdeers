//! Tetrahedron visualization for 4D direction vectors
//!
//! A tetrahedron is used to visualize 4D unit vectors in 3D space.
//! The 4 vertices of the tetrahedron represent the positive X, Y, Z, W directions in 4D.
//! A vector arrow points from the center toward the relatively weighted direction.

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::input::Zone;

/// Square root of 3 - used to convert between scale and vertex coordinates
const SQRT_3: f32 = 1.732_050_8;

#[must_use]
pub fn magnitude_4d(v: Vector4<f32>) -> f32 {
    (v.x.powi(2) + v.y.powi(2) + v.z.powi(2) + v.w.powi(2)).sqrt()
}

const TETRAHEDRON_BASE_VERTICES: [[f32; 3]; 4] = [
    [1.0, 1.0, 1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [1.0, -1.0, -1.0],
];

fn tetrahedron_vertices(scale: f32) -> [Vector3<f32>; 4] {
    let s = scale / SQRT_3;
    TETRAHEDRON_BASE_VERTICES.map(|[x, y, z]| Vector3::new(x * s, y * s, z * s))
}

/// Layout parameters for tetrahedron rendering, proportional to view size
#[derive(Debug, Clone, Copy)]
pub struct TetrahedronLayout {
    pub scale: f32,
    pub edge_offset: f32,
}

/// Compute tetrahedron layout based on view rect dimensions
#[must_use]
pub fn tetrahedron_layout(view_rect: egui::Rect) -> TetrahedronLayout {
    let longer_side = view_rect.width().max(view_rect.height());
    TetrahedronLayout {
        scale: longer_side * 0.05,
        edge_offset: longer_side * 0.07,
    }
}

/// A vertex of the tetrahedron with label information
#[derive(Debug, Clone)]
pub struct TetrahedronVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub label: String,
    pub axis_4d: char,
}

/// An edge connecting two vertices
#[derive(Debug, Clone, Copy)]
pub struct TetrahedronEdge {
    pub vertex_indices: [usize; 2],
}

/// Vector arrow from center pointing in weighted direction
#[derive(Debug, Clone)]
pub struct VectorArrow {
    pub end_position: Vector3<f32>,
    pub arrow_head_size: f32,
}

/// Complete tetrahedron visualization data
#[derive(Debug, Clone)]
pub struct TetrahedronGadget {
    pub vertices: [TetrahedronVertex; 4],
    pub edges: [TetrahedronEdge; 6],
    pub vector_arrow: VectorArrow,
    pub center: Vector3<f32>,
    pub scale: f32,
    pub tip_label: Option<String>,
    pub base_label: Option<String>,
    pub component_values: [f32; 4],
    pub vector_magnitude: f32,
}

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
    pub const fn tip_label(&self) -> Option<&String> {
        self.tip_label.as_ref()
    }

    #[must_use]
    pub const fn base_label(&self) -> Option<&String> {
        self.base_label.as_ref()
    }
}

fn component_weights(vector_4d: Vector4<f32>) -> Option<[f32; 4]> {
    let norm = magnitude_4d(vector_4d);
    if norm < 1e-6 {
        return None;
    }
    let normalized = vector_4d / norm;
    Some([
        normalized.x.abs(),
        normalized.y.abs(),
        normalized.z.abs(),
        normalized.w.abs(),
    ])
}

fn compute_weighted_direction_3d(vector_4d: Vector4<f32>) -> Vector3<f32> {
    let Some(weights) = component_weights(vector_4d) else {
        return Vector3::zeros();
    };

    let mut result = Vector3::zeros();
    for (i, &weight) in weights.iter().enumerate() {
        let [x, y, z] = TETRAHEDRON_BASE_VERTICES[i];
        result += Vector3::new(x, y, z) * weight;
    }

    result
}

#[cfg(test)]
#[must_use]
fn normalize_4d_vector(v: Vector4<f32>) -> Vector4<f32> {
    let norm = magnitude_4d(v);
    if norm < 1e-6 {
        Vector4::new(0.0, 0.0, 0.0, 0.0)
    } else {
        v / norm
    }
}

#[cfg(test)]
#[must_use]
fn compute_weighted_direction(vector_4d: Vector4<f32>) -> Vector3<f32> {
    compute_weighted_direction_3d(vector_4d) / SQRT_3
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ComponentColor {
    #[must_use]
    pub fn to_egui_color(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

#[must_use]
pub fn compute_component_color(component: f32, max_abs: f32) -> ComponentColor {
    const EPSILON: f32 = 1e-6;

    if max_abs < EPSILON || component.abs() < EPSILON {
        return ComponentColor {
            r: 255,
            g: 255,
            b: 255,
            a: 220,
        };
    }

    let relative_strength = (component.abs() / max_abs).min(1.0);

    if component > 0.0 {
        let intensity = relative_strength;
        let r = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.8));
        let g = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.5));
        let b = 255;
        ComponentColor { r, g, b, a: 230 }
    } else {
        let intensity = relative_strength;
        let r = 255;
        let g = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.6));
        let b = crate::colors::to_u8(255.0 * (1.0 - intensity * 0.6));
        ComponentColor { r, g, b, a: 230 }
    }
}

#[must_use]
pub fn format_component_value(value: f32) -> String {
    if value.abs() < 1e-6 {
        "0.00".to_string()
    } else if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

#[must_use]
pub fn format_magnitude(magnitude: f32) -> String {
    format_component_value(magnitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;

    #[test]
    fn test_tetrahedron_vertices_count() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(gadget.vertices.len(), 4);
    }

    #[test]
    fn test_tetrahedron_edges_count() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(gadget.edges.len(), 6);
    }

    #[test]
    fn test_tetrahedron_vertex_labels() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));

        let labels: Vec<&str> = gadget.vertices.iter().map(|v| v.label.as_str()).collect();
        assert!(labels.contains(&"X"));
        assert!(labels.contains(&"Y"));
        assert!(labels.contains(&"Z"));
        assert!(labels.contains(&"W"));
    }

    #[test]
    fn test_tetrahedron_vertex_axes() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));

        let axes: Vec<char> = gadget.vertices.iter().map(|v| v.axis_4d).collect();
        assert!(axes.contains(&'X'));
        assert!(axes.contains(&'Y'));
        assert!(axes.contains(&'Z'));
        assert!(axes.contains(&'W'));
    }

    #[test]
    fn test_tetrahedron_edge_connectivity() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));

        for edge in &gadget.edges {
            assert!(edge.vertex_indices[0] < 4);
            assert!(edge.vertex_indices[1] < 4);
            assert_ne!(edge.vertex_indices[0], edge.vertex_indices[1]);
        }
    }

    #[test]
    fn test_tetrahedron_edges_unique() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));

        let mut edges_seen = std::collections::HashSet::new();
        for edge in &gadget.edges {
            let mut sorted = edge.vertex_indices;
            sorted.sort();
            assert!(
                !edges_seen.contains(&sorted),
                "Duplicate edge found: {:?}",
                sorted
            );
            edges_seen.insert(sorted);
        }
    }

    #[test]
    fn test_vector_arrow_only_x() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // Arrow should point toward +X vertex (s, s, s) where s = 1/sqrt(3)
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_y() {
        let vector = Vector4::new(0.0, 1.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // Arrow should point toward +Y vertex (-s, -s, s)
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_z() {
        let vector = Vector4::new(0.0, 0.0, 1.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // Arrow should point toward +Z vertex (-s, s, -s)
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, -expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_w() {
        let vector = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // Arrow should point toward +W vertex (s, -s, -s)
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, -expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_xy_equal() {
        let vector = Vector4::new(1.0, 1.0, 0.0, 0.0).normalize();
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        assert_approx_eq(gadget.vector_arrow.end_position.x, 0.0, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.0, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.8165, 1e-2);
    }

    #[test]
    fn test_vector_arrow_xyz_equal() {
        let vector = Vector4::new(1.0, 1.0, 1.0, 0.0).normalize();
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        assert_approx_eq(gadget.vector_arrow.end_position.x, -0.333, 1e-2);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.333, 1e-2);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.333, 1e-2);
    }

    #[test]
    fn test_vector_arrow_xyzw_equal() {
        let vector = Vector4::new(1.0, 1.0, 1.0, 1.0).normalize();
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // All components equal - arrow should be at center
        assert_approx_eq(gadget.vector_arrow.end_position.x, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.0, 1e-5);
    }

    #[test]
    fn test_vector_arrow_zero_vector() {
        let vector = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        assert_approx_eq(gadget.vector_arrow.end_position.x, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.0, 1e-5);
    }

    #[test]
    fn test_vector_arrow_negative_x() {
        let vector = Vector4::new(-1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        // Negative X also points toward X vertex (abs value used for position)
        // The color indicates the sign, not the position
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    #[test]
    fn test_normalize_4d_vector() {
        let v = Vector4::new(3.0, 4.0, 0.0, 0.0);
        let normalized = normalize_4d_vector(v);

        let norm = (normalized.x.powi(2)
            + normalized.y.powi(2)
            + normalized.z.powi(2)
            + normalized.w.powi(2))
        .sqrt();
        assert_approx_eq(norm, 1.0, 1e-6);
        assert_approx_eq(normalized.x, 0.6, 1e-6);
        assert_approx_eq(normalized.y, 0.8, 1e-6);
    }

    #[test]
    fn test_normalize_4d_vector_zero() {
        let v = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let normalized = normalize_4d_vector(v);

        assert_eq!(normalized.x, 0.0);
        assert_eq!(normalized.y, 0.0);
        assert_eq!(normalized.z, 0.0);
        assert_eq!(normalized.w, 0.0);
    }

    #[test]
    fn test_compute_weighted_direction() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let direction = compute_weighted_direction(vector);

        let expected = 1.0 / SQRT_3;
        assert_approx_eq(direction.x, expected, 1e-3);
        assert_approx_eq(direction.y, expected, 1e-3);
        assert_approx_eq(direction.z, expected, 1e-3);
    }

    #[test]
    fn test_tetrahedron_with_scale() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let scale = 2.0;
        let gadget = TetrahedronGadget::from_4d_vector_with_scale(vector, scale);

        assert_approx_eq(gadget.scale, scale, 1e-5);
        // +X vertex should be at (s, s, s) where s = scale/sqrt(3)
        let s = scale / SQRT_3;
        assert_approx_eq(gadget.vertices[0].position.x, s, 1e-5);
        assert_approx_eq(gadget.vertices[0].position.y, s, 1e-5);
        assert_approx_eq(gadget.vertices[0].position.z, s, 1e-5);
    }

    #[test]
    fn test_vertex_3d() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        let Some(pos) = gadget.vertex_3d(0) else {
            panic!("Failed to get vertex 3d position");
        };

        let s = 1.0 / SQRT_3;
        assert_approx_eq(pos.x, s, 1e-5);
        assert_approx_eq(pos.y, s, 1e-5);
        assert_approx_eq(pos.z, s, 1e-5);
    }

    #[test]
    fn test_vertex_3d_invalid_index() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        let pos = gadget.vertex_3d(10);
        assert!(pos.is_none());
    }

    #[test]
    fn test_arrow_position() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        let arrow = gadget.arrow_position();
        let s = 1.0 / SQRT_3;
        assert_approx_eq(arrow.x, s, 1e-3);
        assert_approx_eq(arrow.y, s, 1e-3);
        assert_approx_eq(arrow.z, s, 1e-3);
    }

    #[test]
    fn test_vector_arrow_mixed_components() {
        let vector = Vector4::new(2.0, 1.0, 0.5, 0.0).normalize();
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        let arrow_len = (gadget.vector_arrow.end_position.x.powi(2)
            + gadget.vector_arrow.end_position.y.powi(2)
            + gadget.vector_arrow.end_position.z.powi(2))
        .sqrt();

        assert!(arrow_len > 0.0);
        assert!(
            gadget.vector_arrow.end_position.y.abs() > gadget.vector_arrow.end_position.x.abs()
        );
    }

    #[test]
    fn test_tetrahedron_center() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        assert_approx_eq(gadget.center.x, 0.0, 1e-5);
        assert_approx_eq(gadget.center.y, 0.0, 1e-5);
        assert_approx_eq(gadget.center.z, 0.0, 1e-5);
    }

    #[test]
    fn test_arrow_head_size() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let gadget = TetrahedronGadget::from_4d_vector(vector);

        let expected_size = gadget.scale * 0.15;
        assert_approx_eq(gadget.vector_arrow.arrow_head_size, expected_size, 1e-5);
    }

    #[test]
    fn test_vector_arrow_direction_consistency() {
        let s = 1.0 / SQRT_3;
        let vectors = vec![
            (Vector4::new(1.0, 0.0, 0.0, 0.0), s, s, s),
            (Vector4::new(0.0, 1.0, 0.0, 0.0), -s, -s, s),
            (Vector4::new(0.0, 0.0, 1.0, 0.0), -s, s, -s),
            (Vector4::new(0.0, 0.0, 0.0, 1.0), s, -s, -s),
        ];

        for (vector, expected_x, expected_y, expected_z) in vectors {
            let gadget = TetrahedronGadget::from_4d_vector(vector);
            assert_approx_eq(gadget.vector_arrow.end_position.x, expected_x, 1e-3);
            assert_approx_eq(gadget.vector_arrow.end_position.y, expected_y, 1e-3);
            assert_approx_eq(gadget.vector_arrow.end_position.z, expected_z, 1e-3);
        }
    }

    #[test]
    fn test_for_zone_up() {
        let vector = Vector4::new(0.0, 0.0, 1.0, 0.0);
        let gadget =
            TetrahedronGadget::for_zone(vector, Zone::North, UnitQuaternion::identity(), 1.0);

        let arrow = gadget.vector_arrow.end_position;
        assert!(
            arrow.y > 0.5,
            "Arrow should point up (positive Y), got {:?}",
            arrow
        );
    }

    #[test]
    fn test_for_zone_right() {
        let vector = Vector4::new(0.0, 1.0, 0.0, 0.0);
        let gadget =
            TetrahedronGadget::for_zone(vector, Zone::East, UnitQuaternion::identity(), 1.0);

        let arrow = gadget.vector_arrow.end_position;
        assert!(
            arrow.x > 0.5,
            "Arrow should point right (positive X), got {:?}",
            arrow
        );
    }

    #[test]
    fn test_component_color_zero() {
        let color = compute_component_color(0.0, 1.0);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 255);
        assert_eq!(color.b, 255);
    }

    #[test]
    fn test_component_color_positive_max() {
        let color = compute_component_color(1.0, 1.0);
        assert!(color.b == 255, "Positive max should be blue-ish");
        assert!(color.r < 200, "Positive max should have low red");
    }

    #[test]
    fn test_component_color_negative_max() {
        let color = compute_component_color(-1.0, 1.0);
        assert_eq!(color.r, 255, "Negative max should have full red");
        assert!(color.g < 200, "Negative max should have low green");
        assert!(color.b < 200, "Negative max should have low blue");
    }

    #[test]
    fn test_component_color_partial_strength() {
        let color = compute_component_color(0.5, 1.0);
        assert!(color.b == 255, "Positive should have full blue");
        assert!(
            color.r > 100,
            "Partial strength should have some red (less saturated)"
        );

        let color_max = compute_component_color(1.0, 1.0);
        assert!(
            color.r > color_max.r,
            "Partial strength should be less saturated than max"
        );
    }

    #[test]
    fn test_format_component_value() {
        assert_eq!(format_component_value(0.0), "0.00");
        assert_eq!(format_component_value(1.234), "1.23");
        assert_eq!(format_component_value(-0.567), "-0.57");
        assert_eq!(format_component_value(12.34), "12.3");
        assert_eq!(format_component_value(123.45), "123");
    }

    #[test]
    fn test_format_magnitude() {
        assert_eq!(format_magnitude(0.0), "0.00");
        assert_eq!(format_magnitude(1.234), "1.23");
        assert_eq!(format_magnitude(12.34), "12.3");
        assert_eq!(format_magnitude(123.45), "123");
    }

    #[test]
    fn test_gadget_with_tip_label() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0))
            .with_tip_label("test");
        assert_eq!(gadget.tip_label, Some("test".to_string()));
    }

    #[test]
    fn test_gadget_component_values() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 2.0, 3.0, 4.0));
        assert_approx_eq(gadget.component_values[0], 1.0, 1e-5);
        assert_approx_eq(gadget.component_values[1], 2.0, 1e-5);
        assert_approx_eq(gadget.component_values[2], 3.0, 1e-5);
        assert_approx_eq(gadget.component_values[3], 4.0, 1e-5);
    }

    #[test]
    fn test_gadget_vector_magnitude() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        assert_approx_eq(gadget.vector_magnitude, 1.0, 1e-5);

        let gadget2 = TetrahedronGadget::from_4d_vector(Vector4::new(2.0, 0.0, 0.0, 0.0));
        assert_approx_eq(gadget2.vector_magnitude, 2.0, 1e-5);
    }
}
