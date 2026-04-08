//! Rendering utilities for stereo 3D visualization

use eframe::egui;
use nalgebra::UnitQuaternion;
use std::collections::HashMap;

use crate::camera::Camera;
use crate::colors::{
    ARROW_GLOW, ARROW_PRIMARY, ARROW_TIP, LABEL_DEFAULT, OBJECT_TINT_NEGATIVE,
    OBJECT_TINT_POSITIVE, OUTLINE_DEFAULT, OUTLINE_THIN, TEXT_HIGHLIGHT, VIEWPORT_BG,
};
use crate::input::{TetraId, Zone};
use crate::polytopes::Vertex4D;
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{get_tetrahedron_layout, TetrahedronGadget};

pub const STEREO_SCALE_FACTOR: f32 = 0.35;
const EDGE_STROKE_WIDTH: f32 = 2.5;
const EDGE_CLIP_MARGIN: f32 = 50.0;
const NEAR_PLANE_THRESHOLD: f32 = 0.1;
const TETRA_FOCAL_LENGTH_SCALE: f32 = 3.0;
const TETRA_EDGE_STROKE: f32 = 1.5;
const ARROW_STROKE_WIDTH: f32 = 2.0;
const COMPASS_ARROW_STROKE_WIDTH: f32 = 3.0;
const ARROW_HEAD_SCALE: f32 = 15.0;
const COMPASS_ARROW_HEAD_SCALE: f32 = 20.0;
const ARROW_HEAD_HALF_WIDTH: f32 = 0.5;
const VERTEX_LABEL_FONT_SIZE: f32 = 14.0;
const MAGNITUDE_LABEL_FONT_SIZE: f32 = 10.0;
const COMPASS_LABEL_FONT_SIZE: f32 = 16.0;
const COMPASS_VALUE_FONT_SIZE: f32 = 11.0;
const BASE_LABEL_FONT_SIZE: f32 = 11.0;
const BASE_LABEL_OFFSET_Y: f32 = 18.0;
const TAP_LABEL_FONT_SIZE: f32 = 11.0;

#[must_use]
pub fn split_stereo_views(rect: egui::Rect) -> (egui::Rect, egui::Rect) {
    let left_rect = egui::Rect {
        min: rect.min,
        max: egui::pos2(rect.center().x, rect.max.y),
    };
    let right_rect = egui::Rect {
        min: egui::pos2(rect.center().x, rect.min.y),
        max: rect.max,
    };
    (left_rect, right_rect)
}

pub fn render_stereo_views(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    eye_separation: f32,
    projection_distance: f32,
    mode: ProjectionMode,
    render_fn: impl Fn(&egui::Painter, &StereoProjector, egui::Rect),
) {
    let (left_rect, right_rect) = split_stereo_views(rect);
    let scale = rect.height().min(rect.width() * 0.5) * STEREO_SCALE_FACTOR;

    let left_projector = StereoProjector::for_eye(
        left_rect.center(),
        scale,
        eye_separation,
        projection_distance,
        mode,
        -1.0,
    );
    let left_painter = ui.painter().with_clip_rect(left_rect);
    render_fn(&left_painter, &left_projector, left_rect);

    let right_projector = StereoProjector::for_eye(
        right_rect.center(),
        scale,
        eye_separation,
        projection_distance,
        mode,
        1.0,
    );
    let right_painter = ui.painter().with_clip_rect(right_rect);
    render_fn(&right_painter, &right_projector, right_rect);
}

pub fn draw_background(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, VIEWPORT_BG);
}

pub fn draw_center_divider(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
    );
}

pub fn render_tap_zone_label(
    painter: &egui::Painter,
    view_rect: egui::Rect,
    zone: Zone,
    label: &str,
    text_color: Option<egui::Color32>,
) {
    let (label_pos, align) = match zone {
        Zone::NorthWest => (view_rect.min, egui::Align2::LEFT_TOP),
        Zone::NorthEast => (
            egui::Pos2::new(view_rect.max.x, view_rect.min.y),
            egui::Align2::RIGHT_TOP,
        ),
        Zone::SouthWest => (
            egui::Pos2::new(view_rect.min.x, view_rect.max.y),
            egui::Align2::LEFT_BOTTOM,
        ),
        Zone::SouthEast => (view_rect.max, egui::Align2::RIGHT_BOTTOM),
        Zone::North => (
            egui::Pos2::new(view_rect.center().x, view_rect.min.y),
            egui::Align2::CENTER_TOP,
        ),
        Zone::South => (
            egui::Pos2::new(view_rect.center().x, view_rect.max.y),
            egui::Align2::CENTER_BOTTOM,
        ),
        Zone::West => (
            egui::Pos2::new(view_rect.min.x, view_rect.center().y),
            egui::Align2::LEFT_CENTER,
        ),
        Zone::East => (
            egui::Pos2::new(view_rect.max.x, view_rect.center().y),
            egui::Align2::RIGHT_CENTER,
        ),
        Zone::Center => (view_rect.center(), egui::Align2::CENTER_CENTER),
    };

    let font_id = egui::FontId::proportional(TAP_LABEL_FONT_SIZE);
    let outline_color = OUTLINE_DEFAULT;
    let text_color = text_color.unwrap_or(LABEL_DEFAULT);

    painter.text(label_pos, align, label, font_id.clone(), outline_color);
    painter.text(label_pos, align, label, font_id, text_color);
}

pub fn render_common_menu_half(painter: &egui::Painter, rect: egui::Rect) {
    render_tap_zone_label(painter, rect, Zone::NorthWest, "Menu", None);
}

/// Draw a filled triangular arrow head pointing in the given direction.
pub fn draw_arrow_head(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: egui::Vec2,
    head_size: f32,
    color: egui::Color32,
) {
    let dir = direction.normalized();
    let perp = egui::Vec2::new(-dir.y, dir.x);
    let arrow_base = tip - dir * head_size;
    let arrow_left = arrow_base + perp * (head_size * ARROW_HEAD_HALF_WIDTH);
    let arrow_right = arrow_base - perp * (head_size * ARROW_HEAD_HALF_WIDTH);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, arrow_left, arrow_right],
        color,
        egui::Stroke::NONE,
    ));
}

/// Draw text with a single-pixel outline behind it for readability.
pub fn render_outlined_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    text_color: egui::Color32,
    outline_color: egui::Color32,
) {
    painter.text(pos, align, text, font_id.clone(), outline_color);
    painter.text(pos, align, text, font_id, text_color);
}

const OUTLINE_OFFSET: f32 = 0.5;

/// Draw text with a dual offset outline (±0.5px) for high-contrast readability.
pub fn render_dual_outlined_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    text_color: egui::Color32,
    outline_color: egui::Color32,
) {
    let offset = egui::Vec2::new(OUTLINE_OFFSET, OUTLINE_OFFSET);
    painter.text(pos + offset, align, text, font_id.clone(), outline_color);
    painter.text(pos - offset, align, text, font_id.clone(), outline_color);
    painter.text(pos, align, text, font_id, text_color);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompassFrameMode {
    #[default]
    World,
    Camera,
}

impl CompassFrameMode {
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::World => Self::Camera,
            Self::Camera => Self::World,
        }
    }

    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::World => "Frame: World",
            Self::Camera => "Frame: Camera",
        }
    }
}

pub const DEFAULT_W_THICKNESS: f32 = 2.5;
pub const DEFAULT_W_COLOR_INTENSITY: f32 = 0.35;

#[derive(Debug, Clone, Copy)]
pub struct FourDSettings {
    pub w_thickness: f32,
    pub w_color_intensity: f32,
}

impl Default for FourDSettings {
    fn default() -> Self {
        Self {
            w_thickness: DEFAULT_W_THICKNESS,
            w_color_intensity: DEFAULT_W_COLOR_INTENSITY,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StereoSettings {
    pub eye_separation: f32,
    pub projection_distance: f32,
    pub projection_mode: ProjectionMode,
}

impl Default for StereoSettings {
    fn default() -> Self {
        Self {
            eye_separation: 0.12,
            projection_distance: 3.0,
            projection_mode: ProjectionMode::Perspective,
        }
    }
}

impl StereoSettings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_eye_separation(mut self, separation: f32) -> Self {
        self.eye_separation = separation;
        self
    }

    #[must_use]
    pub const fn with_projection_distance(mut self, distance: f32) -> Self {
        self.projection_distance = distance;
        self
    }

    #[must_use]
    pub const fn with_projection_mode(mut self, mode: ProjectionMode) -> Self {
        self.projection_mode = mode;
        self
    }
}

/// Compute the color for a normalized W coordinate.
///
/// Positive W fades toward blue, negative W fades toward red, with intensity
/// controlling how much the green channel is affected.
#[must_use]
pub fn w_to_color(normalized_w: f32, alpha: u8, intensity: f32) -> egui::Color32 {
    if normalized_w >= 0.0 {
        let t = normalized_w;
        let r = crate::colors::to_u8(255.0 * (1.0 - t));
        let g = crate::colors::to_u8(255.0 * (1.0 - t * intensity));
        let b = crate::colors::to_u8(255.0 * (1.0 - t) + 255.0 * t);
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    } else {
        let t = -normalized_w;
        let r = 255u8;
        let g = crate::colors::to_u8(255.0 * (1.0 - t * intensity));
        let b = crate::colors::to_u8(255.0 * (1.0 - t));
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StereoProjector {
    center: egui::Pos2,
    scale: f32,
    eye_offset: f32,
    projection_distance: f32,
    mode: ProjectionMode,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectedPoint {
    pub screen_pos: egui::Pos2,
    pub depth: f32,
}

impl StereoProjector {
    #[must_use]
    pub const fn new(
        center: egui::Pos2,
        scale: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> Self {
        Self {
            center,
            scale,
            eye_offset: 0.0,
            projection_distance,
            mode,
        }
    }

    #[must_use]
    pub fn for_eye(
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
        eye_sign: f32,
    ) -> Self {
        Self {
            center,
            scale,
            eye_offset: eye_sign * eye_separation * 0.5,
            projection_distance,
            mode,
        }
    }

    #[must_use]
    pub const fn center(&self) -> egui::Pos2 {
        self.center
    }

    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    #[must_use]
    pub const fn with_center(&self, center: egui::Pos2) -> Self {
        Self {
            center,
            scale: self.scale,
            eye_offset: self.eye_offset,
            projection_distance: self.projection_distance,
            mode: self.mode,
        }
    }

    #[must_use]
    pub const fn with_scale(&self, scale: f32) -> Self {
        Self {
            center: self.center,
            scale,
            eye_offset: self.eye_offset,
            projection_distance: self.projection_distance,
            mode: self.mode,
        }
    }

    #[must_use]
    pub fn project_3d(&self, x: f32, y: f32, z: f32) -> Option<ProjectedPoint> {
        let x_shifted = x - self.eye_offset;

        let scale_factor = match self.mode {
            ProjectionMode::Perspective => {
                let z_offset = self.projection_distance + z;
                if z_offset <= NEAR_PLANE_THRESHOLD {
                    return None;
                }
                self.projection_distance / z_offset
            }
            ProjectionMode::Orthographic => 1.0,
        };

        let final_scale = self.scale * scale_factor;
        Some(ProjectedPoint {
            screen_pos: egui::Pos2::new(
                self.center.x + x_shifted * final_scale,
                self.center.y - y * final_scale,
            ),
            depth: z,
        })
    }
}

pub struct TesseractRenderContext<'a> {
    pub vertices: &'a [Vertex4D],
    pub indices: &'a [u16],
    mat_4d: nalgebra::Matrix4<f32>,
    offset_4d: nalgebra::Vector4<f32>,
    mat_3d: nalgebra::Matrix4<f32>,
    pub w_half: f32,
    pub camera: Camera,
    pub w_color_intensity: f32,
    pub projection_distance: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectRotationAngles {
    pub xy: f32,
    pub xz: f32,
    pub yz: f32,
    pub xw: f32,
    pub yw: f32,
    pub zw: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TesseractRenderConfig {
    pub rotation_angles: ObjectRotationAngles,
    pub four_d: FourDSettings,
    pub stereo: StereoSettings,
}

struct TetraRenderSpec<'a> {
    vector_4d: nalgebra::Vector4<f32>,
    zone: Zone,
    center_x: f32,
    center_y: f32,
    user_rotation: UnitQuaternion<f32>,
    scale: f32,
    show_captions: bool,
    show_magnitudes: bool,
    base_label: Option<&'a str>,
}

pub struct TransformedVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub in_slice: bool,
}

impl<'a> TesseractRenderContext<'a> {
    #[must_use]
    pub fn from_config(
        vertices: &'a [Vertex4D],
        indices: &'a [u16],
        camera: &Camera,
        config: TesseractRenderConfig,
    ) -> Self {
        let object_rotation = Rotation4D::from_6_plane_angles(
            config.rotation_angles.xy,
            config.rotation_angles.xz,
            config.rotation_angles.yz,
            config.rotation_angles.xw,
            config.rotation_angles.yw,
            config.rotation_angles.zw,
        );

        let inv_q_left = camera.rotation_4d.q_left().inverse();
        let w_half = config.four_d.w_thickness * 0.5;
        let camera_4d_rotation_inverse = camera.rotation_4d.inverse_q_right_only();

        let combined_4d = object_rotation.then(&camera_4d_rotation_inverse);
        let mat_4d = combined_4d.to_matrix();
        let offset_4d = camera_4d_rotation_inverse.rotate_vector(camera.position);

        let cam_3d = inv_q_left.to_rotation_matrix();
        let mat_3d = nalgebra::Matrix4::new(
            cam_3d[(0, 0)],
            cam_3d[(0, 1)],
            cam_3d[(0, 2)],
            0.0,
            cam_3d[(1, 0)],
            cam_3d[(1, 1)],
            cam_3d[(1, 2)],
            0.0,
            cam_3d[(2, 0)],
            cam_3d[(2, 1)],
            cam_3d[(2, 2)],
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        );

        Self {
            vertices,
            indices,
            mat_4d,
            offset_4d,
            mat_3d,
            w_half,
            camera: camera.clone(),
            w_color_intensity: config.four_d.w_color_intensity,
            projection_distance: config.stereo.projection_distance,
        }
    }

    #[must_use]
    pub fn transform_vertices(&self) -> Vec<TransformedVertex> {
        self.vertices
            .iter()
            .map(|v| {
                let v4 = nalgebra::Vector4::new(
                    v.position[0],
                    v.position[1],
                    v.position[2],
                    v.position[3],
                );
                let p_4d = self.mat_4d * v4 - self.offset_4d;
                let result = self.mat_3d * p_4d;
                TransformedVertex {
                    x: result.x,
                    y: result.y,
                    z: result.z,
                    w: p_4d.w,
                    in_slice: p_4d.w >= -self.w_half && p_4d.w <= self.w_half,
                }
            })
            .collect()
    }

    #[allow(clippy::similar_names)]
    pub fn render_edges(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[TransformedVertex],
        clip_rect: egui::Rect,
    ) {
        let stroke_width = EDGE_STROKE_WIDTH;
        let near_plane = self.projection_distance;
        let margin = EDGE_CLIP_MARGIN;
        let x_min = clip_rect.min.x - margin;
        let x_max = clip_rect.max.x + margin;
        let y_min = clip_rect.min.y - margin;
        let y_max = clip_rect.max.y + margin;

        let shapes: Vec<egui::Shape> = self
            .indices
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() != 2 {
                    return None;
                }

                let t0 = &transformed[chunk[0] as usize];
                let t1 = &transformed[chunk[1] as usize];

                if !t0.in_slice && !t1.in_slice {
                    return None;
                }

                if t0.z <= -near_plane && t1.z <= -near_plane {
                    return None;
                }

                let s0 = projector
                    .project_3d(t0.x, t0.y, t0.z)
                    .map(|p| p.screen_pos)?;
                let s1 = projector
                    .project_3d(t1.x, t1.y, t1.z)
                    .map(|p| p.screen_pos)?;

                let seg_x_min = s0.x.min(s1.x);
                let seg_x_max = s0.x.max(s1.x);
                let seg_y_min = s0.y.min(s1.y);
                let seg_y_max = s0.y.max(s1.y);
                if seg_x_max < x_min || seg_x_min > x_max || seg_y_max < y_min || seg_y_min > y_max
                {
                    return None;
                }

                let w_avg = f32::midpoint(t0.w, t1.w);
                let alpha = if t0.in_slice && t1.in_slice { 255 } else { 100 };

                let normalized_w = (w_avg / self.w_half).clamp(-1.0, 1.0);
                let color = w_to_color(normalized_w, alpha, self.w_color_intensity);

                Some(egui::Shape::line_segment(
                    [s0, s1],
                    egui::Stroke::new(stroke_width, color),
                ))
            })
            .collect();

        painter.extend(shapes);
    }

    pub fn render_zone_labels(&self, painter: &egui::Painter, view_rect: egui::Rect) {
        let basis = self.camera.rotation_4d.basis_vectors();
        let layout = get_tetrahedron_layout(view_rect);
        let offset = layout.edge_offset;
        let third_w = view_rect.width() / 3.0;
        let third_h = view_rect.height() / 3.0;

        let labels: Vec<(&str, String, &str, f32, f32)> = vec![
            (
                "↑",
                format_4d_vector_compact(basis[1]),
                "Up",
                view_rect.center().x,
                view_rect.min.y + offset * 0.5,
            ),
            (
                "↓",
                format_4d_vector_compact(neg_vec(basis[1])),
                "Down",
                view_rect.center().x,
                view_rect.max.y - offset * 0.7,
            ),
            (
                "←",
                format_4d_vector_compact(neg_vec(basis[0])),
                "Left",
                view_rect.min.x + offset * 0.5,
                view_rect.center().y,
            ),
            (
                "→",
                format_4d_vector_compact(basis[0]),
                "Right",
                view_rect.max.x - offset * 0.4,
                view_rect.center().y,
            ),
            (
                "↗",
                format_4d_vector_compact(basis[2]),
                "Fwd",
                view_rect.min.x + third_w * 2.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                "↙",
                format_4d_vector_compact(neg_vec(basis[2])),
                "Back",
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 2.5,
            ),
            (
                "↖",
                format_4d_vector_compact(basis[3]),
                "Kata",
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                "↘",
                format_4d_vector_compact(neg_vec(basis[3])),
                "Ana",
                view_rect.min.x + third_w * 2.5,
                view_rect.min.y + third_h * 2.5,
            ),
        ];

        for (symbol, vector, action, x, y) in labels {
            let pos = egui::Pos2::new(x, y);
            let text = format!("{symbol}\n{action}\n{vector}");
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(10.0),
                crate::colors::TEXT_DIM,
            );
        }
    }

    pub fn render_tetrahedron_gadget(
        &self,
        painter: &egui::Painter,
        view_rect: egui::Rect,
        tetrahedron_rotations: &HashMap<TetraId, UnitQuaternion<f32>>,
    ) {
        let basis = self.camera.rotation_4d.basis_vectors();
        let layout = get_tetrahedron_layout(view_rect);
        let offset = layout.edge_offset;
        let third_w = view_rect.width() / 3.0;
        let third_h = view_rect.height() / 3.0;

        let tetrahedra: Vec<(nalgebra::Vector4<f32>, Zone, f32, f32)> = vec![
            (
                nalgebra::Vector4::from(basis[1]),
                Zone::North,
                view_rect.center().x,
                view_rect.min.y + offset,
            ),
            (
                nalgebra::Vector4::from(neg_vec(basis[1])),
                Zone::South,
                view_rect.center().x,
                view_rect.max.y - offset,
            ),
            (
                nalgebra::Vector4::from(neg_vec(basis[0])),
                Zone::West,
                view_rect.min.x + offset,
                view_rect.center().y,
            ),
            (
                nalgebra::Vector4::from(basis[0]),
                Zone::East,
                view_rect.max.x - offset,
                view_rect.center().y,
            ),
            (
                nalgebra::Vector4::from(basis[2]),
                Zone::NorthEast,
                view_rect.min.x + third_w * 2.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                nalgebra::Vector4::from(neg_vec(basis[2])),
                Zone::SouthWest,
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 2.5,
            ),
            (
                nalgebra::Vector4::from(basis[3]),
                Zone::NorthWest,
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                nalgebra::Vector4::from(neg_vec(basis[3])),
                Zone::SouthEast,
                view_rect.min.x + third_w * 2.5,
                view_rect.min.y + third_h * 2.5,
            ),
        ];

        for (vector_4d, zone, x, y) in tetrahedra {
            let tetra_id = TetraId {
                is_left_view: false,
                zone,
            };
            let user_rotation = tetrahedron_rotations
                .get(&tetra_id)
                .copied()
                .unwrap_or_else(UnitQuaternion::identity);

            let base_label = zone_to_direction_label(zone);
            let base_label = if base_label.is_empty() {
                None
            } else {
                Some(base_label)
            };

            let spec = TetraRenderSpec {
                vector_4d,
                zone,
                center_x: x,
                center_y: y,
                user_rotation,
                scale: layout.scale,
                show_captions: true,
                show_magnitudes: false,
                base_label,
            };
            render_single_tetrahedron(painter, &spec);
        }
    }
}

const fn zone_to_direction_label(zone: Zone) -> &'static str {
    match zone {
        Zone::North => "U",
        Zone::South => "D",
        Zone::West => "L",
        Zone::East => "R",
        Zone::NorthEast => "F",
        Zone::SouthWest => "B",
        Zone::NorthWest => "K",
        Zone::SouthEast => "A",
        Zone::Center => "",
    }
}

#[allow(clippy::too_many_lines)]
fn render_single_tetrahedron(painter: &egui::Painter, spec: &TetraRenderSpec<'_>) {
    let gadget =
        TetrahedronGadget::for_zone(spec.vector_4d, spec.zone, spec.user_rotation, spec.scale);
    let focal_length = spec.scale * TETRA_FOCAL_LENGTH_SCALE;

    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget.get_vertex_3d(v0_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > NEAR_PLANE_THRESHOLD {
                let s = focal_length / z_offset;
                Some((spec.center_x + pos.x * s, spec.center_y - pos.y * s))
            } else {
                None
            }
        });
        let p1 = gadget.get_vertex_3d(v1_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > NEAR_PLANE_THRESHOLD {
                let s = focal_length / z_offset;
                Some((spec.center_x + pos.x * s, spec.center_y - pos.y * s))
            } else {
                None
            }
        });

        if let (Some(p0), Some(p1)) = (p0, p1) {
            painter.line_segment(
                [egui::Pos2::new(p0.0, p0.1), egui::Pos2::new(p1.0, p1.1)],
                egui::Stroke::new(TETRA_EDGE_STROKE, OBJECT_TINT_POSITIVE),
            );
        }
    }

    if spec.show_captions || spec.show_magnitudes {
        let component_mags: [f32; 4] = gadget.component_values.map(f32::abs);
        let max_mag = component_mags.iter().copied().fold(0.0f32, f32::max);

        for (i, vertex) in gadget.vertices.iter().enumerate() {
            let component = gadget.component_values[i];

            if let Some(pos) = gadget.get_vertex_3d(i) {
                let z_offset = focal_length + pos.z;
                if z_offset > NEAR_PLANE_THRESHOLD {
                    let s = focal_length / z_offset;
                    let screen_pos =
                        egui::Pos2::new(spec.center_x + pos.x * s, spec.center_y - pos.y * s);

                    if spec.show_captions {
                        let color = crate::tetrahedron::compute_component_color(component, max_mag);
                        let egui_color = color.to_egui_color();
                        let font_id = egui::FontId::proportional(VERTEX_LABEL_FONT_SIZE);

                        render_dual_outlined_text(
                            painter,
                            screen_pos,
                            egui::Align2::CENTER_CENTER,
                            &vertex.label,
                            font_id,
                            egui_color,
                            OUTLINE_DEFAULT,
                        );
                    }

                    if spec.show_magnitudes {
                        if let Some(normal) = gadget.get_vertex_normal(i) {
                            let label_x = pos.x + normal.x * 20.0;
                            let label_y = pos.y + normal.y * 20.0;
                            let label_pos = egui::Pos2::new(
                                spec.center_x + label_x * s,
                                spec.center_y - label_y * s,
                            );
                            let value_text = crate::tetrahedron::format_component_value(component);
                            let font_id = egui::FontId::monospace(10.0);

                            render_dual_outlined_text(
                                painter,
                                label_pos,
                                egui::Align2::CENTER_CENTER,
                                &value_text,
                                font_id,
                                TEXT_HIGHLIGHT,
                                OUTLINE_THIN,
                            );
                        }
                    }
                }
            }
        }
    }

    let arrow = gadget.arrow_position();
    let z_offset = focal_length + arrow.z;
    if z_offset > NEAR_PLANE_THRESHOLD {
        let s = focal_length / z_offset;
        let center = egui::Pos2::new(spec.center_x, spec.center_y);
        let arrow_end = egui::Pos2::new(spec.center_x + arrow.x * s, spec.center_y - arrow.y * s);
        let arrow_vec = arrow_end - center;

        if arrow_vec.length() > 1e-3 {
            painter.line_segment(
                [center, arrow_end],
                egui::Stroke::new(ARROW_STROKE_WIDTH, ARROW_PRIMARY),
            );

            let arrow_head_size = gadget.arrow_head_size() * ARROW_HEAD_SCALE;
            if arrow_vec.length() > arrow_head_size {
                draw_arrow_head(
                    painter,
                    arrow_end,
                    arrow_vec,
                    arrow_head_size,
                    ARROW_PRIMARY,
                );
            }
        }

        painter.circle_filled(center, 2.0, ARROW_GLOW);

        if let Some(ref label) = gadget.tip_label() {
            let tip_offset = egui::Vec2::new(0.0, -12.0);
            let label_pos = arrow_end + tip_offset;
            painter.text(
                label_pos,
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::proportional(MAGNITUDE_LABEL_FONT_SIZE),
                ARROW_TIP,
            );
        } else if arrow_vec.length() > 1e-3 {
            painter.circle_filled(arrow_end, 3.0, ARROW_PRIMARY);
        }
    }

    if let Some(label) = spec.base_label {
        let base_pos = egui::Pos2::new(spec.center_x, spec.center_y + BASE_LABEL_OFFSET_Y);
        let font_id = egui::FontId::proportional(BASE_LABEL_FONT_SIZE);
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
}

fn neg_vec(v: [f32; 4]) -> [f32; 4] {
    [-v[0], -v[1], -v[2], -v[3]]
}

fn format_4d_vector_compact(v: [f32; 4]) -> String {
    let components: [(f32, &str); 4] = [(v[0], "X"), (v[1], "Y"), (v[2], "Z"), (v[3], "W")];

    let parts: Vec<String> = components
        .iter()
        .filter(|(val, _)| val.abs() >= 0.05)
        .map(|(val, axis)| {
            if (val - 1.0).abs() < 0.05 {
                format!("+{axis}")
            } else if (val + 1.0).abs() < 0.05 {
                format!("-{axis}")
            } else {
                format!("{val:+.1}{axis}")
            }
        })
        .collect();

    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

#[allow(clippy::too_many_lines)]
pub fn render_tetrahedron_with_projector(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    frame_mode: CompassFrameMode,
) {
    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget
            .get_vertex_3d(v0_idx)
            .and_then(|pos| projector.project_3d(pos.x, pos.y, pos.z));
        let p1 = gadget
            .get_vertex_3d(v1_idx)
            .and_then(|pos| projector.project_3d(pos.x, pos.y, pos.z));

        if let (Some(p0), Some(p1)) = (p0, p1) {
            painter.line_segment(
                [p0.screen_pos, p1.screen_pos],
                egui::Stroke::new(ARROW_STROKE_WIDTH, OBJECT_TINT_NEGATIVE),
            );
        }
    }

    let component_mags: [f32; 4] = gadget.component_values.map(f32::abs);
    let max_mag = component_mags.iter().copied().fold(0.0f32, f32::max);

    for (i, vertex) in gadget.vertices.iter().enumerate() {
        let component = gadget.component_values[i];
        let color = crate::tetrahedron::compute_component_color(component, max_mag);
        let egui_color = color.to_egui_color();

        if let (Some(pos), Some(normal)) = (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i)) {
            let label_offset = 0.15;
            let label_x = pos.x + normal.x * label_offset;
            let label_y = pos.y + normal.y * label_offset;
            if let Some(p) = projector.project_3d(label_x, label_y, pos.z) {
                let font_id = egui::FontId::proportional(COMPASS_LABEL_FONT_SIZE);

                let vertex_label = compass_vertex_label(frame_mode, i, component, &vertex.label);

                render_dual_outlined_text(
                    painter,
                    p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id,
                    egui_color,
                    OUTLINE_DEFAULT,
                );
            }
        }

        if let (Some(pos), Some(normal)) = (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i)) {
            let label_offset = 0.35;
            let label_x = pos.x + normal.x * label_offset;
            let label_y = pos.y + normal.y * label_offset;
            if let Some(label_p) = projector.project_3d(label_x, label_y, pos.z) {
                let value_text = crate::tetrahedron::format_component_value(component);
                let font_id = egui::FontId::monospace(COMPASS_VALUE_FONT_SIZE);

                render_outlined_text(
                    painter,
                    label_p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    &value_text,
                    font_id,
                    TEXT_HIGHLIGHT,
                    OUTLINE_THIN,
                );
            }
        }
    }

    let arrow = gadget.arrow_position();
    let arrow_p = projector.project_3d(arrow.x, arrow.y, arrow.z);
    let origin_p = projector.project_3d(0.0, 0.0, 0.0);
    if let (Some(arrow_p), Some(origin_p)) = (arrow_p, origin_p) {
        let arrow_end = arrow_p.screen_pos;
        let arrow_start = origin_p.screen_pos;
        let arrow_vec = arrow_end - arrow_start;

        if arrow_vec.length() > 1e-3 {
            painter.line_segment(
                [arrow_start, arrow_end],
                egui::Stroke::new(COMPASS_ARROW_STROKE_WIDTH, ARROW_PRIMARY),
            );

            let arrow_head_size = gadget.arrow_head_size() * COMPASS_ARROW_HEAD_SCALE;
            if arrow_vec.length() > arrow_head_size {
                draw_arrow_head(
                    painter,
                    arrow_end,
                    arrow_vec,
                    arrow_head_size,
                    ARROW_PRIMARY,
                );
            }
        }

        painter.circle_filled(arrow_start, 3.0, ARROW_GLOW);

        if let Some(ref label) = gadget.base_label {
            let base_pos = arrow_start + egui::Vec2::new(0.0, BASE_LABEL_OFFSET_Y);
            let font_id = egui::FontId::proportional(BASE_LABEL_FONT_SIZE);
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
            let tip_offset = egui::Vec2::new(0.0, -15.0);
            let label_pos = arrow_end + tip_offset;
            painter.text(
                label_pos,
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::proportional(12.0),
                ARROW_TIP,
            );
        } else if arrow_vec.length() > 1e-3 {
            painter.circle_filled(arrow_end, 4.0, ARROW_PRIMARY);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;

    fn cube_vertices(x: f32, y: f32, z: f32) -> Vec<(f32, f32, f32)> {
        let size = 0.5;
        let rx: f32 = 0.3;
        let ry: f32 = 0.2;
        let rz: f32 = 0.1;
        let cos_x = rx.cos();
        let sin_x = rx.sin();
        let cos_y = ry.cos();
        let sin_y = ry.sin();
        let cos_z = rz.cos();
        let sin_z = rz.sin();

        let corners = [
            (-1.0, -1.0, -1.0),
            (1.0, -1.0, -1.0),
            (1.0, 1.0, -1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (1.0, -1.0, 1.0),
            (1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
        ];

        corners
            .iter()
            .map(|(cx, cy, cz)| {
                let px = cx * size;
                let py = cy * size;
                let pz = cz * size;
                let y1 = py * cos_x - pz * sin_x;
                let z1 = py * sin_x + pz * cos_x;
                let px1 = px * cos_y + z1 * sin_y;
                let z2 = -px * sin_y + z1 * cos_y;
                let px2 = px1 * cos_z - y1 * sin_z;
                let py2 = px1 * sin_z + y1 * cos_z;
                (x + px2, y + py2, z + z2)
            })
            .collect()
    }

    fn make_eye_projector(
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
        eye_sign: f32,
    ) -> StereoProjector {
        StereoProjector::for_eye(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            eye_sign,
        )
    }

    fn project_cube_for_eyes(
        vertices: &[(f32, f32, f32)],
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> (Vec<Option<ProjectedPoint>>, Vec<Option<ProjectedPoint>>) {
        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            1.0,
        );
        let left: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| left_proj.project_3d(*x, *y, *z))
            .collect();
        let right: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| right_proj.project_3d(*x, *y, *z))
            .collect();
        (left, right)
    }

    #[test]
    fn test_stereo_eyes_produce_different_x_coordinates() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(
            &vertices,
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            let l = l.expect("left should project");
            let r = r.expect("right should project");
            assert!(
                (l.screen_pos.x - r.screen_pos.x).abs() > 0.01,
                "Vertex {}: left.x ({:.4}) should differ from right.x ({:.4})",
                i,
                l.screen_pos.x,
                r.screen_pos.x
            );
        }
    }

    #[test]
    fn test_stereo_eyes_have_same_y_coordinates() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(
            &vertices,
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        for (l, r) in left.iter().zip(right.iter()) {
            let l = l.expect("left should project");
            let r = r.expect("right should project");
            assert_approx_eq(l.screen_pos.y, r.screen_pos.y, 1e-6);
        }
    }

    #[test]
    fn test_parallax_increases_with_depth() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let far_left = left_proj.project_3d(0.0, 0.0, -4.0).unwrap();
        let far_right = right_proj.project_3d(0.0, 0.0, -4.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = left_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_right = right_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_parallax = (near_left.screen_pos.x - near_right.screen_pos.x).abs();

        assert!(
            far_parallax > near_parallax,
            "Far parallax ({:.4}) should be greater than near parallax ({:.4})",
            far_parallax,
            near_parallax
        );
    }

    #[test]
    fn test_orthographic_parallax_constant_across_depth() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Orthographic,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Orthographic,
            1.0,
        );

        let far_left = left_proj.project_3d(0.0, 0.0, -10.0).unwrap();
        let far_right = right_proj.project_3d(0.0, 0.0, -10.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = left_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_right = right_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_parallax = (near_left.screen_pos.x - near_right.screen_pos.x).abs();

        assert_approx_eq(far_parallax, near_parallax, 1e-6);
    }

    #[test]
    fn test_no_eye_has_no_parallax() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let mono = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        let eye = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let with_eye = eye.project_3d(0.5, 0.3, -2.0).unwrap();
        let no_eye = mono.project_3d(0.5, 0.3, -2.0).unwrap();

        assert!(
            (with_eye.screen_pos.x - no_eye.screen_pos.x).abs() > 0.01,
            "With-eye projection should differ from no-eye"
        );
    }

    #[test]
    fn test_behind_camera_returns_none() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        assert!(projector.project_3d(0.0, 0.0, -5.1).is_none());
    }

    #[test]
    fn test_left_eye_sees_right_right_eye_sees_left() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let mono = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let left = left_proj.project_3d(0.0, 0.0, -2.0).unwrap();
        let right = right_proj.project_3d(0.0, 0.0, -2.0).unwrap();
        let mono_p = mono.project_3d(0.0, 0.0, -2.0).unwrap();

        assert!(
            left.screen_pos.x > mono_p.screen_pos.x,
            "Left eye ({:.4}) should be right of mono ({:.4}) — camera shifted left sees object shifted right",
            left.screen_pos.x,
            mono_p.screen_pos.x
        );
        assert!(
            right.screen_pos.x < mono_p.screen_pos.x,
            "Right eye ({:.4}) should be left of mono ({:.4}) — camera shifted right sees object shifted left",
            right.screen_pos.x,
            mono_p.screen_pos.x
        );
    }

    #[test]
    fn test_render_transform_matches_quaternion_pipeline() {
        use crate::camera::Camera;
        use crate::polytopes::Vertex4D;
        use nalgebra::Vector3;

        for (rot4d_x, rot4d_y, rot3d_x, rot3d_y) in [
            (0.0f32, 0.0f32, 0.0f32, 0.0f32),
            (50.0, 30.0, 0.0, 0.0),
            (0.0, 0.0, 20.0, -10.0),
            (50.0, 30.0, 20.0, -10.0),
            (-40.0, 70.0, 15.0, 25.0),
        ] {
            let mut camera = Camera::new();
            camera.rotate_4d(rot4d_x, rot4d_y);
            camera.rotate(rot3d_x, rot3d_y);

            let test_verts: Vec<Vertex4D> = vec![
                Vertex4D::new(1.0, 0.0, 0.0, 0.0),
                Vertex4D::new(0.0, 1.0, 0.0, 0.0),
                Vertex4D::new(0.0, 0.0, 1.0, 0.0),
                Vertex4D::new(0.0, 0.0, 0.0, 1.0),
                Vertex4D::new(1.0, 2.0, -3.0, 4.0),
                Vertex4D::new(-1.0, -2.0, 3.0, -4.0),
            ];
            let indices: Vec<u16> = vec![];

            let config = TesseractRenderConfig {
                rotation_angles: ObjectRotationAngles::default(),
                four_d: FourDSettings::default(),
                stereo: StereoSettings::default(),
            };

            let ctx = TesseractRenderContext::from_config(&test_verts, &indices, &camera, config);
            let transformed = ctx.transform_vertices();

            let qr_inv = camera.rotation_4d.inverse_q_right_only();
            let inv_q_left = camera.rotation_4d.q_left().inverse();

            for (i, v) in test_verts.iter().enumerate() {
                let v4 = nalgebra::Vector4::new(
                    v.position[0],
                    v.position[1],
                    v.position[2],
                    v.position[3],
                );
                let p_4d = qr_inv.rotate_vector(v4 - camera.position);

                let p3 = Vector3::new(p_4d.x, p_4d.y, p_4d.z);
                let expected_xyz = inv_q_left.transform_vector(&p3);
                let expected_w = p_4d.w;

                let t = &transformed[i];
                assert_approx_eq(t.x, expected_xyz.x, 1e-4);
                assert_approx_eq(t.y, expected_xyz.y, 1e-4);
                assert_approx_eq(t.z, expected_xyz.z, 1e-4);
                assert_approx_eq(t.w, expected_w, 1e-4);
            }
        }
    }

    #[test]
    fn test_w_to_color_zero_w() {
        let c = w_to_color(0.0, 255, 0.35);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 255);
        assert_eq!(c.b(), 255);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_positive_w_full() {
        let c = w_to_color(1.0, 255, 0.35);
        assert_eq!(c.r(), 0);
        assert_eq!(c.b(), 255);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_negative_w_full() {
        let c = w_to_color(-1.0, 255, 0.35);
        assert_eq!(c.r(), 255);
        assert_eq!(c.b(), 0);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_alpha_passthrough() {
        let c = w_to_color(0.0, 128, 0.35);
        assert_eq!(c.a(), 128);
    }

    #[test]
    fn test_w_to_color_positive_w_reduces_red() {
        let c_half = w_to_color(0.5, 255, 0.35);
        assert!(c_half.r() < 255);
        assert!(c_half.r() > 0);
    }

    #[test]
    fn test_w_to_color_negative_w_reduces_blue() {
        let c_half = w_to_color(-0.5, 255, 0.35);
        assert!(c_half.b() < 255);
        assert!(c_half.b() > 0);
    }

    #[test]
    fn test_w_to_color_intensity_affects_green() {
        let c_low = w_to_color(0.5, 255, 0.1);
        let c_high = w_to_color(0.5, 255, 0.9);
        assert!(c_low.g() > c_high.g());
    }

    #[test]
    fn test_compass_frame_mode_display_label() {
        assert_eq!(CompassFrameMode::World.display_label(), "Frame: World");
        assert_eq!(CompassFrameMode::Camera.display_label(), "Frame: Camera");
    }

    #[test]
    fn test_compass_frame_mode_other() {
        assert_eq!(CompassFrameMode::World.other(), CompassFrameMode::Camera);
        assert_eq!(CompassFrameMode::Camera.other(), CompassFrameMode::World);
    }
}
