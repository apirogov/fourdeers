//! Tetrahedron visualization for 4D direction vectors
//!
//! A tetrahedron is used to visualize 4D unit vectors in 3D space.
//! The 4 vertices of the tetrahedron represent the positive X, Y, Z, W directions in 4D.
//! A vector arrow points from the center toward the relatively weighted direction.

pub mod color;
pub mod format;
pub mod gadget;
pub mod geometry;
pub mod types;

pub use color::{compute_component_color, ComponentColor};
pub use format::{format_component_value, format_magnitude};
pub use geometry::{magnitude_4d, SQRT_3};
pub use types::{
    tetrahedron_layout, TetrahedronEdge, TetrahedronGadget, TetrahedronLayout, TetrahedronVertex,
    VectorArrow,
};

#[cfg(test)]
mod tests {
    use nalgebra::{UnitQuaternion, Vector4};

    use crate::input::Zone;
    use crate::test_utils::assert_approx_eq;

    use super::*;

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
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_y() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(0.0, 1.0, 0.0, 0.0));
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_z() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(0.0, 0.0, 1.0, 0.0));
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, -expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, -expected, 1e-3);
    }

    #[test]
    fn test_vector_arrow_only_w() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(0.0, 0.0, 0.0, 1.0));
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
        assert_approx_eq(gadget.vector_arrow.end_position.x, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.0, 1e-5);
    }

    #[test]
    fn test_vector_arrow_zero_vector() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(0.0, 0.0, 0.0, 0.0));
        assert_approx_eq(gadget.vector_arrow.end_position.x, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.y, 0.0, 1e-5);
        assert_approx_eq(gadget.vector_arrow.end_position.z, 0.0, 1e-5);
    }

    #[test]
    fn test_vector_arrow_negative_x() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(-1.0, 0.0, 0.0, 0.0));
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(gadget.vector_arrow.end_position.x, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.y, expected, 1e-3);
        assert_approx_eq(gadget.vector_arrow.end_position.z, expected, 1e-3);
    }

    fn normalize_4d_vector(v: Vector4<f32>) -> Vector4<f32> {
        let norm = magnitude_4d(v);
        if norm < 1e-6 {
            Vector4::zeros()
        } else {
            v / norm
        }
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
        let normalized = normalize_4d_vector(Vector4::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(normalized.x, 0.0);
        assert_eq!(normalized.y, 0.0);
        assert_eq!(normalized.z, 0.0);
        assert_eq!(normalized.w, 0.0);
    }

    #[test]
    fn test_compute_weighted_direction() {
        let vector = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let direction = geometry::compute_weighted_direction_3d(vector) / SQRT_3;
        let expected = 1.0 / SQRT_3;
        assert_approx_eq(direction.x, expected, 1e-3);
        assert_approx_eq(direction.y, expected, 1e-3);
        assert_approx_eq(direction.z, expected, 1e-3);
    }

    #[test]
    fn test_tetrahedron_with_scale() {
        let scale = 2.0;
        let gadget =
            TetrahedronGadget::from_4d_vector_with_scale(Vector4::new(1.0, 0.0, 0.0, 0.0), scale);
        assert_approx_eq(gadget.scale, scale, 1e-5);
        let s = scale / SQRT_3;
        assert_approx_eq(gadget.vertices[0].position.x, s, 1e-5);
        assert_approx_eq(gadget.vertices[0].position.y, s, 1e-5);
        assert_approx_eq(gadget.vertices[0].position.z, s, 1e-5);
    }

    #[test]
    fn test_vertex_3d() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        let Some(pos) = gadget.vertex_3d(0) else {
            panic!("Failed to get vertex 3d position")
        };
        let s = 1.0 / SQRT_3;
        assert_approx_eq(pos.x, s, 1e-5);
        assert_approx_eq(pos.y, s, 1e-5);
        assert_approx_eq(pos.z, s, 1e-5);
    }

    #[test]
    fn test_vertex_3d_invalid_index() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        assert!(gadget.vertex_3d(10).is_none());
    }

    #[test]
    fn test_arrow_position() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
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
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
        assert_approx_eq(gadget.center.x, 0.0, 1e-5);
        assert_approx_eq(gadget.center.y, 0.0, 1e-5);
        assert_approx_eq(gadget.center.z, 0.0, 1e-5);
    }

    #[test]
    fn test_arrow_head_size() {
        let gadget = TetrahedronGadget::from_4d_vector(Vector4::new(1.0, 0.0, 0.0, 0.0));
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
        let gadget = TetrahedronGadget::for_zone(
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Zone::North,
            UnitQuaternion::identity(),
            1.0,
        );
        let arrow = gadget.vector_arrow.end_position;
        assert!(
            arrow.y > 0.5,
            "Arrow should point up (positive Y), got {:?}",
            arrow
        );
    }

    #[test]
    fn test_for_zone_right() {
        let gadget = TetrahedronGadget::for_zone(
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Zone::East,
            UnitQuaternion::identity(),
            1.0,
        );
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
        assert!(color.r > 100, "Partial strength should have some red");
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
