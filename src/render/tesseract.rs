//! Tesseract rendering pipeline and zone tetrahedron rendering

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{format_4d_vector, Camera, CameraProjection};
use crate::input::{TetraId, Zone};
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
    projection: CameraProjection,
    pub w_half: f32,
    pub camera: &'a Camera,
    pub w_color_intensity: f32,
    pub projection_distance: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TesseractRenderConfig {
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
        camera: &'a Camera,
        config: TesseractRenderConfig,
    ) -> Self {
        let projection = CameraProjection::new(camera);
        let w_half = config.four_d.w_thickness * 0.5;

        Self {
            vertices,
            indices,
            projection,
            w_half,
            camera,
            w_color_intensity: config.four_d.w_color_intensity,
            projection_distance: config.stereo.projection_distance,
        }
    }

    #[must_use]
    pub fn transform_vertices(&self) -> Vec<TransformedVertex> {
        self.vertices
            .iter()
            .map(|v| {
                let (xyz, w) = self.projection.project(*v);
                TransformedVertex {
                    x: xyz.x,
                    y: xyz.y,
                    z: xyz.z,
                    w,
                    in_slice: w >= -self.w_half && w <= self.w_half,
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
        let basis = self.camera.rotation_4d().basis_vectors();
        let entries = compute_zone_layout(&basis, view_rect);

        let label_offsets: [(f32, f32); 8] = [
            (0.0, -0.5),
            (0.0, 0.3),
            (-0.5, 0.0),
            (0.6, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
        ];

        let offset = tetrahedron_layout(view_rect).edge_offset;

        for (i, entry) in entries.iter().enumerate() {
            let (symbol, action) = zone_label_text(entry.zone);
            let vector = format_4d_vector(entry.basis_vector, 0.05, 1);
            let text = format!("{symbol}\n{action}\n{vector}");
            let (dx, dy) = label_offsets[i];
            let pos = egui::Pos2::new(entry.x + dx * offset, entry.y + dy * offset);
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
        let basis = self.camera.rotation_4d().basis_vectors();
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

const fn zone_label_text(zone: Zone) -> (&'static str, &'static str) {
    match zone {
        Zone::North => ("\u{2191}", "Up"),
        Zone::South => ("\u{2193}", "Down"),
        Zone::West => ("\u{2190}", "Left"),
        Zone::East => ("\u{2192}", "Right"),
        Zone::NorthEast => ("\u{2197}", "Fwd"),
        Zone::SouthWest => ("\u{2199}", "Back"),
        Zone::NorthWest => ("\u{2196}", "Kata"),
        Zone::SouthEast => ("\u{2198}", "Ana"),
        Zone::Center => ("", ""),
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
