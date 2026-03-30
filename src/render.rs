//! Rendering utilities for stereo 3D visualization

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3};

use crate::camera::Camera;
use crate::colors::*;
use crate::input::{TetraId, Zone};
use crate::polytopes::Vertex4D;
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{get_tetrahedron_layout, TetrahedronGadget};

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
    pub fn new(center: egui::Pos2, scale: f32, eye_separation: f32, mode: ProjectionMode) -> Self {
        Self {
            center,
            scale,
            eye_separation,
            projection_distance: 3.0,
            mode,
        }
    }

    pub fn from_rect(rect: egui::Rect, eye_separation: f32, mode: ProjectionMode) -> Self {
        let scale = rect.height().min(rect.width()) * 0.35;
        Self::new(rect.center(), scale, eye_separation, mode)
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
        let (scale_factor, parallax) = match self.mode {
            ProjectionMode::Perspective => {
                let z_offset = self.projection_distance + z;
                if z_offset <= 0.1 {
                    return None;
                }
                let sf = self.projection_distance / z_offset;
                let eye_offset = eye_sign * self.eye_separation * 0.5;
                (sf, eye_offset * sf)
            }
            ProjectionMode::Orthographic => {
                let eye_offset = eye_sign * self.eye_separation * 0.5;
                let parallax = eye_offset;
                (1.0, parallax)
            }
        };

        let final_scale = self.scale * scale_factor;
        Some(ProjectedPoint {
            screen_pos: egui::Pos2::new(
                self.center.x + x * final_scale + parallax,
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

pub struct TesseractRenderContext {
    pub vertices: Vec<Vertex4D>,
    pub indices: Vec<u16>,
    pub object_rotation: Rotation4D,
    pub inv_q_left: UnitQuaternion<f32>,
    pub w_half: f32,
    pub camera_4d_rotation_inverse: Rotation4D,
    pub camera: Camera,
    pub w_color_intensity: f32,
    pub eye_separation: f32,
    pub projection_distance: f32,
    pub projection_mode: ProjectionMode,
}

impl TesseractRenderContext {
    pub fn new(
        vertices: Vec<Vertex4D>,
        indices: Vec<u16>,
        camera: &Camera,
        rot_xy: f32,
        rot_xz: f32,
        rot_yz: f32,
        rot_xw: f32,
        rot_yw: f32,
        rot_zw: f32,
        w_thickness: f32,
        w_color_intensity: f32,
        eye_separation: f32,
        projection_distance: f32,
        projection_mode: ProjectionMode,
    ) -> Self {
        let object_rotation =
            Rotation4D::from_6_plane_angles(rot_xy, rot_xz, rot_yz, rot_xw, rot_yw, rot_zw);

        let inv_q_left = camera.rotation_4d.q_left().inverse();
        let w_half = w_thickness * 0.5;
        let camera_4d_rotation_inverse = camera.rotation_4d.inverse_q_right_only();

        Self {
            vertices,
            indices,
            object_rotation,
            inv_q_left,
            w_half,
            camera_4d_rotation_inverse,
            camera: camera.clone(),
            w_color_intensity,
            eye_separation,
            projection_distance,
            projection_mode,
        }
    }

    pub fn with_stereo_settings(
        vertices: Vec<Vertex4D>,
        indices: Vec<u16>,
        camera: &Camera,
        rot_xy: f32,
        rot_xz: f32,
        rot_yz: f32,
        rot_xw: f32,
        rot_yw: f32,
        rot_zw: f32,
        w_thickness: f32,
        w_color_intensity: f32,
        stereo: &StereoSettings,
    ) -> Self {
        Self::new(
            vertices,
            indices,
            camera,
            rot_xy,
            rot_xz,
            rot_yz,
            rot_xw,
            rot_yw,
            rot_zw,
            w_thickness,
            w_color_intensity,
            stereo.eye_separation,
            stereo.projection_distance,
            stereo.projection_mode,
        )
    }

    pub fn render_eye_view(
        &self,
        ui: &mut egui::Ui,
        view_rect: egui::Rect,
        eye_sign: f32,
        is_left_view: bool,
        show_debug: bool,
        show_controls: bool,
        tetrahedron_rotations: &std::collections::HashMap<TetraId, UnitQuaternion<f32>>,
    ) {
        let center = view_rect.center();
        let scale = view_rect.height().min(view_rect.width()) * 0.35;

        let projector =
            StereoProjector::new(center, scale, self.eye_separation, self.projection_mode);

        let painter = ui.painter().with_clip_rect(view_rect);

        self.render_tesseract_edges(&painter, &projector, eye_sign);
        if show_debug {
            self.render_zone_labels(&painter, view_rect, is_left_view);
        }

        if is_left_view || show_controls {
            self.render_tetrahedron_gadget(
                &painter,
                view_rect,
                is_left_view,
                tetrahedron_rotations,
            );
        }
    }

    fn render_tesseract_edges(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        eye_sign: f32,
    ) {
        for chunk in self.indices.chunks(2) {
            if chunk.len() != 2 {
                continue;
            }

            let v0 = &self.vertices[chunk[0] as usize];
            let v1 = &self.vertices[chunk[1] as usize];

            let p0_object = self.object_rotation.rotate_vector(v0.position.into());
            let p1_object = self.object_rotation.rotate_vector(v1.position.into());

            let p0_world = p0_object - self.camera.position;
            let p1_world = p1_object - self.camera.position;

            let p0_4d = self.camera_4d_rotation_inverse.rotate_vector(p0_world);
            let p1_4d = self.camera_4d_rotation_inverse.rotate_vector(p1_world);

            let w0_in_slice = p0_4d.w >= -self.w_half && p0_4d.w <= self.w_half;
            let w1_in_slice = p1_4d.w >= -self.w_half && p1_4d.w <= self.w_half;

            if !w0_in_slice && !w1_in_slice {
                continue;
            }

            let (screen_p0, screen_p1) =
                self.project_edge_points(p0_4d, p1_4d, projector, eye_sign);

            let Some((s0, s1)) = screen_p0.zip(screen_p1) else {
                continue;
            };

            let w_avg = (p0_4d.w + p1_4d.w) / 2.0;
            let alpha = if w0_in_slice && w1_in_slice { 255 } else { 100 };

            let normalized_w = (w_avg / self.w_half).clamp(-1.0, 1.0);
            let color = w_to_color(normalized_w, alpha, self.w_color_intensity);

            painter.line_segment([s0, s1], egui::Stroke::new(2.5, color));
        }
    }

    fn project_edge_points(
        &self,
        p0_4d: nalgebra::Vector4<f32>,
        p1_4d: nalgebra::Vector4<f32>,
        projector: &StereoProjector,
        eye_sign: f32,
    ) -> (Option<egui::Pos2>, Option<egui::Pos2>) {
        let p0_rel = Vector3::new(p0_4d.x, p0_4d.y, p0_4d.z);
        let p1_rel = Vector3::new(p1_4d.x, p1_4d.y, p1_4d.z);

        let p0_cam = self.inv_q_left.transform_vector(&p0_rel);
        let p1_cam = self.inv_q_left.transform_vector(&p1_rel);

        let s0 = projector
            .project_3d(p0_cam.x, p0_cam.y, p0_cam.z, eye_sign)
            .map(|p| p.screen_pos);
        let s1 = projector
            .project_3d(p1_cam.x, p1_cam.y, p1_cam.z, eye_sign)
            .map(|p| p.screen_pos);

        (s0, s1)
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
        tetrahedron_rotations: &std::collections::HashMap<TetraId, UnitQuaternion<f32>>,
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

            render_single_tetrahedron(
                painter,
                vector_4d,
                zone,
                x,
                y,
                user_rotation,
                layout.scale,
                true,
                false,
                base_label,
            );
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

fn render_single_tetrahedron(
    painter: &egui::Painter,
    vector_4d: nalgebra::Vector4<f32>,
    zone: Zone,
    center_x: f32,
    center_y: f32,
    user_rotation: UnitQuaternion<f32>,
    scale: f32,
    show_captions: bool,
    show_magnitudes: bool,
    base_label: Option<&str>,
) {
    let gadget = TetrahedronGadget::for_zone(vector_4d, zone, user_rotation, scale);
    let focal_length = scale * 3.0;

    for edge in &gadget.edges {
        let v0_idx = edge.vertex_indices[0];
        let v1_idx = edge.vertex_indices[1];

        let p0 = gadget.get_vertex_3d(v0_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > 0.1 {
                let s = focal_length / z_offset;
                Some((center_x + pos.x * s, center_y - pos.y * s))
            } else {
                None
            }
        });
        let p1 = gadget.get_vertex_3d(v1_idx).and_then(|pos| {
            let z_offset = focal_length + pos.z;
            if z_offset > 0.1 {
                let s = focal_length / z_offset;
                Some((center_x + pos.x * s, center_y - pos.y * s))
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

    if show_captions || show_magnitudes {
        let component_mags: [f32; 4] = gadget.component_values.map(|v| v.abs());
        let max_mag = component_mags.iter().cloned().fold(0.0f32, f32::max);

        for (i, vertex) in gadget.vertices.iter().enumerate() {
            let component = gadget.component_values[i];

            if let Some(pos) = gadget.get_vertex_3d(i) {
                let z_offset = focal_length + pos.z;
                if z_offset > 0.1 {
                    let s = focal_length / z_offset;
                    let screen_pos = egui::Pos2::new(center_x + pos.x * s, center_y - pos.y * s);

                    if show_captions {
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

                    if show_magnitudes {
                        if let Some(normal) = gadget.get_vertex_normal(i) {
                            let label_x = pos.x + normal.x * 20.0;
                            let label_y = pos.y + normal.y * 20.0;
                            let label_pos =
                                egui::Pos2::new(center_x + label_x * s, center_y - label_y * s);
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
        let center = egui::Pos2::new(center_x, center_y);
        let arrow_end = egui::Pos2::new(center_x + arrow.x * s, center_y - arrow.y * s);
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

    if let Some(label) = base_label {
        let base_pos = egui::Pos2::new(center_x, center_y + 18.0);
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
    rotation: &UnitQuaternion<f32>,
    projector: &StereoProjector,
) {
    let (left_rect, right_rect) = split_stereo_views(rect);
    let gadget = TetrahedronGadget::from_4d_vector_with_quaternion(vector_4d, *rotation, 1.0)
        .with_auto_magnitude_label()
        .with_base_label("Compass");

    let left_projector = projector.with_center(left_rect.center());
    let left_painter = ui.painter().with_clip_rect(left_rect);
    render_tetrahedron_with_projector(&left_painter, &gadget, &left_projector, -1.0);

    let right_projector = projector.with_center(right_rect.center());
    let right_painter = ui.painter().with_clip_rect(right_rect);
    render_tetrahedron_with_projector(&right_painter, &gadget, &right_projector, 1.0);
}

fn render_tetrahedron_with_projector(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    eye_sign: f32,
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

                painter.text(
                    p.screen_pos + egui::Vec2::new(0.5, 0.5),
                    egui::Align2::CENTER_CENTER,
                    &vertex.label,
                    font_id.clone(),
                    outline_color,
                );
                painter.text(
                    p.screen_pos + egui::Vec2::new(-0.5, -0.5),
                    egui::Align2::CENTER_CENTER,
                    &vertex.label,
                    font_id.clone(),
                    outline_color,
                );
                painter.text(
                    p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    &vertex.label,
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
