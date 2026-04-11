use std::f32::consts::TAU;
use std::sync::Arc;

use eframe::egui;

pub(crate) struct LineBatch {
    mesh: egui::Mesh,
    stroke_width: f32,
}

impl LineBatch {
    pub fn new(stroke_width: f32) -> Self {
        let mut mesh = egui::Mesh::default();
        mesh.reserve_triangles(128);
        mesh.reserve_vertices(512);
        Self { mesh, stroke_width }
    }

    pub fn set_stroke_width(&mut self, width: f32) {
        self.stroke_width = width;
    }

    pub fn add_segment(&mut self, a: egui::Pos2, b: egui::Pos2, color: egui::Color32) {
        let dir = b - a;
        let len = dir.length();
        if len < 1e-10 {
            return;
        }
        let half_w = self.stroke_width * 0.5;
        let normal = egui::Vec2::new(-dir.y, dir.x) / len * half_w;
        let idx = self.mesh.vertices.len() as u32;
        self.mesh.colored_vertex(a + normal, color);
        self.mesh.colored_vertex(a - normal, color);
        self.mesh.colored_vertex(b + normal, color);
        self.mesh.colored_vertex(b - normal, color);
        self.mesh.add_triangle(idx, idx + 1, idx + 2);
        self.mesh.add_triangle(idx + 2, idx + 1, idx + 3);
    }

    pub fn add_segment_with_gradient(
        &mut self,
        a: egui::Pos2,
        b: egui::Pos2,
        color_a: egui::Color32,
        color_b: egui::Color32,
    ) {
        let dir = b - a;
        let len = dir.length();
        if len < 1e-10 {
            return;
        }
        let half_w = self.stroke_width * 0.5;
        let normal = egui::Vec2::new(-dir.y, dir.x) / len * half_w;
        let idx = self.mesh.vertices.len() as u32;
        self.mesh.colored_vertex(a + normal, color_a);
        self.mesh.colored_vertex(a - normal, color_a);
        self.mesh.colored_vertex(b + normal, color_b);
        self.mesh.colored_vertex(b - normal, color_b);
        self.mesh.add_triangle(idx, idx + 1, idx + 2);
        self.mesh.add_triangle(idx + 2, idx + 1, idx + 3);
    }

    pub fn add_segment_with_width(
        &mut self,
        a: egui::Pos2,
        b: egui::Pos2,
        width: f32,
        color: egui::Color32,
    ) {
        let saved = self.stroke_width;
        self.stroke_width = width;
        self.add_segment(a, b, color);
        self.stroke_width = saved;
    }

    pub fn add_convex_polygon_filled(&mut self, points: &[egui::Pos2], fill: egui::Color32) {
        if points.len() < 3 {
            return;
        }
        let base = self.mesh.vertices.len() as u32;
        for &p in points {
            self.mesh.colored_vertex(p, fill);
        }
        for i in 1..points.len() - 1 {
            self.mesh
                .add_triangle(base, base + i as u32, base + (i + 1) as u32);
        }
    }

    pub fn add_convex_polygon(
        &mut self,
        points: &[egui::Pos2],
        fill: egui::Color32,
        stroke_width: f32,
        stroke_color: egui::Color32,
    ) {
        self.add_convex_polygon_filled(points, fill);
        let n = points.len();
        for i in 0..n {
            let j = (i + 1) % n;
            self.add_segment_with_width(points[i], points[j], stroke_width, stroke_color);
        }
    }

    pub fn add_circle_filled(&mut self, center: egui::Pos2, radius: f32, color: egui::Color32) {
        if radius <= 0.0 {
            return;
        }
        const SEGMENTS: u32 = 16;
        let base = self.mesh.vertices.len() as u32;
        self.mesh.colored_vertex(center, color);
        for i in 0..=SEGMENTS {
            let angle = i as f32 * TAU / SEGMENTS as f32;
            let p = center + radius * egui::Vec2::new(angle.cos(), angle.sin());
            self.mesh.colored_vertex(p, color);
        }
        for i in 0..SEGMENTS {
            self.mesh.add_triangle(base, base + 1 + i, base + 2 + i);
        }
    }

    pub fn submit(self, painter: &egui::Painter) {
        if !self.mesh.indices.is_empty() {
            painter.add(egui::Shape::Mesh(Arc::new(self.mesh)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: egui::Pos2, b: egui::Pos2, tol: f32) -> bool {
        (a.x - b.x).abs() < tol && (a.y - b.y).abs() < tol
    }

    #[test]
    fn test_add_segment_produces_quad() {
        let mut batch = LineBatch::new(2.0);
        let a = egui::Pos2::new(10.0, 20.0);
        let b = egui::Pos2::new(30.0, 20.0);
        batch.add_segment(a, b, egui::Color32::WHITE);

        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);

        let half_w = 1.0;
        let expected_normal = egui::Vec2::new(0.0, 1.0) * half_w;
        assert!(approx_eq(mesh.vertices[0].pos, a + expected_normal, 1e-4));
        assert!(approx_eq(mesh.vertices[1].pos, a - expected_normal, 1e-4));
        assert!(approx_eq(mesh.vertices[2].pos, b + expected_normal, 1e-4));
        assert!(approx_eq(mesh.vertices[3].pos, b - expected_normal, 1e-4));
    }

    #[test]
    fn test_add_segment_zero_length_produces_nothing() {
        let mut batch = LineBatch::new(2.0);
        let p = egui::Pos2::new(5.0, 5.0);
        batch.add_segment(p, p, egui::Color32::WHITE);
        assert!(batch.mesh.indices.is_empty());
    }

    #[test]
    fn test_add_segment_vertical() {
        let mut batch = LineBatch::new(4.0);
        batch.add_segment(
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(0.0, 10.0),
            egui::Color32::WHITE,
        );
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 4);

        assert!(approx_eq(
            mesh.vertices[0].pos,
            egui::Pos2::new(-2.0, 0.0),
            1e-4
        ));
        assert!(approx_eq(
            mesh.vertices[1].pos,
            egui::Pos2::new(2.0, 0.0),
            1e-4
        ));
    }

    #[test]
    fn test_add_convex_polygon_triangle() {
        let mut batch = LineBatch::new(1.0);
        batch.add_convex_polygon_filled(
            &[
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(10.0, 0.0),
                egui::Pos2::new(5.0, 10.0),
            ],
            egui::Color32::RED,
        );
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.indices, &[0, 1, 2]);
    }

    #[test]
    fn test_add_convex_polygon_quad() {
        let mut batch = LineBatch::new(1.0);
        batch.add_convex_polygon_filled(
            &[
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(10.0, 0.0),
                egui::Pos2::new(10.0, 10.0),
                egui::Pos2::new(0.0, 10.0),
            ],
            egui::Color32::GREEN,
        );
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.indices, &[0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn test_add_convex_polygon_less_than_3_points() {
        let mut batch = LineBatch::new(1.0);
        batch.add_convex_polygon_filled(&[egui::Pos2::new(0.0, 0.0)], egui::Color32::WHITE);
        batch.add_convex_polygon_filled(
            &[egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0)],
            egui::Color32::WHITE,
        );
        assert!(batch.mesh.indices.is_empty());
    }

    #[test]
    fn test_add_circle_filled() {
        let mut batch = LineBatch::new(1.0);
        batch.add_circle_filled(egui::Pos2::new(50.0, 50.0), 5.0, egui::Color32::WHITE);
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 18); // 1 center + 17 perimeter
        assert_eq!(mesh.indices.len(), 48); // 16 triangles × 3
    }

    #[test]
    fn test_add_circle_zero_radius() {
        let mut batch = LineBatch::new(1.0);
        batch.add_circle_filled(egui::Pos2::new(50.0, 50.0), 0.0, egui::Color32::WHITE);
        assert!(batch.mesh.indices.is_empty());
    }

    #[test]
    fn test_add_circle_negative_radius() {
        let mut batch = LineBatch::new(1.0);
        batch.add_circle_filled(egui::Pos2::new(50.0, 50.0), -1.0, egui::Color32::WHITE);
        assert!(batch.mesh.indices.is_empty());
    }

    #[test]
    fn test_multiple_segments_batched() {
        let mut batch = LineBatch::new(1.0);
        batch.add_segment(
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(10.0, 0.0),
            egui::Color32::WHITE,
        );
        batch.add_segment(
            egui::Pos2::new(0.0, 5.0),
            egui::Pos2::new(10.0, 5.0),
            egui::Color32::WHITE,
        );
        batch.add_segment(
            egui::Pos2::new(0.0, 10.0),
            egui::Pos2::new(10.0, 10.0),
            egui::Color32::WHITE,
        );
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 12); // 3 segments × 4 vertices
        assert_eq!(mesh.indices.len(), 18); // 3 segments × 6 indices
    }

    #[test]
    fn test_mixed_shapes_batched() {
        let mut batch = LineBatch::new(1.0);
        batch.add_segment(
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(10.0, 0.0),
            egui::Color32::WHITE,
        );
        batch.add_convex_polygon_filled(
            &[
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(5.0, 0.0),
                egui::Pos2::new(0.0, 5.0),
            ],
            egui::Color32::RED,
        );
        batch.add_circle_filled(egui::Pos2::new(20.0, 20.0), 3.0, egui::Color32::BLUE);
        let mesh = batch.mesh;
        assert_eq!(mesh.vertices.len(), 4 + 3 + 18); // segment + triangle + circle
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn test_is_empty_initially() {
        let batch = LineBatch::new(1.0);
        assert!(batch.mesh.indices.is_empty());
    }

    #[test]
    fn test_add_segment_with_width_overrides() {
        let mut batch = LineBatch::new(1.0);
        batch.add_segment_with_width(
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(10.0, 0.0),
            4.0,
            egui::Color32::WHITE,
        );
        let mesh = batch.mesh;
        assert!(approx_eq(
            mesh.vertices[0].pos,
            egui::Pos2::new(0.0, 2.0),
            1e-4
        ));
        assert!(approx_eq(
            mesh.vertices[1].pos,
            egui::Pos2::new(0.0, -2.0),
            1e-4
        ));
    }

    #[test]
    fn test_stroke_width_restored_after_add_segment_with_width() {
        let mut batch = LineBatch::new(1.0);
        batch.add_segment_with_width(
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(10.0, 0.0),
            4.0,
            egui::Color32::WHITE,
        );
        assert_eq!(batch.stroke_width, 1.0);
    }
}
