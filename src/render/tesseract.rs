//! Tesseract rendering pipeline and zone tetrahedron rendering

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{format_4d_vector, Camera};
use crate::input::{TetraId, Zone};
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{tetrahedron_layout, TetrahedronGadget};

use super::ui::render_outlined_text;
use super::{
    w_to_color, FourDSettings, StereoProjector, StereoSettings, TetraStyle, BASE_LABEL_FONT_SIZE,
    BASE_LABEL_OFFSET_Y, NEAR_PLANE_THRESHOLD, TESSERACT_EDGE_STROKE_WIDTH,
};

const EDGE_CLIP_MARGIN: f32 = 50.0;
const TETRA_FOCAL_LENGTH_SCALE: f32 = 3.0;
const ZONE_LABEL_FONT_SIZE: f32 = 10.0;

pub struct TesseractRenderContext<'a> {
    pub vertices: &'a [Vector4<f32>],
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
        vertices: &'a [Vector4<f32>],
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
                let p_4d = self.mat_4d * v - self.offset_4d;
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
        let stroke_width = TESSERACT_EDGE_STROKE_WIDTH;
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
        let layout = tetrahedron_layout(view_rect);
        let offset = layout.edge_offset;
        let third_w = view_rect.width() / 3.0;
        let third_h = view_rect.height() / 3.0;

        let labels: Vec<(&str, String, &str, f32, f32)> = vec![
            (
                "\u{2191}",
                format_4d_vector(basis[1], 0.05, 1),
                "Up",
                view_rect.center().x,
                view_rect.min.y + offset * 0.5,
            ),
            (
                "\u{2193}",
                format_4d_vector(neg_vec(basis[1]), 0.05, 1),
                "Down",
                view_rect.center().x,
                view_rect.max.y - offset * 0.7,
            ),
            (
                "\u{2190}",
                format_4d_vector(neg_vec(basis[0]), 0.05, 1),
                "Left",
                view_rect.min.x + offset * 0.5,
                view_rect.center().y,
            ),
            (
                "\u{2192}",
                format_4d_vector(basis[0], 0.05, 1),
                "Right",
                view_rect.max.x - offset * 0.4,
                view_rect.center().y,
            ),
            (
                "\u{2197}",
                format_4d_vector(basis[2], 0.05, 1),
                "Fwd",
                view_rect.min.x + third_w * 2.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                "\u{2199}",
                format_4d_vector(neg_vec(basis[2]), 0.05, 1),
                "Back",
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 2.5,
            ),
            (
                "\u{2196}",
                format_4d_vector(basis[3], 0.05, 1),
                "Kata",
                view_rect.min.x + third_w * 0.5,
                view_rect.min.y + third_h * 0.5,
            ),
            (
                "\u{2198}",
                format_4d_vector(neg_vec(basis[3]), 0.05, 1),
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
                egui::FontId::proportional(ZONE_LABEL_FONT_SIZE),
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
        let entries = compute_zone_layout(&basis, view_rect);
        let scale = tetrahedron_layout(view_rect).scale;

        for entry in entries {
            let tetra_id = TetraId {
                is_left_view: false,
                zone: entry.zone,
            };
            let user_rotation = tetrahedron_rotations
                .get(&tetra_id)
                .copied()
                .unwrap_or_else(UnitQuaternion::identity);

            let base_label = zone_to_direction_label(entry.zone);
            let base_label = if base_label.is_empty() {
                None
            } else {
                Some(base_label)
            };

            let spec = TetraRenderSpec {
                vector_4d: Vector4::from(entry.basis_vector),
                zone: entry.zone,
                center_x: entry.x,
                center_y: entry.y,
                user_rotation,
                scale,
                base_label,
            };
            render_single_tetrahedron(painter, &spec);
        }
    }
}

struct ZoneLayoutEntry {
    basis_vector: [f32; 4],
    zone: Zone,
    x: f32,
    y: f32,
}

fn compute_zone_layout(basis: &[[f32; 4]; 4], view_rect: egui::Rect) -> Vec<ZoneLayoutEntry> {
    let offset = tetrahedron_layout(view_rect).edge_offset;
    let third_w = view_rect.width() / 3.0;
    let third_h = view_rect.height() / 3.0;

    vec![
        ZoneLayoutEntry {
            basis_vector: basis[1],
            zone: Zone::North,
            x: view_rect.center().x,
            y: view_rect.min.y + offset,
        },
        ZoneLayoutEntry {
            basis_vector: neg_vec(basis[1]),
            zone: Zone::South,
            x: view_rect.center().x,
            y: view_rect.max.y - offset,
        },
        ZoneLayoutEntry {
            basis_vector: neg_vec(basis[0]),
            zone: Zone::West,
            x: view_rect.min.x + offset,
            y: view_rect.center().y,
        },
        ZoneLayoutEntry {
            basis_vector: basis[0],
            zone: Zone::East,
            x: view_rect.max.x - offset,
            y: view_rect.center().y,
        },
        ZoneLayoutEntry {
            basis_vector: basis[2],
            zone: Zone::NorthEast,
            x: view_rect.min.x + third_w * 2.5,
            y: view_rect.min.y + third_h * 0.5,
        },
        ZoneLayoutEntry {
            basis_vector: neg_vec(basis[2]),
            zone: Zone::SouthWest,
            x: view_rect.min.x + third_w * 0.5,
            y: view_rect.min.y + third_h * 2.5,
        },
        ZoneLayoutEntry {
            basis_vector: basis[3],
            zone: Zone::NorthWest,
            x: view_rect.min.x + third_w * 0.5,
            y: view_rect.min.y + third_h * 0.5,
        },
        ZoneLayoutEntry {
            basis_vector: neg_vec(basis[3]),
            zone: Zone::SouthEast,
            x: view_rect.min.x + third_w * 2.5,
            y: view_rect.min.y + third_h * 2.5,
        },
    ]
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

fn render_single_tetrahedron(painter: &egui::Painter, spec: &TetraRenderSpec<'_>) {
    let gadget =
        TetrahedronGadget::for_zone(spec.vector_4d, spec.zone, spec.user_rotation, spec.scale);
    let focal_length = spec.scale * TETRA_FOCAL_LENGTH_SCALE;

    let mut style = TetraStyle::zone_tetra();
    if spec.base_label.is_some() {
        style.base_label_font_size = BASE_LABEL_FONT_SIZE;
        style.base_label_offset_y = BASE_LABEL_OFFSET_Y;
    }

    super::render_tetrahedron(
        painter,
        &gadget,
        |x, y, z| {
            let z_offset = focal_length + z;
            if z_offset > NEAR_PLANE_THRESHOLD {
                let s = focal_length / z_offset;
                Some(egui::Pos2::new(
                    spec.center_x + x * s,
                    spec.center_y - y * s,
                ))
            } else {
                None
            }
        },
        &style,
    );

    if let Some(label) = spec.base_label {
        let base_pos = egui::Pos2::new(spec.center_x, spec.center_y + BASE_LABEL_OFFSET_Y);
        let font_id = egui::FontId::proportional(BASE_LABEL_FONT_SIZE);
        render_outlined_text(
            painter,
            base_pos,
            egui::Align2::CENTER_CENTER,
            label,
            font_id,
            crate::colors::LABEL_DEFAULT,
            crate::colors::OUTLINE_DEFAULT,
        );
    }
}

fn neg_vec(v: [f32; 4]) -> [f32; 4] {
    [-v[0], -v[1], -v[2], -v[3]]
}
