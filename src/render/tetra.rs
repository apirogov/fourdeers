//! Tetrahedron rendering: styles, label modes, and the core render function

use eframe::egui;

use crate::colors::{
    ARROW_GLOW, ARROW_PRIMARY, ARROW_TIP, LABEL_DEFAULT, OBJECT_TINT_NEGATIVE, OUTLINE_DEFAULT,
    OUTLINE_THIN, TEXT_HIGHLIGHT,
};
use crate::render::batch::LineBatch;
use crate::tetrahedron::TetrahedronGadget;

use super::ui::{draw_arrow_head, render_dual_outlined_text, render_outlined_text};
use super::{
    CompassFrameMode, StereoProjector, ARROW_END_DOT_RADIUS, ARROW_STROKE_WIDTH,
    BASE_LABEL_FONT_SIZE, BASE_LABEL_OFFSET_Y,
};

pub enum TetraLabelMode {
    Compass(CompassFrameMode),
    Raw,
    Hidden,
}

pub struct TetraStyle {
    pub edge_stroke_width: f32,
    pub edge_color: egui::Color32,
    pub vertex_label_font_size: f32,
    pub vertex_label_font_proportional: bool,
    pub label_mode: TetraLabelMode,
    pub label_normal_offset: f32,
    pub outline_color: egui::Color32,
    pub show_component_values: bool,
    pub component_value_font_size: f32,
    pub component_value_normal_offset: f32,
    pub arrow_stroke_width: f32,
    pub arrow_color: egui::Color32,
    pub arrow_head_scale: f32,
    pub origin_dot_radius: f32,
    pub origin_dot_color: egui::Color32,
    pub tip_dot_radius: f32,
    pub tip_label_font_size: f32,
    pub tip_label_offset_y: f32,
    pub tip_label_color: egui::Color32,
    pub base_label_font_size: f32,
    pub base_label_offset_y: f32,
}

impl TetraStyle {
    #[must_use]
    pub fn compass() -> Self {
        Self {
            edge_stroke_width: ARROW_STROKE_WIDTH,
            edge_color: OBJECT_TINT_NEGATIVE,
            vertex_label_font_size: 16.0,
            vertex_label_font_proportional: true,
            label_mode: TetraLabelMode::Compass(CompassFrameMode::World),
            label_normal_offset: 0.15,
            outline_color: OUTLINE_DEFAULT,
            show_component_values: true,
            component_value_font_size: 11.0,
            component_value_normal_offset: 0.35,
            arrow_stroke_width: 3.0,
            arrow_color: ARROW_PRIMARY,
            arrow_head_scale: 20.0,
            origin_dot_radius: ARROW_END_DOT_RADIUS,
            origin_dot_color: ARROW_GLOW,
            tip_dot_radius: 4.0,
            tip_label_font_size: 12.0,
            tip_label_offset_y: 15.0,
            tip_label_color: ARROW_TIP,
            base_label_font_size: BASE_LABEL_FONT_SIZE,
            base_label_offset_y: BASE_LABEL_OFFSET_Y,
        }
    }

    #[must_use]
    pub fn zone_tetra() -> Self {
        Self {
            edge_stroke_width: 1.5,
            edge_color: crate::colors::OBJECT_TINT_POSITIVE,
            vertex_label_font_size: 14.0,
            vertex_label_font_proportional: true,
            label_mode: TetraLabelMode::Raw,
            label_normal_offset: 0.0,
            outline_color: OUTLINE_DEFAULT,
            show_component_values: false,
            component_value_font_size: 10.0,
            component_value_normal_offset: 0.0,
            arrow_stroke_width: ARROW_STROKE_WIDTH,
            arrow_color: ARROW_PRIMARY,
            arrow_head_scale: 15.0,
            origin_dot_radius: 2.0,
            origin_dot_color: ARROW_GLOW,
            tip_dot_radius: ARROW_END_DOT_RADIUS,
            tip_label_font_size: 10.0,
            tip_label_offset_y: 12.0,
            tip_label_color: ARROW_TIP,
            base_label_font_size: BASE_LABEL_FONT_SIZE,
            base_label_offset_y: BASE_LABEL_OFFSET_Y,
        }
    }
}

pub(crate) fn render_tetrahedron(
    batch: &mut LineBatch,
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    project: impl Fn(f32, f32, f32) -> Option<egui::Pos2>,
    style: &TetraStyle,
) {
    render_tetra_edges(batch, gadget, &project, style);

    let component_mags: [f32; 4] = gadget.component_values.map(f32::abs);
    let max_mag = component_mags.iter().copied().fold(0.0f32, f32::max);
    render_tetra_labels(painter, gadget, &project, style, max_mag);

    render_tetra_arrow(batch, painter, gadget, &project, style);
}

fn render_tetra_edges(
    batch: &mut LineBatch,
    gadget: &TetrahedronGadget,
    project: &impl Fn(f32, f32, f32) -> Option<egui::Pos2>,
    style: &TetraStyle,
) {
    batch.set_stroke_width(style.edge_stroke_width);
    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget
            .vertex_3d(v0_idx)
            .and_then(|pos| project(pos.x, pos.y, pos.z));
        let p1 = gadget
            .vertex_3d(v1_idx)
            .and_then(|pos| project(pos.x, pos.y, pos.z));

        if let (Some(p0), Some(p1)) = (p0, p1) {
            batch.add_segment(p0, p1, style.edge_color);
        }
    }
}

fn render_tetra_labels(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    project: &impl Fn(f32, f32, f32) -> Option<egui::Pos2>,
    style: &TetraStyle,
    max_mag: f32,
) {
    if matches!(style.label_mode, TetraLabelMode::Hidden) {
        return;
    }

    let font_id = if style.vertex_label_font_proportional {
        egui::FontId::proportional(style.vertex_label_font_size)
    } else {
        egui::FontId::monospace(style.vertex_label_font_size)
    };

    for (i, vertex) in gadget.vertices.iter().enumerate() {
        let component = gadget.component_values[i];
        let color = crate::tetrahedron::compute_component_color(component, max_mag);
        let egui_color = color.to_egui_color();

        if let (Some(pos), Some(normal)) = (gadget.vertex_3d(i), gadget.vertex_normal(i)) {
            let label_x = pos.x + normal.x * style.label_normal_offset;
            let label_y = pos.y + normal.y * style.label_normal_offset;
            if let Some(p) = project(label_x, label_y, pos.z) {
                let vertex_label = match &style.label_mode {
                    TetraLabelMode::Compass(frame_mode) => {
                        compass_vertex_label(*frame_mode, i, component, &vertex.label)
                    }
                    TetraLabelMode::Raw => &vertex.label,
                    TetraLabelMode::Hidden => unreachable!(),
                };

                render_dual_outlined_text(
                    painter,
                    p,
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id.clone(),
                    egui_color,
                    style.outline_color,
                );
            }
        }

        if style.show_component_values {
            if let (Some(pos), Some(normal)) = (gadget.vertex_3d(i), gadget.vertex_normal(i)) {
                let offset = style.component_value_normal_offset;
                let label_x = pos.x + normal.x * offset;
                let label_y = pos.y + normal.y * offset;
                if let Some(label_p) = project(label_x, label_y, pos.z) {
                    let value_text = crate::tetrahedron::format_component_value(component);
                    let value_font = egui::FontId::monospace(style.component_value_font_size);

                    render_outlined_text(
                        painter,
                        label_p,
                        egui::Align2::CENTER_CENTER,
                        &value_text,
                        value_font,
                        TEXT_HIGHLIGHT,
                        OUTLINE_THIN,
                    );
                }
            }
        }
    }
}

fn render_tetra_arrow(
    batch: &mut LineBatch,
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    project: &impl Fn(f32, f32, f32) -> Option<egui::Pos2>,
    style: &TetraStyle,
) {
    let arrow = gadget.arrow_position();
    let arrow_p = project(arrow.x, arrow.y, arrow.z);
    let origin_p = project(0.0, 0.0, 0.0);
    let (Some(arrow_end), Some(arrow_start)) = (arrow_p, origin_p) else {
        return;
    };
    let arrow_vec = arrow_end - arrow_start;

    if arrow_vec.length() > 1e-3 {
        batch.add_segment_with_width(
            arrow_start,
            arrow_end,
            style.arrow_stroke_width,
            style.arrow_color,
        );

        let arrow_head_size = gadget.arrow_head_size() * style.arrow_head_scale;
        if arrow_vec.length() > arrow_head_size {
            draw_arrow_head(
                painter,
                arrow_end,
                arrow_vec,
                arrow_head_size,
                style.arrow_color,
            );
        }
    }

    batch.add_circle_filled(arrow_start, style.origin_dot_radius, style.origin_dot_color);

    if let Some(ref label) = gadget.base_label {
        let base_pos = arrow_start + egui::Vec2::new(0.0, style.base_label_offset_y);
        let font_id = egui::FontId::proportional(style.base_label_font_size);
        render_outlined_text(
            painter,
            base_pos,
            egui::Align2::CENTER_CENTER,
            label,
            font_id,
            LABEL_DEFAULT,
            OUTLINE_DEFAULT,
        );
    }

    if let Some(ref label) = gadget.tip_label {
        let tip_offset = egui::Vec2::new(0.0, -style.tip_label_offset_y);
        let label_pos = arrow_end + tip_offset;
        painter.text(
            label_pos,
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::proportional(style.tip_label_font_size),
            style.tip_label_color,
        );
    } else if arrow_vec.length() > 1e-3 {
        batch.add_circle_filled(arrow_end, style.tip_dot_radius, style.arrow_color);
    }
}

pub(crate) fn render_tetrahedron_with_projector(
    batch: &mut LineBatch,
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    frame_mode: CompassFrameMode,
) {
    let mut style = TetraStyle::compass();
    style.label_mode = TetraLabelMode::Compass(frame_mode);
    render_tetrahedron(
        batch,
        painter,
        gadget,
        |x, y, z| projector.project_3d(x, y, z).map(|p| p.screen_pos),
        &style,
    );
}

#[must_use]
pub fn compass_vertex_label(
    frame_mode: CompassFrameMode,
    component_index: usize,
    component_value: f32,
    world_label: &str,
) -> &str {
    if matches!(frame_mode, CompassFrameMode::World) {
        return world_label;
    }

    let positive = component_value >= 0.0;
    match component_index {
        0 => {
            if positive {
                "R"
            } else {
                "L"
            }
        }
        1 => {
            if positive {
                "U"
            } else {
                "D"
            }
        }
        2 => {
            if positive {
                "F"
            } else {
                "B"
            }
        }
        3 => {
            if positive {
                "K"
            } else {
                "A"
            }
        }
        _ => world_label,
    }
}
