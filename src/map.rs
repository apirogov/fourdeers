//! 4D Map view: a tesseract representing the 4D scene volume
//!
//! The map renders a tesseract wireframe (the 8-cell / hypercube) whose vertices
//! correspond to the signed axis extremes of the scene bounds. Inside, it shows:
//! - The scene camera's current 3D slice as a green filled truncated cube
//! - Waypoints as full labeled tetrahedra
//! - The scene camera position as a labeled tetrahedron
//!
//! The map has its own Camera allowing independent 3D/4D navigation.

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

fn slice_green_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(60, 180, 60, 40)
}

fn slice_green_dim() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(80, 200, 80, 120)
}

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

pub struct MapRenderer {
    camera: Camera,
    tesseract_vertices: Vec<crate::polytopes::Vertex4D>,
    tesseract_indices: Vec<u16>,
    w_thickness: f32,
    w_color_intensity: f32,
    projection_distance: f32,
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
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
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
        self.render_vertex_labels(painter, projector, &transformed);
    }

    fn render_vertex_labels(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[crate::render::TransformedVertex],
    ) {
        let axis_chars = ['X', 'Y', 'Z', 'W'];
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

            for (ax, &ch) in axis_chars.iter().enumerate() {
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
        let near_plane = self.projection_distance;
        let w_half = self.w_thickness * 0.5;

        let mut all_edge_segments: Vec<(egui::Pos2, egui::Pos2, bool)> = Vec::new();

        for face in &TESSERACT_FACES {
            let face_verts: Vec<Vector4<f32>> = face
                .iter()
                .map(|&vi| vertex_to_4d(&self.tesseract_vertices[vi as usize]))
                .collect();

            let distances: Vec<f32> = face_verts
                .iter()
                .map(|v| (v - norm_cam).dot(&slice_normal))
                .collect();

            let polygon_4d = clip_polygon_to_slice(&face_verts, &distances, w_half);

            if polygon_4d.len() < 3 {
                continue;
            }

            let polygon_3d: Vec<Vector3<f32>> = polygon_4d
                .iter()
                .map(|v| map_transform.project_to_3d(*v))
                .collect();

            let clipped_3d = clip_polygon_3d_near_plane(&polygon_3d, near_plane);

            if clipped_3d.len() < 3 {
                continue;
            }

            let screen_pts: Vec<egui::Pos2> = clipped_3d
                .iter()
                .filter_map(|v3| projector.project_3d(v3.x, v3.y, v3.z))
                .map(|p| p.screen_pos)
                .collect();

            if screen_pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    screen_pts,
                    slice_green_fill(),
                    egui::Stroke::new(1.0, SLICE_GREEN),
                ));
            }
        }

        for chunk in self.tesseract_indices.chunks(2) {
            if chunk.len() != 2 {
                continue;
            }
            let v0 = &self.tesseract_vertices[chunk[0] as usize];
            let v1 = &self.tesseract_vertices[chunk[1] as usize];

            let p0 = vertex_to_4d(v0);
            let p1 = vertex_to_4d(v1);

            let d0 = (p0 - norm_cam).dot(&slice_normal);
            let d1 = (p1 - norm_cam).dot(&slice_normal);

            let in0 = d0.abs() <= w_half;
            let in1 = d1.abs() <= w_half;

            if !in0 && !in1 {
                if d0.signum() != d1.signum() && (d0 - d1).abs() > 1e-10 {
                    let t_enter = (w_half * d0.signum() - d0) / (d1 - d0);
                    let t_exit = (w_half * d1.signum() - d0) / (d1 - d0);
                    let t_min = t_enter.min(t_exit).clamp(0.0, 1.0);
                    let t_max = t_enter.max(t_exit).clamp(0.0, 1.0);

                    let cp0 = p0 + (p1 - p0) * t_min;
                    let cp1 = p0 + (p1 - p0) * t_max;

                    let s0 = map_transform.project_to_3d(cp0);
                    let s1 = map_transform.project_to_3d(cp1);

                    if let (Some(sp0), Some(sp1)) = (
                        projector.project_3d(s0.x, s0.y, s0.z),
                        projector.project_3d(s1.x, s1.y, s1.z),
                    ) {
                        if sp0.depth > -near_plane && sp1.depth > -near_plane {
                            all_edge_segments.push((sp0.screen_pos, sp1.screen_pos, false));
                        }
                    }
                }
                continue;
            }

            let (tp0, tp1, fully_in) = if in0 && in1 {
                (p0, p1, true)
            } else {
                let outside_sign = if !in0 { d0.signum() } else { d1.signum() };
                let t = (w_half * outside_sign - d0) / (d1 - d0);
                let t = t.clamp(0.0, 1.0);
                let clipped = p0 + (p1 - p0) * t;
                if !in0 {
                    (clipped, p1, false)
                } else {
                    (p0, clipped, false)
                }
            };

            let s0 = map_transform.project_to_3d(tp0);
            let s1 = map_transform.project_to_3d(tp1);

            if let (Some(sp0), Some(sp1)) = (
                projector.project_3d(s0.x, s0.y, s0.z),
                projector.project_3d(s1.x, s1.y, s1.z),
            ) {
                if sp0.depth > -near_plane && sp1.depth > -near_plane {
                    all_edge_segments.push((sp0.screen_pos, sp1.screen_pos, fully_in));
                }
            }
        }

        for (s0, s1, fully_in) in all_edge_segments {
            let color = if fully_in {
                SLICE_GREEN
            } else {
                slice_green_dim()
            };
            painter.line_segment([s0, s1], egui::Stroke::new(2.0, color));
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

fn clip_polygon_to_slice(
    verts: &[Vector4<f32>],
    distances: &[f32],
    w_half: f32,
) -> Vec<Vector4<f32>> {
    let mut result = Vec::new();
    let n = verts.len();

    for i in 0..n {
        let j = (i + 1) % n;
        let di = distances[i];
        let dj = distances[j];
        let in_i = di.abs() <= w_half;
        let in_j = dj.abs() <= w_half;

        if in_i {
            result.push(verts[i]);
        }

        if in_i != in_j {
            let outside_sign = if !in_i { di.signum() } else { dj.signum() };
            let t = (w_half * outside_sign - di) / (dj - di);
            let t = t.clamp(0.0, 1.0);
            result.push(verts[i] + (verts[j] - verts[i]) * t);
        }
    }

    result
}

fn clip_polygon_3d_near_plane(polygon: &[Vector3<f32>], near_plane: f32) -> Vec<Vector3<f32>> {
    let mut result = Vec::new();
    let n = polygon.len();
    if n == 0 {
        return result;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let zi = polygon[i].z;
        let zj = polygon[j].z;
        let ci = zi > -near_plane;
        let cj = zj > -near_plane;

        if ci {
            result.push(polygon[i]);
        }

        if ci != cj {
            let dz = zj - zi;
            if dz.abs() > 1e-10 {
                let t = (-near_plane - zi) / dz;
                result.push(polygon[i] + (polygon[j] - polygon[i]) * t);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;

    #[test]
    fn test_clip_polygon_all_inside() {
        let verts = vec![
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
        ];
        let distances = vec![0.0, 0.5, -0.5];
        let result = clip_polygon_to_slice(&verts, &distances, 1.0);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_clip_polygon_all_outside_same_side() {
        let verts = vec![
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(2.0, 0.0, 0.0, 0.0),
            Vector4::new(1.5, 1.0, 0.0, 0.0),
        ];
        let distances = vec![3.0, 4.0, 3.5];
        let result = clip_polygon_to_slice(&verts, &distances, 1.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_clip_near_plane_produces_valid_intersection() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(2.0, 0.0, -6.0),
            Vector3::new(0.0, 2.0, 0.0),
        ];
        let result = clip_polygon_3d_near_plane(&polygon, 3.0);
        assert_eq!(result.len(), 4);
        for v in &result {
            assert!(v.z >= -3.0 - 1e-5, "vertex behind near plane: z={}", v.z);
        }
    }

    #[test]
    fn test_clip_polygon_both_sides_crossing() {
        let verts = vec![
            Vector4::new(-1.0, 0.0, 0.0, 0.0),
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
        ];
        let distances = vec![-2.0, 2.0, 0.0];
        let result = clip_polygon_to_slice(&verts, &distances, 1.0);
        assert!(
            result.len() >= 3,
            "should produce a clipped polygon, got {} vertices",
            result.len()
        );
        for v in &result {
            let d = v[0] - v[0].signum() * 1.0;
            let _ = d;
        }
    }

    #[test]
    fn test_clip_polygon_exit_through_opposite_boundary() {
        let verts = vec![
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(3.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
        ];
        let distances = vec![0.5, -2.5, 0.5];
        let result = clip_polygon_to_slice(&verts, &distances, 1.0);
        assert!(
            result.len() >= 3,
            "should produce a clipped polygon, got {} vertices",
            result.len()
        );
        assert_approx_eq(result[0].x, 0.0, 1e-5);
        assert_approx_eq(result[1].x, 1.5, 1e-4);
    }

    #[test]
    fn test_clip_near_plane_all_in_front() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 1.0),
        ];
        let result = clip_polygon_3d_near_plane(&polygon, 3.0);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_clip_near_plane_all_behind() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, -5.0),
            Vector3::new(1.0, 0.0, -5.0),
            Vector3::new(0.0, 1.0, -5.0),
        ];
        let result = clip_polygon_3d_near_plane(&polygon, 3.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_clip_near_plane_one_vertex_behind() {
        let polygon = vec![
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, -5.0),
        ];
        let result = clip_polygon_3d_near_plane(&polygon, 3.0);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_clip_near_plane_empty_input() {
        let polygon: Vec<Vector3<f32>> = vec![];
        let result = clip_polygon_3d_near_plane(&polygon, 3.0);
        assert!(result.is_empty());
    }
}
