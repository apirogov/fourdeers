use eframe::egui;
use nalgebra::{Vector3, Vector4};

use crate::colors::{ARROW_GLOW, ARROW_PRIMARY};
use crate::render::{CompassFrameMode, StereoProjector, TetraLabelMode, TetraStyle};
use crate::tetrahedron::TetrahedronGadget;

use super::{
    EDGE_STROKE_WIDTH, MAP_ARROW_HEAD_SCALE, MAP_DISTANCE_FONT_SIZE, MAP_DISTANCE_LABEL_OFFSET_Y,
    MAP_TIP_FONT_SIZE, MAP_TIP_LABEL_OFFSET_Y, MAP_VERTEX_FONT_SIZE,
};

pub(super) fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let ar = f32::from(a.r());
    let ag = f32::from(a.g());
    let ab = f32::from(a.b());
    let br = f32::from(b.r());
    let bg = f32::from(b.g());
    let bb = f32::from(b.b());
    egui::Color32::from_rgb(
        crate::colors::to_u8(ar + (br - ar) * t),
        crate::colors::to_u8(ag + (bg - ag) * t),
        crate::colors::to_u8(ab + (bb - ab) * t),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_tetrahedron_in_map(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    frame_mode: CompassFrameMode,
    edge_color: egui::Color32,
    alpha: f32,
    center_3d: Vector3<f32>,
    labels_visible: bool,
) {
    let a = crate::colors::to_u8;
    let style = TetraStyle {
        edge_stroke_width: EDGE_STROKE_WIDTH,
        edge_color: egui::Color32::from_rgba_unmultiplied(
            edge_color.r(),
            edge_color.g(),
            edge_color.b(),
            a(alpha * 200.0),
        ),
        vertex_label_font_size: MAP_VERTEX_FONT_SIZE,
        vertex_label_font_proportional: false,
        label_mode: if labels_visible {
            TetraLabelMode::Compass(frame_mode)
        } else {
            TetraLabelMode::Hidden
        },
        label_normal_offset: 0.12,
        outline_color: egui::Color32::from_rgba_unmultiplied(
            edge_color.r(),
            edge_color.g(),
            edge_color.b(),
            a(alpha * 120.0),
        ),
        show_component_values: false,
        component_value_font_size: 0.0,
        component_value_normal_offset: 0.0,
        arrow_stroke_width: 2.0,
        arrow_color: egui::Color32::from_rgba_unmultiplied(
            ARROW_PRIMARY.r(),
            ARROW_PRIMARY.g(),
            ARROW_PRIMARY.b(),
            a(alpha * 255.0),
        ),
        arrow_head_scale: MAP_ARROW_HEAD_SCALE,
        origin_dot_radius: 2.0,
        origin_dot_color: ARROW_GLOW,
        tip_dot_radius: 0.0,
        tip_label_font_size: MAP_TIP_FONT_SIZE,
        tip_label_offset_y: -MAP_TIP_LABEL_OFFSET_Y,
        tip_label_color: egui::Color32::from_rgba_unmultiplied(255, 180, 80, a(alpha * 230.0)),
        base_label_font_size: MAP_DISTANCE_FONT_SIZE,
        base_label_offset_y: MAP_DISTANCE_LABEL_OFFSET_Y,
    };

    crate::render::render_tetrahedron(
        painter,
        gadget,
        |x, y, z| {
            projector
                .project_3d(x + center_3d.x, y + center_3d.y, z + center_3d.z)
                .map(|p| p.screen_pos)
        },
        &style,
    );
}

pub(super) fn edge_axis(vertices: &[Vector4<f32>], i0: usize, i1: usize) -> Option<usize> {
    let v0 = vertices[i0];
    let v1 = vertices[i1];
    let mut diff_axis = None;
    for ax in 0..4 {
        if (v0[ax] - v1[ax]).abs() > f32::EPSILON {
            if diff_axis.is_some() {
                return None;
            }
            diff_axis = Some(ax);
        }
    }
    diff_axis
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::*;

    #[test]
    fn test_lerp_color_endpoints() {
        let a = egui::Color32::from_rgb(0, 0, 0);
        let b = egui::Color32::from_rgb(255, 255, 255);
        let at_zero = lerp_color(a, b, 0.0);
        let at_one = lerp_color(a, b, 1.0);
        assert_eq!(at_zero, a);
        assert_eq!(at_one, b);
    }

    #[test]
    fn test_lerp_color_midpoint() {
        let a = egui::Color32::from_rgb(0, 0, 0);
        let b = egui::Color32::from_rgb(100, 200, 50);
        let mid = lerp_color(a, b, 0.5);
        assert_eq!(mid.r(), 50);
        assert_eq!(mid.g(), 100);
        assert_eq!(mid.b(), 25);
    }

    #[test]
    fn test_lerp_color_clamps() {
        let a = egui::Color32::from_rgb(0, 0, 0);
        let b = egui::Color32::from_rgb(255, 255, 255);
        assert_eq!(lerp_color(a, b, -1.0), a);
        assert_eq!(lerp_color(a, b, 2.0), b);
    }
}
