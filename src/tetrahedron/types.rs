use eframe::egui;
use nalgebra::Vector3;

pub struct TetrahedronLayout {
    pub scale: f32,
    pub edge_offset: f32,
}

#[must_use]
pub fn tetrahedron_layout(view_rect: egui::Rect) -> TetrahedronLayout {
    let longer_side = view_rect.width().max(view_rect.height());
    TetrahedronLayout {
        scale: longer_side * 0.05,
        edge_offset: longer_side * 0.07,
    }
}

#[derive(Debug, Clone)]
pub struct TetrahedronVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub label: String,
    pub axis_4d: char,
}

#[derive(Debug, Clone, Copy)]
pub struct TetrahedronEdge {
    pub vertex_indices: [usize; 2],
}

#[derive(Debug, Clone)]
pub struct VectorArrow {
    pub end_position: Vector3<f32>,
    pub arrow_head_size: f32,
}

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
