//! Rendering utilities for stereo 3D visualization

use eframe::egui;
use nalgebra::UnitQuaternion;
use std::collections::HashMap;

use crate::camera::Camera;
use crate::colors::*;
use crate::input::{TetraId, Zone};
use crate::polytopes::Vertex4D;
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{
    format_magnitude, get_tetrahedron_layout, magnitude_4d, TetrahedronGadget,
};

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

pub fn draw_background(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, viewport_bg());
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
    let _third_w = view_rect.width() / 3.0;
    let _third_h = view_rect.height() / 3.0;

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

    let font_id = egui::FontId::proportional(11.0);
    let outline_color = outline_default();
    let text_color = text_color.unwrap_or_else(label_default);

    painter.text(label_pos, align, label, font_id.clone(), outline_color);
    painter.text(label_pos, align, label, font_id, text_color);
}

pub fn render_menu_label(painter: &egui::Painter, view_rect: egui::Rect) {
    render_tap_zone_label(painter, view_rect, Zone::NorthWest, "Menu", None);
}

pub fn render_stereo_menu<F>(painter: &egui::Painter, view_rect: egui::Rect, toy_menu: F)
where
    F: Fn(&egui::Painter, egui::Rect),
{
    let left_rect = egui::Rect {
        min: view_rect.min,
        max: egui::pos2(view_rect.center().x, view_rect.min.y + 30.0),
    };
    let right_rect = egui::Rect {
        min: egui::pos2(view_rect.center().x, view_rect.min.y),
        max: egui::pos2(view_rect.max.x, view_rect.min.y + 30.0),
    };

    render_common_menu_half(painter, left_rect);
    toy_menu(painter, right_rect);
}

pub fn render_common_menu_half(painter: &egui::Painter, rect: egui::Rect) {
    render_tap_zone_label(painter, rect, Zone::NorthWest, "Menu", None);
}

pub fn render_toy_menu_half<F>(painter: &egui::Painter, rect: egui::Rect, toy_menu: F)
where
    F: Fn(&egui::Painter, egui::Rect),
{
    toy_menu(painter, rect);
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

#[derive(Debug, Clone, Copy)]
pub struct FourDSettings {
    pub w_thickness: f32,
    pub w_color_intensity: f32,
}

impl Default for FourDSettings {
    fn default() -> Self {
        Self {
            w_thickness: 2.5,
            w_color_intensity: 0.35,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_eye_separation(mut self, separation: f32) -> Self {
        self.eye_separation = separation;
        self
    }

    pub fn with_projection_distance(mut self, distance: f32) -> Self {
        self.projection_distance = distance;
        self
    }

    pub fn with_projection_mode(mut self, mode: ProjectionMode) -> Self {
        self.projection_mode = mode;
        self
    }
}

pub fn w_to_color(normalized_w: f32, alpha: u8, intensity: f32) -> egui::Color32 {
    if normalized_w >= 0.0 {
        let t = normalized_w;
        let r = (255.0 * (1.0 - t)) as u8;
        let g = (255.0 * (1.0 - t * intensity)) as u8;
        let b = (255.0 * (1.0 - t) + 255.0 * t) as u8;
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    } else {
        let t = -normalized_w;
        let r = 255u8;
        let g = (255.0 * (1.0 - t * intensity)) as u8;
        let b = (255.0 * (1.0 - t)) as u8;
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StereoProjector {
    center: egui::Pos2,
    scale: f32,
    eye_separation: f32,
    projection_distance: f32,
    mode: ProjectionMode,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectedPoint {
    pub screen_pos: egui::Pos2,
    pub depth: f32,
}

impl StereoProjector {
    pub fn new(
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> Self {
        Self {
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
        }
    }

    pub fn from_rect(
        rect: egui::Rect,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> Self {
        let scale = rect.height().min(rect.width()) * 0.35;
        Self::new(
            rect.center(),
            scale,
            eye_separation,
            projection_distance,
            mode,
        )
    }

    pub fn center(&self) -> egui::Pos2 {
        self.center
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn with_center(&self, center: egui::Pos2) -> Self {
        Self {
            center,
            scale: self.scale,
            eye_separation: self.eye_separation,
            projection_distance: self.projection_distance,
            mode: self.mode,
        }
    }

    pub fn with_scale(&self, scale: f32) -> Self {
        Self {
            center: self.center,
            scale,
            eye_separation: self.eye_separation,
            projection_distance: self.projection_distance,
            mode: self.mode,
        }
    }

    pub fn project_3d(&self, x: f32, y: f32, z: f32, eye_sign: f32) -> Option<ProjectedPoint> {
        let eye_offset = eye_sign * self.eye_separation * 0.5;
        let x_shifted = x - eye_offset;

        let scale_factor = match self.mode {
            ProjectionMode::Perspective => {
                let z_offset = self.projection_distance + z;
                if z_offset <= 0.1 {
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

    pub fn project_3d_no_eye(&self, x: f32, y: f32, z: f32) -> Option<ProjectedPoint> {
        let scale_factor = match self.mode {
            ProjectionMode::Perspective => {
                let z_offset = self.projection_distance + z;
                if z_offset <= 0.1 {
                    return None;
                }
                self.projection_distance / z_offset
            }
            ProjectionMode::Orthographic => 1.0,
        };

        let final_scale = self.scale * scale_factor;
        Some(ProjectedPoint {
            screen_pos: egui::Pos2::new(
                self.center.x + x * final_scale,
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
    pub eye_separation: f32,
    pub projection_distance: f32,
    pub projection_mode: ProjectionMode,
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

pub struct EyeRenderOptions<'a> {
    pub eye_sign: f32,
    pub is_left_view: bool,
    pub show_debug: bool,
    pub show_controls: bool,
    pub tetrahedron_rotations: &'a HashMap<TetraId, UnitQuaternion<f32>>,
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
            eye_separation: config.stereo.eye_separation,
            projection_distance: config.stereo.projection_distance,
            projection_mode: ProjectionMode::Perspective,
        }
    }

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

    pub fn render_eye_view(
        &self,
        ui: &mut egui::Ui,
        view_rect: egui::Rect,
        transformed: &[TransformedVertex],
        options: EyeRenderOptions<'_>,
    ) {
        let center = view_rect.center();
        let scale = view_rect.height().min(view_rect.width()) * 0.35;

        let projector = StereoProjector::new(
            center,
            scale,
            self.eye_separation,
            self.projection_distance,
            self.projection_mode,
        );

        let painter = ui.painter().with_clip_rect(view_rect);

        self.render_edges(
            &painter,
            &projector,
            transformed,
            options.eye_sign,
            view_rect,
        );
        if options.show_debug {
            self.render_zone_labels(&painter, view_rect, options.is_left_view);
        }

        if options.is_left_view || options.show_controls {
            self.render_tetrahedron_gadget(
                &painter,
                view_rect,
                options.is_left_view,
                options.tetrahedron_rotations,
            );
        }
    }

    fn render_edges(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[TransformedVertex],
        eye_sign: f32,
        clip_rect: egui::Rect,
    ) {
        let stroke_width = 2.5;
        let near_plane = self.projection_distance;
        let margin = 50.0;
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
                    .project_3d(t0.x, t0.y, t0.z, eye_sign)
                    .map(|p| p.screen_pos)?;
                let s1 = projector
                    .project_3d(t1.x, t1.y, t1.z, eye_sign)
                    .map(|p| p.screen_pos)?;

                let seg_x_min = s0.x.min(s1.x);
                let seg_x_max = s0.x.max(s1.x);
                let seg_y_min = s0.y.min(s1.y);
                let seg_y_max = s0.y.max(s1.y);
                if seg_x_max < x_min || seg_x_min > x_max || seg_y_max < y_min || seg_y_min > y_max
                {
                    return None;
                }

                let w_avg = (t0.w + t1.w) / 2.0;
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

    fn render_zone_labels(
        &self,
        painter: &egui::Painter,
        view_rect: egui::Rect,
        is_left_view: bool,
    ) {
        if is_left_view {
            return;
        }

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
            let text = format!("{}\n{}\n{}", symbol, action, vector);
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgba_unmultiplied(200, 200, 200, 150),
            );
        }
    }

    fn render_tetrahedron_gadget(
        &self,
        painter: &egui::Painter,
        view_rect: egui::Rect,
        is_left_view: bool,
        tetrahedron_rotations: &HashMap<TetraId, UnitQuaternion<f32>>,
    ) {
        if is_left_view {
            return;
        }

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
            let tetra_id = TetraId { is_left_view, zone };
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

fn zone_to_direction_label(zone: Zone) -> &'static str {
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

fn render_single_tetrahedron(painter: &egui::Painter, spec: &TetraRenderSpec<'_>) {
    let gadget =
        TetrahedronGadget::for_zone(spec.vector_4d, spec.zone, spec.user_rotation, spec.scale);
    let focal_length = spec.scale * 3.0;

    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget.get_vertex_3d(v0_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > 0.1 {
                let s = focal_length / z_offset;
                Some((spec.center_x + pos.x * s, spec.center_y - pos.y * s))
            } else {
                None
            }
        });
        let p1 = gadget.get_vertex_3d(v1_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > 0.1 {
                let s = focal_length / z_offset;
                Some((spec.center_x + pos.x * s, spec.center_y - pos.y * s))
            } else {
                None
            }
        });

        if let (Some(p0), Some(p1)) = (p0, p1) {
            painter.line_segment(
                [egui::Pos2::new(p0.0, p0.1), egui::Pos2::new(p1.0, p1.1)],
                egui::Stroke::new(1.5, object_tint_positive()),
            );
        }
    }

    if spec.show_captions || spec.show_magnitudes {
        let component_mags: [f32; 4] = gadget.component_values.map(|v| v.abs());
        let max_mag = component_mags.iter().cloned().fold(0.0f32, f32::max);

        for (i, vertex) in gadget.vertices.iter().enumerate() {
            let component = gadget.component_values[i];

            if let Some(pos) = gadget.get_vertex_3d(i) {
                let z_offset = focal_length + pos.z;
                if z_offset > 0.1 {
                    let s = focal_length / z_offset;
                    let screen_pos =
                        egui::Pos2::new(spec.center_x + pos.x * s, spec.center_y - pos.y * s);

                    if spec.show_captions {
                        let color = crate::tetrahedron::compute_component_color(component, max_mag);
                        let egui_color = color.to_egui_color();
                        let font_id = egui::FontId::proportional(14.0);
                        let outline_color = outline_default();

                        painter.text(
                            screen_pos + egui::Vec2::new(0.5, 0.5),
                            egui::Align2::CENTER_CENTER,
                            &vertex.label,
                            font_id.clone(),
                            outline_color,
                        );
                        painter.text(
                            screen_pos + egui::Vec2::new(-0.5, -0.5),
                            egui::Align2::CENTER_CENTER,
                            &vertex.label,
                            font_id.clone(),
                            outline_color,
                        );
                        painter.text(
                            screen_pos,
                            egui::Align2::CENTER_CENTER,
                            &vertex.label,
                            font_id,
                            egui_color,
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
                            let outline_color = outline_thin();
                            let text_color = text_highlight();

                            painter.text(
                                label_pos + egui::Vec2::new(0.5, 0.5),
                                egui::Align2::CENTER_CENTER,
                                &value_text,
                                font_id.clone(),
                                outline_color,
                            );
                            painter.text(
                                label_pos + egui::Vec2::new(-0.5, -0.5),
                                egui::Align2::CENTER_CENTER,
                                &value_text,
                                font_id.clone(),
                                outline_color,
                            );
                            painter.text(
                                label_pos,
                                egui::Align2::CENTER_CENTER,
                                &value_text,
                                font_id,
                                text_color,
                            );
                        }
                    }
                }
            }
        }
    }

    let arrow = gadget.arrow_position();
    let z_offset = focal_length + arrow.z;
    if z_offset > 0.1 {
        let s = focal_length / z_offset;
        let center = egui::Pos2::new(spec.center_x, spec.center_y);
        let arrow_end = egui::Pos2::new(spec.center_x + arrow.x * s, spec.center_y - arrow.y * s);
        let arrow_vec = arrow_end - center;

        if arrow_vec.length() > 1e-3 {
            painter.line_segment([center, arrow_end], egui::Stroke::new(2.0, arrow_primary()));

            let arrow_head_size = gadget.arrow_head_size() * 15.0;
            if arrow_vec.length() > arrow_head_size {
                let dir = arrow_vec.normalized();
                let perp = egui::Vec2::new(-dir.y, dir.x);

                let arrow_tip = arrow_end;
                let arrow_base = arrow_end - dir * arrow_head_size;
                let arrow_left = arrow_base + perp * (arrow_head_size * 0.5);
                let arrow_right = arrow_base - perp * (arrow_head_size * 0.5);

                painter.add(egui::Shape::convex_polygon(
                    vec![arrow_tip, arrow_left, arrow_right],
                    arrow_primary(),
                    egui::Stroke::NONE,
                ));
            }
        }

        painter.circle_filled(center, 2.0, arrow_glow());

        if let Some(ref label) = gadget.tip_label() {
            let tip_offset = egui::Vec2::new(0.0, -12.0);
            let label_pos = arrow_end + tip_offset;
            painter.text(
                label_pos,
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::proportional(10.0),
                arrow_tip(),
            );
        } else if arrow_vec.length() > 1e-3 {
            painter.circle_filled(arrow_end, 3.0, arrow_primary());
        }
    }

    if let Some(label) = spec.base_label {
        let base_pos = egui::Pos2::new(spec.center_x, spec.center_y + 18.0);
        let font_id = egui::FontId::proportional(11.0);
        let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180);
        let text_color = label_default();

        painter.text(
            base_pos + egui::Vec2::new(0.5, 0.5),
            egui::Align2::CENTER_CENTER,
            label,
            font_id.clone(),
            outline_color,
        );
        painter.text(
            base_pos,
            egui::Align2::CENTER_CENTER,
            label,
            font_id,
            text_color,
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
            if val.abs() < 0.05 {
                String::new()
            } else if (val - 1.0).abs() < 0.05 {
                format!("+{}", axis)
            } else if (val + 1.0).abs() < 0.05 {
                format!("-{}", axis)
            } else {
                format!("{:+.1}{}", val, axis)
            }
        })
        .collect();

    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

/// Render a tetrahedron as a floating overlay (not in 3D scene) with stereo effect
pub fn render_stereo_tetrahedron_overlay(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    vector_4d: nalgebra::Vector4<f32>,
    waypoint_title: &str,
    frame_mode: CompassFrameMode,
    rotation: &UnitQuaternion<f32>,
    projector: &StereoProjector,
) {
    let (left_rect, right_rect) = split_stereo_views(rect);
    let magnitude_label = format_magnitude(magnitude_4d(vector_4d));
    let gadget = TetrahedronGadget::from_4d_vector_with_quaternion(vector_4d, *rotation, 1.0)
        .with_base_label(magnitude_label)
        .with_tip_label(waypoint_title);

    let left_projector = projector.with_center(left_rect.center());
    let left_painter = ui.painter().with_clip_rect(left_rect);
    render_tetrahedron_with_projector(&left_painter, &gadget, &left_projector, -1.0, frame_mode);

    let right_projector = projector.with_center(right_rect.center());
    let right_painter = ui.painter().with_clip_rect(right_rect);
    render_tetrahedron_with_projector(&right_painter, &gadget, &right_projector, 1.0, frame_mode);
}

fn render_tetrahedron_with_projector(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    eye_sign: f32,
    frame_mode: CompassFrameMode,
) {
    let center = projector.center();

    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget
            .get_vertex_3d(v0_idx)
            .and_then(|pos| projector.project_3d(pos.x, pos.y, pos.z, eye_sign));
        let p1 = gadget
            .get_vertex_3d(v1_idx)
            .and_then(|pos| projector.project_3d(pos.x, pos.y, pos.z, eye_sign));

        if let (Some(p0), Some(p1)) = (p0, p1) {
            painter.line_segment(
                [p0.screen_pos, p1.screen_pos],
                egui::Stroke::new(2.0, object_tint_negative()),
            );
        }
    }

    let component_mags: [f32; 4] = gadget.component_values.map(|v| v.abs());
    let max_mag = component_mags.iter().cloned().fold(0.0f32, f32::max);

    for (i, vertex) in gadget.vertices.iter().enumerate() {
        let component = gadget.component_values[i];
        let color = crate::tetrahedron::compute_component_color(component, max_mag);
        let egui_color = color.to_egui_color();

        if let (Some(pos), Some(normal)) = (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i)) {
            let label_offset = 0.15;
            let label_x = pos.x + normal.x * label_offset;
            let label_y = pos.y + normal.y * label_offset;
            if let Some(p) = projector.project_3d(label_x, label_y, pos.z, eye_sign) {
                let font_id = egui::FontId::proportional(16.0);
                let outline_color = outline_default();

                let vertex_label = compass_vertex_label(frame_mode, i, component, &vertex.label);

                painter.text(
                    p.screen_pos + egui::Vec2::new(0.5, 0.5),
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id.clone(),
                    outline_color,
                );
                painter.text(
                    p.screen_pos + egui::Vec2::new(-0.5, -0.5),
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id.clone(),
                    outline_color,
                );
                painter.text(
                    p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id,
                    egui_color,
                );
            }
        }

        if let (Some(pos), Some(normal)) = (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i)) {
            let label_offset = 0.35;
            let label_x = pos.x + normal.x * label_offset;
            let label_y = pos.y + normal.y * label_offset;
            if let Some(label_p) = projector.project_3d(label_x, label_y, pos.z, eye_sign) {
                let value_text = crate::tetrahedron::format_component_value(component);
                let font_id = egui::FontId::monospace(11.0);
                let outline_color = outline_thin();
                let text_color = text_highlight();

                painter.text(
                    label_p.screen_pos + egui::Vec2::new(0.5, 0.5),
                    egui::Align2::CENTER_CENTER,
                    &value_text,
                    font_id.clone(),
                    outline_color,
                );
                painter.text(
                    label_p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    &value_text,
                    font_id,
                    text_color,
                );
            }
        }
    }

    let arrow = gadget.arrow_position();
    if let Some(arrow_p) = projector.project_3d(arrow.x, arrow.y, arrow.z, eye_sign) {
        let arrow_end = arrow_p.screen_pos;
        let arrow_vec = arrow_end - center;

        if arrow_vec.length() > 1e-3 {
            painter.line_segment([center, arrow_end], egui::Stroke::new(3.0, arrow_primary()));

            let arrow_head_size = gadget.arrow_head_size() * 20.0;
            if arrow_vec.length() > arrow_head_size {
                let dir = arrow_vec.normalized();
                let perp = egui::Vec2::new(-dir.y, dir.x);

                let arrow_tip = arrow_end;
                let arrow_base = arrow_end - dir * arrow_head_size;
                let arrow_left = arrow_base + perp * (arrow_head_size * 0.5);
                let arrow_right = arrow_base - perp * (arrow_head_size * 0.5);

                painter.add(egui::Shape::convex_polygon(
                    vec![arrow_tip, arrow_left, arrow_right],
                    arrow_primary(),
                    egui::Stroke::NONE,
                ));
            }
        }

        painter.circle_filled(center, 3.0, arrow_glow());

        if let Some(ref label) = gadget.base_label {
            let base_pos = center + egui::Vec2::new(0.0, 18.0);
            let font_id = egui::FontId::proportional(11.0);
            let outline_color = outline_default();
            let text_color = label_default();

            painter.text(
                base_pos + egui::Vec2::new(0.5, 0.5),
                egui::Align2::CENTER_CENTER,
                label,
                font_id.clone(),
                outline_color,
            );
            painter.text(
                base_pos,
                egui::Align2::CENTER_CENTER,
                label,
                font_id,
                text_color,
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
                arrow_tip(),
            );
        } else if arrow_vec.length() > 1e-3 {
            painter.circle_filled(arrow_end, 4.0, arrow_primary());
        }
    }
}

fn compass_vertex_label(
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

    fn project_cube_for_eyes(
        vertices: &[(f32, f32, f32)],
        projector: &StereoProjector,
    ) -> (Vec<Option<ProjectedPoint>>, Vec<Option<ProjectedPoint>>) {
        let left: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| projector.project_3d(*x, *y, *z, -1.0))
            .collect();
        let right: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| projector.project_3d(*x, *y, *z, 1.0))
            .collect();
        (left, right)
    }

    #[test]
    fn test_stereo_eyes_produce_different_x_coordinates() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(&vertices, &projector);

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

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(&vertices, &projector);

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

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        let far_left = projector.project_3d(0.0, 0.0, -4.0, -1.0).unwrap();
        let far_right = projector.project_3d(0.0, 0.0, -4.0, 1.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = projector.project_3d(0.0, 0.0, -1.0, -1.0).unwrap();
        let near_right = projector.project_3d(0.0, 0.0, -1.0, 1.0).unwrap();
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

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Orthographic,
        );

        let far_left = projector.project_3d(0.0, 0.0, -10.0, -1.0).unwrap();
        let far_right = projector.project_3d(0.0, 0.0, -10.0, 1.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = projector.project_3d(0.0, 0.0, -1.0, -1.0).unwrap();
        let near_right = projector.project_3d(0.0, 0.0, -1.0, 1.0).unwrap();
        let near_parallax = (near_left.screen_pos.x - near_right.screen_pos.x).abs();

        assert_approx_eq(far_parallax, near_parallax, 1e-6);
    }

    #[test]
    fn test_no_eye_has_no_parallax() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        let with_eye = projector.project_3d(0.5, 0.3, -2.0, 1.0).unwrap();
        let no_eye = projector.project_3d_no_eye(0.5, 0.3, -2.0).unwrap();

        assert!(
            (with_eye.screen_pos.x - no_eye.screen_pos.x).abs() > 0.01,
            "With-eye projection should differ from no-eye"
        );
    }

    #[test]
    fn test_behind_camera_returns_none() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        assert!(projector.project_3d(0.0, 0.0, -5.1, 1.0).is_none());
    }

    #[test]
    fn test_left_eye_sees_right_right_eye_sees_left() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        let left = projector.project_3d(0.0, 0.0, -2.0, -1.0).unwrap();
        let right = projector.project_3d(0.0, 0.0, -2.0, 1.0).unwrap();

        let mono = projector.project_3d_no_eye(0.0, 0.0, -2.0).unwrap();

        assert!(
            left.screen_pos.x > mono.screen_pos.x,
            "Left eye ({:.4}) should be right of mono ({:.4}) — camera shifted left sees object shifted right",
            left.screen_pos.x,
            mono.screen_pos.x
        );
        assert!(
            right.screen_pos.x < mono.screen_pos.x,
            "Right eye ({:.4}) should be left of mono ({:.4}) — camera shifted right sees object shifted left",
            right.screen_pos.x,
            mono.screen_pos.x
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
}
