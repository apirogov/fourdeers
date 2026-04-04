//! 4D Map view: a tesseract representing the 4D scene volume
//!
//! The map renders a tesseract wireframe (the 8-cell / hypercube) whose vertices
//! correspond to the signed axis extremes of the scene bounds. Inside, it shows:
//! - The scene camera's current 3D slice as a green filled truncated cube
//! - Waypoints as full labeled tetrahedra
//! - The scene camera position as a labeled tetrahedron
//!
//! The map has its own Camera allowing independent 3D/4D navigation.

#![allow(clippy::excessive_precision)]

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::camera::Camera;
use crate::colors::*;
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_views, CompassFrameMode, FourDSettings,
    ObjectRotationAngles, ProjectionMode, StereoProjector, StereoSettings, TesseractRenderConfig,
    TesseractRenderContext,
};
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{compute_component_color, TetrahedronGadget};
use crate::toy::CompassWaypoint;

const BOUNDS_PADDING_FACTOR: f32 = 0.2;
const SLICE_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
const MAP_CAMERA_BACK_OFFSET: f32 = 4.0;
const NEAR_MARGIN: f32 = 0.5;
#[cfg(test)]
const TESSERACT_EDGE_COUNT: usize = 32;

#[cfg(test)]
const TESSERACT_CROSS_SECTION_VERTEX_COUNT: usize = 8;

#[cfg(test)]
const TESSERACT_CROSS_SECTION_EDGE_COUNT: usize = 12;

const TESSERACT_FACES: [[u16; 4]; 24] = [
    [0, 2, 6, 4],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 1, 3, 2],
    [4, 5, 7, 6],
    [8, 10, 14, 12],
    [9, 11, 15, 13],
    [8, 9, 13, 12],
    [10, 11, 15, 14],
    [8, 9, 11, 10],
    [12, 13, 15, 14],
    [0, 2, 10, 8],
    [1, 3, 11, 9],
    [0, 1, 9, 8],
    [2, 3, 11, 10],
    [4, 6, 14, 12],
    [5, 7, 15, 13],
    [4, 5, 13, 12],
    [6, 7, 15, 14],
    [0, 4, 12, 8],
    [1, 5, 13, 9],
    [2, 6, 14, 10],
    [3, 7, 15, 11],
];

const AXIS_CHARS: [char; 4] = ['X', 'Y', 'Z', 'W'];

fn edge_axis(vertices: &[crate::polytopes::Vertex4D], i0: usize, i1: usize) -> Option<usize> {
    let v0 = vertices[i0].position;
    let v1 = vertices[i1].position;
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

fn slice_green_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(60, 180, 60, 40)
}

pub struct MapRenderer {
    camera: Camera,
    tesseract_vertices: Vec<crate::polytopes::Vertex4D>,
    tesseract_indices: Vec<u16>,
    w_thickness: f32,
    w_color_intensity: f32,
    projection_distance: f32,
    labels_visible: bool,
}
impl Default for MapRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl MapRenderer {
    pub fn new() -> Self {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        Self {
            camera: Camera::new(),
            tesseract_vertices: vertices,
            tesseract_indices: indices,
            w_thickness: 2.5,
            w_color_intensity: 0.35,
            projection_distance: 3.0,
            labels_visible: false,
        }
    }
    pub fn camera(&self) -> &Camera {
        &self.camera
    }
    pub fn toggle_labels(&mut self) {
        self.labels_visible = !self.labels_visible;
    }
    pub fn labels_visible(&self) -> bool {
        self.labels_visible
    }
    pub fn apply_action(&mut self, action: crate::camera::CameraAction, speed: f32) {
        self.camera.apply_action(action, speed);
    }
    pub fn rotate_3d(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate(delta_x, delta_y);
    }
    pub fn rotate_4d(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate_4d(delta_x, delta_y);
    }
    pub fn reset_to_fit(&mut self, scene_camera: &Camera, bounds: &(Vector4<f32>, Vector4<f32>)) {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let q_left = *scene_camera.rotation_4d.q_left();
        let offset_local = Vector3::new(0.0, 0.0, -MAP_CAMERA_BACK_OFFSET);
        let rotated_offset = q_left.transform_vector(&offset_local);
        self.camera.position =
            norm_cam + Vector4::new(rotated_offset[0], rotated_offset[1], rotated_offset[2], 0.0);
        self.camera
            .set_yaw_pitch_l(scene_camera.yaw_l(), scene_camera.pitch_l());
        self.camera.set_yaw_r(scene_camera.yaw_r());
        self.camera.set_pitch_r(scene_camera.pitch_r());
    }
    pub fn render(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        scene_camera: &Camera,
        waypoints: &[CompassWaypoint],
        stereo: StereoSettings,
        frame_mode: CompassFrameMode,
    ) {
        draw_background(ui, rect);
        draw_center_divider(ui, rect);
        let bounds = compute_bounds(scene_camera, waypoints);
        render_stereo_views(
            ui,
            rect,
            stereo.eye_separation,
            stereo.projection_distance,
            ProjectionMode::Perspective,
            |painter, projector, view_rect| {
                self.render_tesseract_wireframe(painter, projector, view_rect);
                self.render_slice_volume(painter, projector, scene_camera, &bounds);
                self.render_waypoints(
                    painter,
                    projector,
                    scene_camera,
                    waypoints,
                    &bounds,
                    frame_mode,
                );
                self.render_camera_position(painter, projector, scene_camera, &bounds, frame_mode);
            },
        );
    }
    fn render_tesseract_wireframe(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        _view_rect: egui::Rect,
    ) {
        let config = TesseractRenderConfig {
            rotation_angles: ObjectRotationAngles::default(),
            four_d: FourDSettings {
                w_thickness: self.w_thickness,
                w_color_intensity: self.w_color_intensity,
            },
            stereo: StereoSettings::new().with_projection_distance(self.projection_distance),
        };
        let ctx = TesseractRenderContext::from_config(
            &self.tesseract_vertices,
            &self.tesseract_indices,
            &self.camera,
            config,
        );
        let transformed = ctx.transform_vertices();
        ctx.render_edges(painter, projector, &transformed, painter.clip_rect());
        if self.labels_visible {
            self.render_vertex_labels(painter, projector, &transformed);
            self.render_edge_labels(painter, projector, &transformed);
        }
    }
    fn render_vertex_labels(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[crate::render::TransformedVertex],
    ) {
        let w_half = self.w_thickness * 0.5;
        for (i, tv) in transformed.iter().enumerate() {
            if !tv.in_slice {
                continue;
            }
            if tv.z <= -self.projection_distance {
                continue;
            }
            let Some(p) = projector.project_3d(tv.x, tv.y, tv.z) else {
                continue;
            };
            let vertex = &self.tesseract_vertices[i];
            let font_id = egui::FontId::monospace(8.0);
            for (ax, &ch) in AXIS_CHARS.iter().enumerate() {
                let component = vertex.position[ax];
                let color = compute_component_color(component, 1.0);
                let egui_color = color.to_egui_color();
                let offset_x = (ax as f32 - 1.5) * 7.0;
                painter.text(
                    p.screen_pos + egui::Vec2::new(offset_x, 8.0),
                    egui::Align2::CENTER_CENTER,
                    ch.to_string(),
                    font_id.clone(),
                    egui_color,
                );
            }
            let normalized_w = (tv.w / w_half).clamp(-1.0, 1.0);
            let dot_color = crate::render::w_to_color(normalized_w, 180, self.w_color_intensity);
            painter.circle_filled(p.screen_pos, 3.0, dot_color);
        }
    }
    fn render_edge_labels(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[crate::render::TransformedVertex],
    ) {
        let font_id = egui::FontId::monospace(7.0);
        let near_plane = self.projection_distance;
        for chunk in self.tesseract_indices.chunks(2) {
            if chunk.len() != 2 {
                continue;
            }
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let t0 = &transformed[i0];
            let t1 = &transformed[i1];
            if !t0.in_slice && !t1.in_slice {
                continue;
            }
            if t0.z <= -near_plane && t1.z <= -near_plane {
                continue;
            }
            let Some(s0) = projector.project_3d(t0.x, t0.y, t0.z) else {
                continue;
            };
            let Some(s1) = projector.project_3d(t1.x, t1.y, t1.z) else {
                continue;
            };
            let Some(ax) = edge_axis(&self.tesseract_vertices, i0, i1) else {
                continue;
            };
            let mid = (s0.screen_pos + s1.screen_pos.to_vec2()) * 0.5 + egui::Vec2::new(4.0, -6.0);
            let ch = AXIS_CHARS[ax];
            painter.text(
                mid,
                egui::Align2::CENTER_CENTER,
                ch.to_string(),
                font_id.clone(),
                egui::Color32::from_rgb(255, 230, 50),
            );
        }
    }
    fn render_slice_volume(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        scene_camera: &Camera,
        bounds: &(Vector4<f32>, Vector4<f32>),
    ) {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let slice_rotation = Rotation4D::new(
            UnitQuaternion::identity(),
            *scene_camera.rotation_4d.q_right(),
        );
        let basis_w = slice_rotation.basis_w();
        let slice_normal = Vector4::new(basis_w[0], basis_w[1], basis_w[2], basis_w[3]);
        let map_transform = MapViewTransform::new(&self.camera);
        let near_z = -self.projection_distance + NEAR_MARGIN;
        let cross_section_4d = compute_slice_cross_section(
            &self.tesseract_vertices,
            &self.tesseract_indices,
            slice_normal,
            norm_cam,
        );
        let cross_section_3d: Vec<Vector3<f32>> = cross_section_4d
            .iter()
            .filter_map(|p4d| {
                let pt3d = map_transform.project_to_3d(*p4d);
                if pt3d.z > near_z {
                    Some(pt3d)
                } else {
                    None
                }
            })
            .collect();
        let cs_edges = compute_cross_section_edges(
            &self.tesseract_vertices,
            &TESSERACT_FACES,
            slice_normal,
            norm_cam,
        );
        if cross_section_3d.len() >= 3 {
            let screen_pts = convex_hull_screen(&cross_section_3d, projector);
            if screen_pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    screen_pts,
                    slice_green_fill(),
                    egui::Stroke::new(1.5, SLICE_GREEN),
                ));
            }
        }
        for [p0, p1] in &cs_edges {
            if let Some(screen_seg) =
                clip_segment_to_screen(&map_transform, projector, near_z, *p0, *p1)
            {
                painter.line_segment(
                    [screen_seg.0, screen_seg.1],
                    egui::Stroke::new(2.0, SLICE_GREEN),
                );
            }
        }
    }
    fn render_waypoints(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        scene_camera: &Camera,
        waypoints: &[CompassWaypoint],
        bounds: &(Vector4<f32>, Vector4<f32>),
        frame_mode: CompassFrameMode,
    ) {
        let map_transform = MapViewTransform::new(&self.camera);
        let slice_info = SliceInfo::new(scene_camera, bounds, &self.camera, self.w_thickness);
        for wp in waypoints {
            let norm_pos = normalize_to_tesseract(wp.position, bounds);
            let vector_4d = match frame_mode {
                CompassFrameMode::Camera => scene_camera.world_vector_to_camera_frame(norm_pos),
                CompassFrameMode::World => norm_pos,
            };
            let s3d = map_transform.project_to_3d(norm_pos);
            if s3d.z <= -self.projection_distance {
                continue;
            }
            let alpha = slice_info.alpha_for_point(norm_pos);
            let scale = 0.15;
            let gadget = TetrahedronGadget::from_4d_vector_with_scale(vector_4d, scale)
                .with_tip_label(wp.title);
            let Some(center_screen) = projector.project_3d(s3d.x, s3d.y, s3d.z) else {
                continue;
            };
            let shifted = projector.with_center(center_screen.screen_pos);
            render_tetrahedron_with_projector(painter, &gadget, &shifted, frame_mode, alpha);
            if alpha < 1.0 {
                let dim_color =
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 150.0) as u8);
                painter.circle_filled(center_screen.screen_pos, 3.0, dim_color);
            }
        }
    }
    fn render_camera_position(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        scene_camera: &Camera,
        bounds: &(Vector4<f32>, Vector4<f32>),
        frame_mode: CompassFrameMode,
    ) {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let map_transform = MapViewTransform::new(&self.camera);
        let slice_info = SliceInfo::new(scene_camera, bounds, &self.camera, self.w_thickness);
        let s3d = map_transform.project_to_3d(norm_cam);
        if s3d.z <= -self.projection_distance {
            return;
        }
        let vector_4d = match frame_mode {
            CompassFrameMode::Camera => scene_camera.world_vector_to_camera_frame(norm_cam),
            CompassFrameMode::World => norm_cam,
        };
        let alpha = slice_info.alpha_for_point(norm_cam);
        let gadget =
            TetrahedronGadget::from_4d_vector_with_scale(vector_4d, 0.2).with_tip_label("Cam");
        let Some(center_screen) = projector.project_3d(s3d.x, s3d.y, s3d.z) else {
            return;
        };
        let shifted = projector.with_center(center_screen.screen_pos);
        render_tetrahedron_with_projector(painter, &gadget, &shifted, frame_mode, alpha);
        let dot_alpha = (alpha * 255.0) as u8;
        painter.circle_filled(
            center_screen.screen_pos,
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, dot_alpha),
        );
    }
}
struct SliceInfo {
    slice_normal: Vector4<f32>,
    norm_cam: Vector4<f32>,
    w_half: f32,
}
impl SliceInfo {
    fn new(
        scene_camera: &Camera,
        bounds: &(Vector4<f32>, Vector4<f32>),
        _map_camera: &Camera,
        w_thickness: f32,
    ) -> Self {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let slice_rotation = Rotation4D::new(
            UnitQuaternion::identity(),
            *scene_camera.rotation_4d.q_right(),
        );
        let basis_w = slice_rotation.basis_w();
        let slice_normal = Vector4::new(basis_w[0], basis_w[1], basis_w[2], basis_w[3]);
        Self {
            slice_normal,
            norm_cam,
            w_half: w_thickness * 0.5,
        }
    }
    fn alpha_for_point(&self, pos: Vector4<f32>) -> f32 {
        let d = (pos - self.norm_cam).dot(&self.slice_normal);
        if d.abs() <= self.w_half {
            1.0
        } else {
            let fade = ((d.abs() - self.w_half) / self.w_half).min(1.0);
            1.0 - fade * 0.7
        }
    }
}
fn render_tetrahedron_with_projector(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    frame_mode: CompassFrameMode,
    alpha: f32,
) {
    let edge_color = if alpha >= 0.99 {
        egui::Color32::from_rgba_unmultiplied(200, 200, 210, 200)
    } else {
        egui::Color32::from_rgba_unmultiplied(200, 200, 210, (alpha * 150.0) as u8)
    };
    for edge in &gadget.edges {
        let v0 = gadget.get_vertex_3d(edge.vertex_indices[0]).unwrap();
        let v1 = gadget.get_vertex_3d(edge.vertex_indices[1]).unwrap();
        if let (Some(p0), Some(p1)) = (
            projector.project_3d(v0.x, v0.y, v0.z),
            projector.project_3d(v1.x, v1.y, v1.z),
        ) {
            painter.line_segment(
                [p0.screen_pos, p1.screen_pos],
                egui::Stroke::new(1.5, edge_color),
            );
        }
    }
    let component_mags: [f32; 4] = gadget.component_values.map(|v| v.abs());
    let max_mag = component_mags.iter().cloned().fold(0.0f32, f32::max);
    for (i, vertex) in gadget.vertices.iter().enumerate() {
        let component = gadget.component_values[i];
        let color = compute_component_color(component, max_mag);
        if let (Some(pos), Some(normal)) = (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i)) {
            let label_offset = 0.12;
            let label_x = pos.x + normal.x * label_offset;
            let label_y = pos.y + normal.y * label_offset;
            if let Some(label_p) = projector.project_3d(label_x, label_y, pos.z) {
                let vertex_label =
                    crate::render::compass_vertex_label(frame_mode, i, component, &vertex.label);
                let font_id = egui::FontId::monospace(10.0);
                let a = (alpha * 230.0) as u8;
                let text_color =
                    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, a);
                let outline = egui::Color32::from_rgba_unmultiplied(0, 0, 0, (alpha * 180.0) as u8);
                painter.text(
                    label_p.screen_pos + egui::Vec2::new(0.5, 0.5),
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id.clone(),
                    outline,
                );
                painter.text(
                    label_p.screen_pos,
                    egui::Align2::CENTER_CENTER,
                    vertex_label,
                    font_id,
                    text_color,
                );
            }
        }
    }
    let arrow = gadget.arrow_position();
    if let (Some(arrow_p), Some(origin_p)) = (
        projector.project_3d(arrow.x, arrow.y, arrow.z),
        projector.project_3d(0.0, 0.0, 0.0),
    ) {
        let arrow_end = arrow_p.screen_pos;
        let arrow_start = origin_p.screen_pos;
        let arrow_vec = arrow_end - arrow_start;
        if arrow_vec.length() > 2.0 {
            let a = (alpha * 255.0) as u8;
            let arrow_color = egui::Color32::from_rgba_unmultiplied(255, 150, 50, a);
            painter.line_segment(
                [arrow_start, arrow_end],
                egui::Stroke::new(2.0, arrow_color),
            );
            let arrow_head_size = gadget.arrow_head_size() * 15.0;
            if arrow_vec.length() > arrow_head_size {
                let dir = arrow_vec.normalized();
                let perp = egui::Vec2::new(-dir.y, dir.x);
                let base = arrow_end - dir * arrow_head_size;
                let left = base + perp * (arrow_head_size * 0.4);
                let right = base - perp * (arrow_head_size * 0.4);
                painter.add(egui::Shape::convex_polygon(
                    vec![arrow_end, left, right],
                    arrow_color,
                    egui::Stroke::NONE,
                ));
            }
        }
        painter.circle_filled(arrow_start, 2.0, arrow_glow());
    }
    if let Some(ref label) = gadget.tip_label {
        if let Some(center) = projector.project_3d(0.0, 0.0, 0.0) {
            let a = (alpha * 230.0) as u8;
            painter.text(
                center.screen_pos + egui::Vec2::new(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgba_unmultiplied(255, 180, 80, a),
            );
        }
    }
}
struct MapViewTransform {
    mat_4d: nalgebra::Matrix4<f32>,
    offset_4d: Vector4<f32>,
    mat_3d: nalgebra::Rotation3<f32>,
}
impl MapViewTransform {
    fn new(map_camera: &Camera) -> Self {
        let map_inv = map_camera.rotation_4d.inverse_q_right_only();
        let mat_4d = map_inv.to_matrix();
        let offset_4d = map_inv.rotate_vector(map_camera.position);
        let mat_3d = map_camera
            .rotation_4d
            .q_left()
            .inverse()
            .to_rotation_matrix();
        Self {
            mat_4d,
            offset_4d,
            mat_3d,
        }
    }
    fn project_to_3d(&self, pos_4d: Vector4<f32>) -> Vector3<f32> {
        let r = self.mat_4d * pos_4d - self.offset_4d;
        self.mat_3d * Vector3::new(r.x, r.y, r.z)
    }
}
pub fn compute_bounds(
    scene_camera: &Camera,
    waypoints: &[CompassWaypoint],
) -> (Vector4<f32>, Vector4<f32>) {
    let mut min = scene_camera.position;
    let mut max = scene_camera.position;
    for wp in waypoints {
        for i in 0..4 {
            min[i] = min[i].min(wp.position[i]);
            max[i] = max[i].max(wp.position[i]);
        }
    }
    for i in 0..4 {
        let range = max[i] - min[i];
        if range < 1e-6 {
            min[i] -= 1.0;
            max[i] += 1.0;
        } else {
            let padding = range * BOUNDS_PADDING_FACTOR;
            min[i] -= padding;
            max[i] += padding;
        }
    }
    (min, max)
}
pub fn normalize_to_tesseract(
    pos: Vector4<f32>,
    bounds: &(Vector4<f32>, Vector4<f32>),
) -> Vector4<f32> {
    let mut result = Vector4::zeros();
    for i in 0..4 {
        let range = bounds.1[i] - bounds.0[i];
        if range.abs() < 1e-6 {
            result[i] = 0.0;
        } else {
            result[i] = 2.0 * (pos[i] - bounds.0[i]) / range - 1.0;
        }
    }
    result
}
fn vertex_to_4d(v: &crate::polytopes::Vertex4D) -> Vector4<f32> {
    Vector4::new(v.position[0], v.position[1], v.position[2], v.position[3])
}
fn clip_segment_to_screen(
    map_transform: &MapViewTransform,
    projector: &StereoProjector,
    near_z: f32,
    p0: Vector4<f32>,
    p1: Vector4<f32>,
) -> Option<(egui::Pos2, egui::Pos2)> {
    let mut s0 = map_transform.project_to_3d(p0);
    let mut s1 = map_transform.project_to_3d(p1);
    let in0 = s0.z > near_z;
    let in1 = s1.z > near_z;
    if !in0 && !in1 {
        return None;
    }
    if !in0 || !in1 {
        let dz = s1.z - s0.z;
        if dz.abs() < 1e-10 {
            return None;
        }
        let t = (near_z - s0.z) / dz;
        let clipped = s0 + (s1 - s0) * t;
        if !in0 {
            s0 = clipped;
        } else {
            s1 = clipped;
        }
    }
    let sp0 = projector.project_3d(s0.x, s0.y, s0.z)?;
    let sp1 = projector.project_3d(s1.x, s1.y, s1.z)?;
    Some((sp0.screen_pos, sp1.screen_pos))
}
#[cfg(test)]
struct SliceSegment {
    p0: Vector4<f32>,
    p1: Vector4<f32>,
    fully_in: bool,
}
fn compute_slice_cross_section(
    vertices: &[crate::polytopes::Vertex4D],
    indices: &[u16],
    slice_normal: Vector4<f32>,
    slice_origin: Vector4<f32>,
) -> Vec<Vector4<f32>> {
    let mut points = Vec::new();
    for chunk in indices.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }
        let p0 = vertex_to_4d(&vertices[chunk[0] as usize]);
        let p1 = vertex_to_4d(&vertices[chunk[1] as usize]);
        let d0 = (p0 - slice_origin).dot(&slice_normal);
        let d1 = (p1 - slice_origin).dot(&slice_normal);
        let denom = d1 - d0;
        if denom.abs() > 1e-10 {
            let t = -d0 / denom;
            if t > 0.0 && t < 1.0 {
                points.push(p0 + (p1 - p0) * t);
            }
        }
    }
    points
}
fn compute_cross_section_edges(
    vertices: &[crate::polytopes::Vertex4D],
    faces: &[[u16; 4]],
    slice_normal: Vector4<f32>,
    slice_origin: Vector4<f32>,
) -> Vec<[Vector4<f32>; 2]> {
    let mut edges = Vec::new();
    for face in faces {
        let face_verts: Vec<Vector4<f32>> = face
            .iter()
            .map(|&vi| vertex_to_4d(&vertices[vi as usize]))
            .collect();
        let n = face_verts.len();
        let distances: Vec<f32> = face_verts
            .iter()
            .map(|v| (v - slice_origin).dot(&slice_normal))
            .collect();
        let mut crossings: Vec<Vector4<f32>> = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let di = distances[i];
            let dj = distances[j];
            if di.signum() != dj.signum() && (di - dj).abs() > 1e-10 {
                let t = di / (di - dj);
                let t = t.clamp(0.0, 1.0);
                crossings.push(face_verts[i] + (face_verts[j] - face_verts[i]) * t);
            }
        }
        if crossings.len() == 2 {
            edges.push([crossings[0], crossings[1]]);
        }
    }
    edges
}
#[cfg(test)]
fn compute_in_band_segments(
    vertices: &[crate::polytopes::Vertex4D],
    indices: &[u16],
    slice_normal: Vector4<f32>,
    slice_origin: Vector4<f32>,
    w_half: f32,
) -> Vec<SliceSegment> {
    let mut segments = Vec::new();
    for chunk in indices.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }
        let p0 = vertex_to_4d(&vertices[chunk[0] as usize]);
        let p1 = vertex_to_4d(&vertices[chunk[1] as usize]);
        let d0 = (p0 - slice_origin).dot(&slice_normal);
        let d1 = (p1 - slice_origin).dot(&slice_normal);
        let denom = d1 - d0;
        let in0 = d0.abs() <= w_half;
        let in1 = d1.abs() <= w_half;
        if !in0 && !in1 {
            if d0.signum() != d1.signum() && denom.abs() > 1e-10 {
                let t_enter = (w_half * d0.signum() - d0) / denom;
                let t_exit = (w_half * d1.signum() - d0) / denom;
                let t_min = t_enter.min(t_exit).clamp(0.0, 1.0);
                let t_max = t_enter.max(t_exit).clamp(0.0, 1.0);
                segments.push(SliceSegment {
                    p0: p0 + (p1 - p0) * t_min,
                    p1: p0 + (p1 - p0) * t_max,
                    fully_in: false,
                });
            }
            continue;
        }
        let (tp0, tp1, fully_in) = if in0 && in1 {
            (p0, p1, true)
        } else {
            let outside_sign = if !in0 { d0.signum() } else { d1.signum() };
            let t = (w_half * outside_sign - d0) / denom;
            let t = t.clamp(0.0, 1.0);
            let clipped = p0 + (p1 - p0) * t;
            if !in0 {
                (clipped, p1, false)
            } else {
                (p0, clipped, false)
            }
        };
        segments.push(SliceSegment {
            p0: tp0,
            p1: tp1,
            fully_in,
        });
    }
    segments
}
fn convex_hull_screen(pts_3d: &[Vector3<f32>], projector: &StereoProjector) -> Vec<egui::Pos2> {
    let pts_2d: Vec<egui::Pos2> = pts_3d
        .iter()
        .filter_map(|v3| projector.project_3d(v3.x, v3.y, v3.z))
        .map(|p| p.screen_pos)
        .collect();
    convex_hull_2d(&pts_2d)
}
fn convex_hull_2d(pts: &[egui::Pos2]) -> Vec<egui::Pos2> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let mut start = 0;
    for i in 1..n {
        if pts[i].x < pts[start].x || (pts[i].x == pts[start].x && pts[i].y < pts[start].y) {
            start = i;
        }
    }
    let mut hull = Vec::new();
    let mut current = start;
    loop {
        hull.push(pts[current]);
        let mut next = 0;
        for i in 0..n {
            if i == current {
                continue;
            }
            if next == current {
                next = i;
                continue;
            }
            let oc = pts[i] - pts[current];
            let on = pts[next] - pts[current];
            let cross = oc.x * on.y - oc.y * on.x;
            if cross > 0.0 {
                next = i;
            } else if cross.abs() < 1e-10 {
                let d_i = oc.x * oc.x + oc.y * oc.y;
                let d_n = on.x * on.x + on.y * on.y;
                if d_i > d_n {
                    next = i;
                }
            }
        }
        current = next;
        if current == start {
            break;
        }
        if hull.len() > n {
            break;
        }
    }
    hull
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::ProjectionMode;
    use crate::test_utils::assert_approx_eq;
    fn make_projector() -> StereoProjector {
        StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        )
    }
    #[test]
    fn test_clip_segment_both_in_front() {
        let mt = MapViewTransform::new(&Camera::new());
        let proj = make_projector();
        let near_z = -3.0 + NEAR_MARGIN;
        let p0 = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 0.0);
        assert!(clip_segment_to_screen(&mt, &proj, near_z, p0, p1).is_some());
    }
    #[test]
    fn test_cross_section_default_w_slice_produces_cube() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        assert_eq!(
            cross.len(),
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            "w=0 tesseract cross-section should have {} vertices, got {}",
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            cross.len()
        );
        for pt in &cross {
            assert_approx_eq(pt[3], 0.0, 1e-6);
            for i in 0..3 {
                assert!(
                    pt[i].abs() <= 1.0 + 1e-6,
                    "xyz component should be in [-1,1]"
                );
            }
        }
    }
    #[test]
    fn test_cross_section_tilted_slice_changes_count() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(1.0, 0.0, 0.0, 1.0).normalize();
        let slice_origin = Vector4::new(0.3, 0.0, 0.0, 0.3);
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        for pt in &cross {
            let d = (pt - slice_origin).dot(&slice_normal);
            assert_approx_eq(d, 0.0, 1e-4);
        }
    }
    #[test]
    fn test_in_band_segments_default_slice() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let w_half = 1.25;
        let segments =
            compute_in_band_segments(&vertices, &indices, slice_normal, slice_origin, w_half);
        assert!(
            segments.len() >= TESSERACT_CROSS_SECTION_EDGE_COUNT,
            "w=0 slice at w_half=1.25 should have >= {} edges, got {}",
            TESSERACT_CROSS_SECTION_EDGE_COUNT,
            segments.len()
        );
        let fully_in_count = segments.iter().filter(|s| s.fully_in).count();
        assert!(
            fully_in_count >= 4,
            "at least 4 edges should be fully in the slice band, got {}",
            fully_in_count
        );
    }
    #[test]
    fn test_in_band_segments_edges_lie_within_band() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let w_half = 1.25;
        let segments =
            compute_in_band_segments(&vertices, &indices, slice_normal, slice_origin, w_half);
        for seg in &segments {
            for p in &[seg.p0, seg.p1] {
                let d = (p - slice_origin).dot(&slice_normal);
                assert!(
                    d.abs() <= w_half + 1e-6,
                    "in-band segment endpoint should be within band: d={}, w_half={}",
                    d,
                    w_half
                );
            }
        }
    }
    #[test]
    fn test_convex_hull_square() {
        let pts = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(1.0, 1.0),
            egui::Pos2::new(0.0, 1.0),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 4);
        for pt in &pts {
            assert!(hull.contains(pt), "hull should contain {:?}", pt);
        }
    }
    #[test]
    fn test_convex_hull_with_interior_points() {
        let pts = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(2.0, 0.0),
            egui::Pos2::new(2.0, 2.0),
            egui::Pos2::new(0.0, 2.0),
            egui::Pos2::new(1.0, 1.0),
            egui::Pos2::new(0.5, 0.5),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(
            hull.len(),
            4,
            "interior points should be excluded from hull"
        );
        assert!(
            !hull.contains(&egui::Pos2::new(1.0, 1.0)),
            "interior point should not be in hull"
        );
        assert!(
            !hull.contains(&egui::Pos2::new(0.5, 0.5)),
            "interior point should not be in hull"
        );
    }
    #[test]
    fn test_convex_hull_triangle() {
        let pts = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(0.5, 1.0),
            egui::Pos2::new(0.5, 0.3),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(hull.len(), 3);
    }
    #[test]
    fn test_convex_hull_collinear() {
        let pts = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(2.0, 0.0),
        ];
        let hull = convex_hull_2d(&pts);
        assert_eq!(
            hull.len(),
            2,
            "collinear points should produce a degenerate hull"
        );
    }
    #[test]
    fn test_convex_hull_preserves_count() {
        let proj = make_projector();
        let pts = vec![
            Vector3::new(0.5, 0.5, 1.0),
            Vector3::new(-0.5, 0.5, 1.0),
            Vector3::new(-0.5, -0.5, 1.0),
            Vector3::new(0.5, -0.5, 1.0),
        ];
        assert_eq!(convex_hull_screen(&pts, &proj).len(), 4);
    }
    #[test]
    fn test_near_margin_value() {
        assert!(NEAR_MARGIN > 0.3);
    }
    #[test]
    fn test_tesseract_edge_and_cross_section_counts() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        assert_eq!(indices.len() / 2, TESSERACT_EDGE_COUNT);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        assert_eq!(cross.len(), TESSERACT_CROSS_SECTION_VERTEX_COUNT);
        let mut edge_count = 0usize;
        for i in 0..cross.len() {
            for j in (i + 1)..cross.len() {
                let diff_count = (0..3)
                    .filter(|&k| (cross[i][k] - cross[j][k]).abs() > 0.5)
                    .count();
                if diff_count == 1 {
                    edge_count += 1;
                }
            }
        }
        assert_eq!(edge_count, TESSERACT_CROSS_SECTION_EDGE_COUNT);
    }
    #[test]
    fn test_zero_w_slice_vertices_form_cube() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let mut distances = std::collections::HashSet::new();
        for i in 0..cross.len() {
            for j in (i + 1)..cross.len() {
                let d = (cross[i] - cross[j]).norm();
                let rounded = (d * 1000.0).round() as i64;
                distances.insert(rounded);
            }
        }
        assert!(
            distances.len() <= 3,
            "cube should have at most 3 distinct edge lengths (side, face diagonal, space diagonal), got {}",
            distances.len()
        );
        let mut side_count = 0usize;
        for i in 0..cross.len() {
            for v in cross.iter().skip(i + 1) {
                let d = (cross[i] - v).norm();
                let rounded = (d * 1000.0).round();
                if rounded == 2000.0 {
                    side_count += 1;
                }
            }
        }
        let edges_per_vertex =
            2.0 * side_count as f32 / TESSERACT_CROSS_SECTION_VERTEX_COUNT as f32;
        assert!(
            (edges_per_vertex - 3.0).abs() < 0.1,
            "each cube vertex should have degree 3, got {}",
            edges_per_vertex
        );
    }
    #[test]
    fn test_8_cell_structure_invariants() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        assert_eq!(vertices.len(), 16);
        assert_eq!(indices.len() / 2, TESSERACT_EDGE_COUNT);
        let mut degrees = vec![0u16; vertices.len()];
        for chunk in indices.chunks(2) {
            if chunk.len() == 2 {
                degrees[chunk[0] as usize] += 1;
                degrees[chunk[1] as usize] += 1;
            }
        }
        for (i, &d) in degrees.iter().enumerate() {
            assert_eq!(d, 4, "tesseract vertex {} should have degree 4", i);
        }
    }
    fn snap_point(p: Vector4<f32>, resolution: f32) -> [i64; 4] {
        [
            (p[0] * resolution).round() as i64,
            (p[1] * resolution).round() as i64,
            (p[2] * resolution).round() as i64,
            (p[3] * resolution).round() as i64,
        ]
    }
    fn make_4d_rotated_camera() -> Camera {
        let mut cam = Camera::new();
        let rot = Rotation4D::from_6_plane_angles(0.37, -0.21, 0.44, 0.29, -0.18, 0.53);
        cam.rotation_4d = rot;
        cam
    }
    #[test]
    fn test_cross_section_edges_from_faces_w0_is_cube() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        assert_eq!(
            cs_edges.len(),
            TESSERACT_CROSS_SECTION_EDGE_COUNT,
            "w=0 cross-section should have 12 edges"
        );
        let mut vertex_counts: std::collections::HashMap<[i64; 4], u32> =
            std::collections::HashMap::new();
        let resolution = 1000.0;
        for [p0, p1] in &cs_edges {
            *vertex_counts
                .entry(snap_point(*p0, resolution))
                .or_insert(0) += 1;
            *vertex_counts
                .entry(snap_point(*p1, resolution))
                .or_insert(0) += 1;
        }
        assert_eq!(
            vertex_counts.len(),
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            "w=0 cross-section should have 8 unique vertices"
        );
        for (key, &deg) in &vertex_counts {
            assert_eq!(
                deg, 3,
                "cube vertex {:?} should have degree 3, got {}",
                key, deg
            );
        }
    }
    #[test]
    fn test_cross_section_edges_match_hull_under_4d_map_rotation() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let map_camera = make_4d_rotated_camera();
        let map_transform = MapViewTransform::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_screen: Vec<egui::Pos2> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project_to_3d(*p);
                if p3.z > near_z {
                    proj.project_3d(p3.x, p3.y, p3.z)
                } else {
                    None
                }
            })
            .map(|p| p.screen_pos)
            .collect();
        for [p0, p1] in &cs_edges {
            let s0 = map_transform.project_to_3d(*p0);
            let s1 = map_transform.project_to_3d(*p1);
            if s0.z <= near_z || s1.z <= near_z {
                continue;
            }
            let Some(sp0) = proj.project_3d(s0.x, s0.y, s0.z) else {
                continue;
            };
            let Some(sp1) = proj.project_3d(s1.x, s1.y, s1.z) else {
                continue;
            };
            let mut found0 = false;
            let mut found1 = false;
            for &cp in &cross_screen {
                if (cp - sp0.screen_pos).length() < 1.0 {
                    found0 = true;
                }
                if (cp - sp1.screen_pos).length() < 1.0 {
                    found1 = true;
                }
            }
            assert!(
                found0,
                "edge endpoint {:?} should match a cross-section screen point (4D rotated map)",
                sp0.screen_pos
            );
            assert!(
                found1,
                "edge endpoint {:?} should match a cross-section screen point (4D rotated map)",
                sp1.screen_pos
            );
        }
    }
    #[test]
    fn test_cross_section_edges_with_tilted_slice() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(1.0, 0.5, -0.3, 1.0).normalize();
        let slice_origin = Vector4::new(0.15, -0.1, 0.05, 0.2);
        let cross = compute_slice_cross_section(&vertices, &_indices, slice_normal, slice_origin);
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        assert!(
            cross.len() >= 4,
            "tilted slice should produce >= 4 vertices, got {}",
            cross.len()
        );
        assert!(
            cs_edges.len() >= 6,
            "tilted slice should produce >= 6 edges, got {}",
            cs_edges.len()
        );
        let resolution = 1000.0;
        let mut vertex_degrees: std::collections::HashMap<[i64; 4], u32> =
            std::collections::HashMap::new();
        for [p0, p1] in &cs_edges {
            *vertex_degrees
                .entry(snap_point(*p0, resolution))
                .or_insert(0) += 1;
            *vertex_degrees
                .entry(snap_point(*p1, resolution))
                .or_insert(0) += 1;
        }
        for (key, &deg) in &vertex_degrees {
            assert!(
                deg >= 3,
                "tilted slice vertex {:?} should have degree >= 3, got {}",
                key,
                deg
            );
        }
    }
    #[test]
    fn test_cross_section_edges_project_consistently_with_map_transform() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.7, -0.3, 0.5, 1.0).normalize();
        let slice_origin = Vector4::new(0.1, -0.05, 0.2, 0.15);
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let map_camera = make_4d_rotated_camera();
        let map_transform = MapViewTransform::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_screen: Vec<egui::Pos2> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project_to_3d(*p);
                if p3.z > near_z {
                    proj.project_3d(p3.x, p3.y, p3.z)
                } else {
                    None
                }
            })
            .map(|p| p.screen_pos)
            .collect();
        for [p0, p1] in &cs_edges {
            let s0 = map_transform.project_to_3d(*p0);
            let s1 = map_transform.project_to_3d(*p1);
            if s0.z <= near_z || s1.z <= near_z {
                continue;
            }
            let Some(sp0) = proj.project_3d(s0.x, s0.y, s0.z) else {
                continue;
            };
            let Some(sp1) = proj.project_3d(s1.x, s1.y, s1.z) else {
                continue;
            };
            let mut found0 = false;
            let mut found1 = false;
            for &cp in &cross_screen {
                if (cp - sp0.screen_pos).length() < 2.0 {
                    found0 = true;
                }
                if (cp - sp1.screen_pos).length() < 2.0 {
                    found1 = true;
                }
            }
            assert!(
                found0,
                "edge endpoint screen {:?} should match a cross-section screen point (tilted slice + 4D map)",
                sp0.screen_pos
            );
            assert!(
                found1,
                "edge endpoint screen {:?} should match a cross-section screen point (tilted slice + 4D map)",
                sp1.screen_pos
            );
        }
    }
}
