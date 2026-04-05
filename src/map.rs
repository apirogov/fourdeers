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
    draw_background, draw_center_divider, render_stereo_views, split_stereo_views,
    CompassFrameMode, FourDSettings, ObjectRotationAngles, ProjectionMode, StereoProjector,
    StereoSettings, TesseractRenderConfig, TesseractRenderContext,
};
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::{compute_component_color, format_magnitude, TetrahedronGadget};
use crate::toy::CompassWaypoint;

const BOUNDS_PADDING_FACTOR: f32 = 0.2;
const SLICE_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
const DIM_GRAY: egui::Color32 = egui::Color32::from_rgb(200, 200, 210);
const MAP_CAMERA_BACK_OFFSET: f32 = 4.0;
const NEAR_MARGIN: f32 = 0.5;
const TETRA_SCALE_WAYPOINT: f32 = 0.15;
const TETRA_SCALE_CAMERA: f32 = 0.2;
const EDGE_STROKE_WIDTH: f32 = 2.5;
const TAP_RADIUS_MULTIPLIER: f32 = 5.0;
const TAP_RADIUS_MIN: f32 = 15.0;
const TAP_RADIUS_MAX: f32 = 50.0;
const VISIBILITY_DARK_GREEN: egui::Color32 = egui::Color32::from_rgb(30, 120, 30);
const FRUSTUM_FAR_DISTANCE: f32 = 5.0;
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

fn visibility_dark_green_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(30, 120, 30, 50)
}

pub struct MapRenderer {
    camera: Camera,
    tesseract_vertices: Vec<crate::polytopes::Vertex4D>,
    tesseract_indices: Vec<u16>,
    w_thickness: f32,
    w_color_intensity: f32,
    projection_distance: f32,
    labels_visible: bool,
    waypoint_tap_zones: Vec<(egui::Pos2, egui::Pos2, f32, usize)>,
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
            waypoint_tap_zones: Vec::new(),
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
        &mut self,
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
                self.render_slice_volume(painter, projector, scene_camera, &bounds, rect, stereo);
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
        let (left_rect, right_rect) = split_stereo_views(rect);
        let scale = rect.height().min(rect.width() * 0.5) * 0.35;
        let left_projector = StereoProjector::for_eye(
            left_rect.center(),
            scale,
            stereo.eye_separation,
            stereo.projection_distance,
            ProjectionMode::Perspective,
            -1.0,
        );
        let right_projector = StereoProjector::for_eye(
            right_rect.center(),
            scale,
            stereo.eye_separation,
            stereo.projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );
        self.compute_waypoint_tap_zones(
            &left_projector,
            &right_projector,
            scene_camera,
            waypoints,
            &bounds,
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
        rect: egui::Rect,
        stereo: StereoSettings,
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
        let screen_pts = if cross_section_3d.len() >= 3 {
            convex_hull_screen(&cross_section_3d, projector)
        } else {
            Vec::new()
        };
        if screen_pts.len() >= 3 {
            painter.add(egui::Shape::convex_polygon(
                screen_pts.clone(),
                slice_green_fill(),
                egui::Stroke::new(1.5, SLICE_GREEN),
            ));

            let cam_3d = map_transform.project_to_3d(norm_cam);
            if cam_3d.z > near_z {
                if let Some(cam_screen) = projector.project_3d(cam_3d.x, cam_3d.y, cam_3d.z) {
                    let (tan_half_fov_x, tan_half_fov_y) =
                        compute_frustum_half_angles(rect, stereo.projection_distance);
                    let right3 = scene_camera.right_vector();
                    let up3 = scene_camera.up_vector();
                    let forward3 = scene_camera.forward_vector();

                    let corner_dirs_3d = [
                        right3 * tan_half_fov_x + up3 * tan_half_fov_y + forward3,
                        -right3 * tan_half_fov_x + up3 * tan_half_fov_y + forward3,
                        -right3 * tan_half_fov_x - up3 * tan_half_fov_y + forward3,
                        right3 * tan_half_fov_x - up3 * tan_half_fov_y + forward3,
                    ];

                    let mut frustum_screen = [egui::Pos2::ZERO; 4];
                    let mut all_valid = true;
                    for (i, dir_3d) in corner_dirs_3d.iter().enumerate() {
                        let dir_4d = scene_camera.project_camera_3d_to_world_4d(*dir_3d);
                        let far_world = scene_camera.position + dir_4d * FRUSTUM_FAR_DISTANCE;
                        let far_tess = normalize_to_tesseract(far_world, bounds);
                        let far_3d = map_transform.project_to_3d(far_tess);
                        if let Some(fp) = projector.project_3d(far_3d.x, far_3d.y, far_3d.z) {
                            frustum_screen[i] = fp.screen_pos;
                        } else {
                            all_valid = false;
                            break;
                        }
                    }

                    if all_valid {
                        let visibility = clip_polygon_to_frustum_cone(
                            &screen_pts,
                            cam_screen.screen_pos,
                            &frustum_screen,
                        );
                        if visibility.len() >= 3 {
                            painter.add(egui::Shape::convex_polygon(
                                visibility,
                                visibility_dark_green_fill(),
                                egui::Stroke::new(1.0, VISIBILITY_DARK_GREEN),
                            ));
                        }
                    }
                }
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
            let (edge_color, alpha) = slice_info.style_for_point(wp.position);
            let gadget =
                TetrahedronGadget::from_4d_vector_with_scale(vector_4d, TETRA_SCALE_WAYPOINT)
                    .with_tip_label(wp.title);
            let Some(center_screen) = projector.project_3d(s3d.x, s3d.y, s3d.z) else {
                continue;
            };
            let dist = (wp.position - scene_camera.position).norm();
            let dist_label = format!("({})", format_magnitude(dist));
            render_tetrahedron_with_projector(
                painter,
                &gadget,
                projector,
                frame_mode,
                edge_color,
                alpha,
                s3d,
                Some(&dist_label),
                self.labels_visible,
            );
            {
                let dot_color = egui::Color32::from_rgba_unmultiplied(
                    edge_color.r(),
                    edge_color.g(),
                    edge_color.b(),
                    (alpha * 200.0) as u8,
                );
                painter.circle_filled(center_screen.screen_pos, 3.0, dot_color);
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
        let (edge_color, alpha) = slice_info.style_for_point(scene_camera.position);
        let gadget = TetrahedronGadget::from_4d_vector_with_scale(vector_4d, TETRA_SCALE_CAMERA)
            .with_tip_label("Cam");
        let Some(center_screen) = projector.project_3d(s3d.x, s3d.y, s3d.z) else {
            return;
        };
        render_tetrahedron_with_projector(
            painter,
            &gadget,
            projector,
            frame_mode,
            edge_color,
            alpha,
            s3d,
            None,
            self.labels_visible,
        );
        let dot_alpha = (alpha * 255.0) as u8;
        painter.circle_filled(
            center_screen.screen_pos,
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, dot_alpha),
        );
    }
    fn compute_waypoint_tap_zones(
        &mut self,
        left_projector: &StereoProjector,
        right_projector: &StereoProjector,
        _scene_camera: &Camera,
        waypoints: &[CompassWaypoint],
        bounds: &(Vector4<f32>, Vector4<f32>),
    ) {
        self.waypoint_tap_zones.clear();
        let map_transform = MapViewTransform::new(&self.camera);
        for (idx, wp) in waypoints.iter().enumerate() {
            let norm_pos = normalize_to_tesseract(wp.position, bounds);
            let s3d = map_transform.project_to_3d(norm_pos);
            if s3d.z <= -self.projection_distance {
                continue;
            }
            let Some(left_p) = left_projector.project_3d(s3d.x, s3d.y, s3d.z) else {
                continue;
            };
            let Some(right_p) = right_projector.project_3d(s3d.x, s3d.y, s3d.z) else {
                continue;
            };
            let z_offset = self.projection_distance + s3d.z;
            if z_offset <= 0.1 {
                continue;
            }
            let projected_size = TETRA_SCALE_WAYPOINT * left_projector.scale() / z_offset;
            let tap_radius =
                (projected_size * TAP_RADIUS_MULTIPLIER).clamp(TAP_RADIUS_MIN, TAP_RADIUS_MAX);
            self.waypoint_tap_zones
                .push((left_p.screen_pos, right_p.screen_pos, tap_radius, idx));
        }
    }
    pub fn find_tapped_waypoint(&self, tap_pos: egui::Pos2) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for &(left_pos, right_pos, radius, wp_index) in &self.waypoint_tap_zones {
            let dist_left = (tap_pos - left_pos).length();
            let dist_right = (tap_pos - right_pos).length();
            let dist = dist_left.min(dist_right);
            if dist <= radius && (best.is_none() || dist < best.unwrap().1) {
                best = Some((wp_index, dist));
            }
        }
        best.map(|(idx, _)| idx)
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let ar = a.r() as f32;
    let ag = a.g() as f32;
    let ab = a.b() as f32;
    let br = b.r() as f32;
    let bg = b.g() as f32;
    let bb = b.b() as f32;
    egui::Color32::from_rgb(
        (ar + (br - ar) * t) as u8,
        (ag + (bg - ag) * t) as u8,
        (ab + (bb - ab) * t) as u8,
    )
}

struct SliceInfo {
    slice_normal: Vector4<f32>,
    cam_position: Vector4<f32>,
    w_half: f32,
}
impl SliceInfo {
    fn new(
        scene_camera: &Camera,
        _bounds: &(Vector4<f32>, Vector4<f32>),
        _map_camera: &Camera,
        w_thickness: f32,
    ) -> Self {
        let slice_rotation = Rotation4D::new(
            UnitQuaternion::identity(),
            *scene_camera.rotation_4d.q_right(),
        );
        let basis_w = slice_rotation.basis_w();
        let slice_normal = Vector4::new(basis_w[0], basis_w[1], basis_w[2], basis_w[3]);
        Self {
            slice_normal,
            cam_position: scene_camera.position,
            w_half: w_thickness * 0.5,
        }
    }
    fn style_for_point(&self, world_pos: Vector4<f32>) -> (egui::Color32, f32) {
        let d = (world_pos - self.cam_position).dot(&self.slice_normal);
        let abs_d = d.abs();
        if abs_d <= self.w_half {
            (SLICE_GREEN, 1.0)
        } else if abs_d < 2.0 * self.w_half {
            let t = ((abs_d - self.w_half) / self.w_half).clamp(0.0, 1.0);
            let alpha = 1.0 - t * 0.7;
            let edge_color = lerp_color(SLICE_GREEN, DIM_GRAY, t);
            (edge_color, alpha)
        } else {
            (DIM_GRAY, 0.3)
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn render_tetrahedron_with_projector(
    painter: &egui::Painter,
    gadget: &TetrahedronGadget,
    projector: &StereoProjector,
    frame_mode: CompassFrameMode,
    edge_color: egui::Color32,
    alpha: f32,
    center_3d: Vector3<f32>,
    distance_label: Option<&str>,
    labels_visible: bool,
) {
    let edge_stroke_color = egui::Color32::from_rgba_unmultiplied(
        edge_color.r(),
        edge_color.g(),
        edge_color.b(),
        (alpha * 200.0) as u8,
    );
    for edge in &gadget.edges {
        let v0 = gadget
            .get_vertex_3d(edge.vertex_indices[0])
            .unwrap()
            .to_vector3()
            + center_3d;
        let v1 = gadget
            .get_vertex_3d(edge.vertex_indices[1])
            .unwrap()
            .to_vector3()
            + center_3d;
        if let (Some(p0), Some(p1)) = (
            projector.project_3d(v0.x, v0.y, v0.z),
            projector.project_3d(v1.x, v1.y, v1.z),
        ) {
            painter.line_segment(
                [p0.screen_pos, p1.screen_pos],
                egui::Stroke::new(EDGE_STROKE_WIDTH, edge_stroke_color),
            );
        }
    }
    if labels_visible {
        let component_mags: [f32; 4] = gadget.component_values.map(|v| v.abs());
        let max_mag = component_mags.iter().cloned().fold(0.0f32, f32::max);
        for (i, vertex) in gadget.vertices.iter().enumerate() {
            let component = gadget.component_values[i];
            let color = compute_component_color(component, max_mag);
            if let (Some(pos), Some(normal)) =
                (gadget.get_vertex_3d(i), gadget.get_vertex_normal(i))
            {
                let pos_v = pos.to_vector3();
                let normal_v = normal.to_vector3();
                let label_pos = pos_v + normal_v * 0.12 + center_3d;
                let pos_c = pos_v + center_3d;
                if let Some(label_p) = projector.project_3d(label_pos.x, label_pos.y, pos_c.z) {
                    let vertex_label = crate::render::compass_vertex_label(
                        frame_mode,
                        i,
                        component,
                        &vertex.label,
                    );
                    let font_id = egui::FontId::monospace(10.0);
                    let a = (alpha * 230.0) as u8;
                    let text_color =
                        egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, a);
                    let outline = egui::Color32::from_rgba_unmultiplied(
                        edge_color.r(),
                        edge_color.g(),
                        edge_color.b(),
                        (alpha * 120.0) as u8,
                    );
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
    }

    let arrow = gadget.arrow_position().to_vector3() + center_3d;
    let origin = center_3d;
    let arrow_screen = projector.project_3d(arrow.x, arrow.y, arrow.z);
    let origin_screen = projector.project_3d(origin.x, origin.y, origin.z);
    if let (Some(arrow_p), Some(origin_p)) = (arrow_screen, origin_screen) {
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
        if let Some(tip_p) = arrow_screen {
            let a = (alpha * 230.0) as u8;
            painter.text(
                tip_p.screen_pos + egui::Vec2::new(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                label,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgba_unmultiplied(255, 180, 80, a),
            );
        }
    }
    if let Some(dist) = distance_label {
        if let Some(base_p) = origin_screen {
            let a = (alpha * 200.0) as u8;
            painter.text(
                base_p.screen_pos + egui::Vec2::new(0.0, 12.0),
                egui::Align2::CENTER_TOP,
                dist,
                egui::FontId::proportional(8.0),
                egui::Color32::from_rgba_unmultiplied(200, 200, 220, a),
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

    #[allow(dead_code)]
    fn direction_to_3d(&self, dir_4d: Vector4<f32>) -> Vector3<f32> {
        let r = self.mat_4d * dir_4d;
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

fn compute_frustum_half_angles(rect: egui::Rect, projection_distance: f32) -> (f32, f32) {
    let scale = rect.height().min(rect.width() * 0.5) * 0.35;
    let half_width = rect.width() * 0.25;
    let half_height = rect.height() * 0.5;
    let tan_half_fov_x = half_width / (scale * projection_distance);
    let tan_half_fov_y = half_height / (scale * projection_distance);
    (tan_half_fov_x, tan_half_fov_y)
}

fn clip_polygon_by_half_plane(
    polygon: &[egui::Pos2],
    edge_start: egui::Pos2,
    edge_end: egui::Pos2,
) -> Vec<egui::Pos2> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let cross_2d = |a: egui::Pos2, b: egui::Pos2, p: egui::Pos2| -> f32 {
        let ab = b - a;
        let ap = p - a;
        ab.x * ap.y - ab.y * ap.x
    };
    let is_inside = |p: egui::Pos2| cross_2d(edge_start, edge_end, p) >= 0.0;
    let n = polygon.len();
    let mut output = Vec::with_capacity(n + 1);
    for i in 0..n {
        let curr = polygon[i];
        let next = polygon[(i + 1) % n];
        let curr_inside = is_inside(curr);
        let next_inside = is_inside(next);
        if curr_inside {
            output.push(curr);
        }
        if curr_inside != next_inside {
            let dc = cross_2d(edge_start, edge_end, curr);
            let dn = cross_2d(edge_start, edge_end, next);
            let denom = dc - dn;
            if denom.abs() > 1e-12 {
                let t = dc / denom;
                let intersection = curr + (next - curr) * t;
                output.push(intersection);
            }
        }
    }
    output
}

fn clip_polygon_to_frustum_cone(
    polygon: &[egui::Pos2],
    cam_screen: egui::Pos2,
    frustum_corners: &[egui::Pos2; 4],
) -> Vec<egui::Pos2> {
    let centroid = egui::Pos2::new(
        (frustum_corners[0].x + frustum_corners[1].x + frustum_corners[2].x + frustum_corners[3].x)
            * 0.25,
        (frustum_corners[0].y + frustum_corners[1].y + frustum_corners[2].y + frustum_corners[3].y)
            * 0.25,
    );
    let mut result = polygon.to_vec();
    for i in 0..4 {
        let edge_start = cam_screen;
        let edge_end = frustum_corners[i];
        let next = frustum_corners[(i + 1) % 4];
        let cross_centroid = {
            let ab = edge_end - edge_start;
            let ac = centroid - edge_start;
            ab.x * ac.y - ab.y * ac.x
        };
        let cross_next = {
            let ab = edge_end - edge_start;
            let an = next - edge_start;
            ab.x * an.y - ab.y * an.x
        };
        if cross_centroid >= 0.0 && cross_next >= 0.0 {
            result = clip_polygon_by_half_plane(&result, edge_start, edge_end);
        } else {
            result = clip_polygon_by_half_plane(&result, edge_end, edge_start);
        }
        if result.is_empty() {
            return Vec::new();
        }
    }
    result
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
    fn test_style_for_point_at_camera_is_in_slab() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let (color, alpha) = info.style_for_point(cam.position);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_in_slab_small_w_offset() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let pos_nearby = cam.position + Vector4::new(0.0, 0.0, 0.0, 0.5);
        let (color, alpha) = info.style_for_point(pos_nearby);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_near_slab_lerps() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let w_half = 1.25;
        let pos_near = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half + 0.5 * w_half);
        let (color, alpha) = info.style_for_point(pos_near);
        assert_ne!(color, SLICE_GREEN);
        assert_ne!(color, DIM_GRAY);
        assert!(
            alpha > 0.3 && alpha < 1.0,
            "alpha should be between 0.3 and 1.0 for near-slab, got {}",
            alpha
        );
    }

    #[test]
    fn test_style_for_point_far_from_slab() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let pos_far = cam.position + Vector4::new(0.0, 0.0, 0.0, 20.0);
        let (color, alpha) = info.style_for_point(pos_far);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_far_negative_w() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let pos_far_neg = cam.position + Vector4::new(0.0, 0.0, 0.0, -20.0);
        let (color, alpha) = info.style_for_point(pos_far_neg);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_boundary_at_w_half() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, &bounds, &map_cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let pos_boundary = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half);
        let (color, alpha) = info.style_for_point(pos_boundary);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_boundary_at_2w_half() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, &bounds, &map_cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let pos_boundary = cam.position + Vector4::new(0.0, 0.0, 0.0, 2.0 * w_half);
        let (color, alpha) = info.style_for_point(pos_boundary);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_with_tilted_slice() {
        let mut cam = Camera::new();
        cam.rotation_4d = Rotation4D::from_6_plane_angles(0.0, 0.0, 0.0, 0.5, 0.0, 0.0);
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let info = SliceInfo::new(&cam, &bounds, &map_cam, 2.5);
        let (color, alpha) = info.style_for_point(cam.position);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
        let pos_far = cam.position + Vector4::new(0.0, 0.0, 0.0, 20.0);
        let (color_far, alpha_far) = info.style_for_point(pos_far);
        assert_eq!(color_far, DIM_GRAY);
        assert_approx_eq(alpha_far, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_lerp_is_continuous() {
        let cam = Camera::new();
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let map_cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, &bounds, &map_cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let epsilon = 0.01;
        let pos_just_inside = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half - epsilon);
        let pos_just_outside = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half + epsilon);
        let (color_in, alpha_in) = info.style_for_point(pos_just_inside);
        let (color_out, alpha_out) = info.style_for_point(pos_just_outside);
        let color_dist = ((color_in.r() as i32 - color_out.r() as i32).abs()
            + (color_in.g() as i32 - color_out.g() as i32).abs()
            + (color_in.b() as i32 - color_out.b() as i32).abs()) as f32;
        assert!(
            color_dist < 15.0,
            "color should be nearly continuous at w_half boundary, distance={}",
            color_dist
        );
        assert!(
            (alpha_in - alpha_out).abs() < 0.1,
            "alpha should be nearly continuous at w_half boundary: in={}, out={}",
            alpha_in,
            alpha_out
        );
    }

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

    #[test]
    fn test_normalize_to_tesseract_center() {
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let center = normalize_to_tesseract(Vector4::new(0.0, 0.0, 0.0, 0.0), &bounds);
        for i in 0..4 {
            assert_approx_eq(center[i], 0.0, 1e-6);
        }
    }

    #[test]
    fn test_normalize_to_tesseract_corners() {
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let min_corner = normalize_to_tesseract(bounds.0, &bounds);
        let max_corner = normalize_to_tesseract(bounds.1, &bounds);
        for i in 0..4 {
            assert_approx_eq(min_corner[i], -1.0, 1e-6);
            assert_approx_eq(max_corner[i], 1.0, 1e-6);
        }
    }

    #[test]
    fn test_normalize_to_tesseract_asymmetric_bounds() {
        let bounds = (
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(10.0, 10.0, 10.0, 10.0),
        );
        let result = normalize_to_tesseract(Vector4::new(5.0, 0.0, 10.0, 2.5), &bounds);
        assert_approx_eq(result[0], 0.0, 1e-6);
        assert_approx_eq(result[1], -1.0, 1e-6);
        assert_approx_eq(result[2], 1.0, 1e-6);
        assert_approx_eq(result[3], -0.5, 1e-6);
    }

    #[test]
    fn test_find_tapped_waypoint_no_zones() {
        let renderer = MapRenderer::new();
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(100.0, 100.0)),
            None
        );
    }

    #[test]
    fn test_find_tapped_waypoint_hit_left_eye() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            20.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(105.0, 205.0)),
            Some(0)
        );
    }

    #[test]
    fn test_find_tapped_waypoint_hit_right_eye() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            20.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(305.0, 205.0)),
            Some(0)
        );
    }

    #[test]
    fn test_find_tapped_waypoint_miss() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            10.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(50.0, 50.0)),
            None
        );
    }

    #[test]
    fn test_find_tapped_waypoint_closest_wins() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![
            (
                egui::Pos2::new(100.0, 100.0),
                egui::Pos2::new(100.0, 100.0),
                30.0,
                0,
            ),
            (
                egui::Pos2::new(115.0, 100.0),
                egui::Pos2::new(115.0, 100.0),
                30.0,
                1,
            ),
        ];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(112.0, 100.0)),
            Some(1)
        );
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

    #[test]
    fn test_direction_to_3d_no_offset() {
        let mut cam = Camera::new();
        cam.position = Vector4::zeros();
        let mt = MapViewTransform::new(&cam);
        let dir = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let pos_result = mt.project_to_3d(dir);
        let dir_result = mt.direction_to_3d(dir);
        assert_approx_eq(pos_result.x, dir_result.x, 1e-6);
        assert_approx_eq(pos_result.y, dir_result.y, 1e-6);
        assert_approx_eq(pos_result.z, dir_result.z, 1e-6);
    }

    #[test]
    fn test_compute_frustum_half_angles() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 400.0));
        let proj_dist = 3.0;
        let (tan_x, tan_y) = compute_frustum_half_angles(rect, proj_dist);
        assert!(tan_x > 0.0, "tan_half_fov_x should be positive");
        assert!(tan_y > 0.0, "tan_half_fov_y should be positive");
        assert!(
            tan_x < tan_y,
            "tan_x should be less than tan_y (stereo split halves width)"
        );
    }

    #[test]
    fn test_clip_polygon_by_half_plane_square() {
        let square = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(1.0, 1.0),
            egui::Pos2::new(0.0, 1.0),
        ];
        let result = clip_polygon_by_half_plane(
            &square,
            egui::Pos2::new(0.5, 1.0),
            egui::Pos2::new(0.5, 0.0),
        );
        assert!(
            result.len() >= 3,
            "clipped polygon should have >= 3 vertices"
        );
        for p in &result {
            assert!(
                p.x >= 0.5 - 1e-6,
                "point {:?} should be on right side of x=0.5",
                p
            );
        }
    }

    #[test]
    fn test_clip_polygon_by_half_plane_triangle() {
        let tri = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(2.0, 0.0),
            egui::Pos2::new(1.0, 2.0),
        ];
        let result =
            clip_polygon_by_half_plane(&tri, egui::Pos2::new(1.0, 1.0), egui::Pos2::new(1.0, 0.0));
        assert!(
            result.len() >= 3,
            "clipped triangle should have >= 3 vertices"
        );
        for p in &result {
            assert!(
                p.x >= 1.0 - 1e-6,
                "point {:?} should be on right side of x=1.0",
                p
            );
        }
    }

    #[test]
    fn test_clip_polygon_empty_result() {
        let square = vec![
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(1.0, 1.0),
            egui::Pos2::new(0.0, 1.0),
        ];
        let result = clip_polygon_by_half_plane(
            &square,
            egui::Pos2::new(2.0, 1.0),
            egui::Pos2::new(2.0, 0.0),
        );
        assert!(
            result.is_empty(),
            "polygon entirely to the left should be clipped away"
        );
    }

    #[test]
    fn test_clip_polygon_unchanged() {
        let square = vec![
            egui::Pos2::new(2.0, 0.0),
            egui::Pos2::new(3.0, 0.0),
            egui::Pos2::new(3.0, 1.0),
            egui::Pos2::new(2.0, 1.0),
        ];
        let result = clip_polygon_by_half_plane(
            &square,
            egui::Pos2::new(0.0, 1.0),
            egui::Pos2::new(0.0, 0.0),
        );
        assert_eq!(
            result.len(),
            4,
            "polygon entirely inside should be unchanged"
        );
    }

    #[test]
    fn test_clip_polygon_to_frustum_cone_basic() {
        let cam = egui::Pos2::new(200.0, 200.0);
        let corners = [
            egui::Pos2::new(180.0, 170.0),
            egui::Pos2::new(220.0, 170.0),
            egui::Pos2::new(220.0, 230.0),
            egui::Pos2::new(180.0, 230.0),
        ];
        let polygon = vec![
            egui::Pos2::new(150.0, 150.0),
            egui::Pos2::new(250.0, 150.0),
            egui::Pos2::new(250.0, 250.0),
            egui::Pos2::new(150.0, 250.0),
        ];
        let result = clip_polygon_to_frustum_cone(&polygon, cam, &corners);
        assert!(
            result.len() >= 3,
            "partially overlapping polygon should produce >= 3 vertices"
        );
    }

    #[test]
    fn test_clip_polygon_to_frustum_cone_full_containment() {
        let cam = egui::Pos2::new(200.0, 200.0);
        let corners = [
            egui::Pos2::new(100.0, 100.0),
            egui::Pos2::new(300.0, 100.0),
            egui::Pos2::new(300.0, 300.0),
            egui::Pos2::new(100.0, 300.0),
        ];
        let polygon = vec![
            egui::Pos2::new(180.0, 180.0),
            egui::Pos2::new(220.0, 180.0),
            egui::Pos2::new(220.0, 220.0),
            egui::Pos2::new(180.0, 220.0),
        ];
        let result = clip_polygon_to_frustum_cone(&polygon, cam, &corners);
        assert_eq!(
            result.len(),
            4,
            "polygon fully inside cone should be unchanged"
        );
    }

    #[test]
    fn test_clip_polygon_to_frustum_cone_no_overlap() {
        let cam = egui::Pos2::new(200.0, 200.0);
        let corners = [
            egui::Pos2::new(190.0, 190.0),
            egui::Pos2::new(210.0, 190.0),
            egui::Pos2::new(210.0, 210.0),
            egui::Pos2::new(190.0, 210.0),
        ];
        let polygon = vec![
            egui::Pos2::new(50.0, 50.0),
            egui::Pos2::new(60.0, 50.0),
            egui::Pos2::new(60.0, 60.0),
            egui::Pos2::new(50.0, 60.0),
        ];
        let result = clip_polygon_to_frustum_cone(&polygon, cam, &corners);
        assert!(
            result.is_empty(),
            "polygon entirely outside cone should be empty"
        );
    }

    #[test]
    fn test_visibility_polygon_is_subset_of_cross_section() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_3d: Vec<Vector3<f32>> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project_to_3d(*p);
                if p3.z > near_z {
                    Some(p3)
                } else {
                    None
                }
            })
            .collect();
        let screen_pts = convex_hull_screen(&cross_3d, &proj);
        if screen_pts.len() < 3 {
            return;
        }
        let scene_camera = Camera::new();
        let norm_cam = Vector4::zeros();
        let cam_3d = map_transform.project_to_3d(norm_cam);
        if cam_3d.z <= near_z {
            return;
        }
        let Some(cam_screen) = proj.project_3d(cam_3d.x, cam_3d.y, cam_3d.z) else {
            return;
        };
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 400.0));
        let (tan_x, tan_y) = compute_frustum_half_angles(rect, 3.0);
        let right3 = scene_camera.right_vector();
        let up3 = scene_camera.up_vector();
        let forward3 = scene_camera.forward_vector();
        let corner_dirs = [
            right3 * tan_x + up3 * tan_y + forward3,
            -right3 * tan_x + up3 * tan_y + forward3,
            -right3 * tan_x - up3 * tan_y + forward3,
            right3 * tan_x - up3 * tan_y + forward3,
        ];
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let mut frustum_screen = [egui::Pos2::ZERO; 4];
        for (i, dir_3d) in corner_dirs.iter().enumerate() {
            let dir_4d = scene_camera.project_camera_3d_to_world_4d(*dir_3d);
            let far_world = scene_camera.position + dir_4d * FRUSTUM_FAR_DISTANCE;
            let far_tess = normalize_to_tesseract(far_world, &bounds);
            let far_3d = map_transform.project_to_3d(far_tess);
            if let Some(fp) = proj.project_3d(far_3d.x, far_3d.y, far_3d.z) {
                frustum_screen[i] = fp.screen_pos;
            }
        }
        let visibility =
            clip_polygon_to_frustum_cone(&screen_pts, cam_screen.screen_pos, &frustum_screen);
        if visibility.len() >= 3 {
            for p in &visibility {
                let mut inside = false;
                let n = screen_pts.len();
                for i in 0..n {
                    let j = (i + 1) % n;
                    let xi = screen_pts[i].x;
                    let yi = screen_pts[i].y;
                    let xj = screen_pts[j].x;
                    let yj = screen_pts[j].y;
                    if ((yi > p.y) != (yj > p.y)) && (p.x < (xj - xi) * (p.y - yi) / (yj - yi) + xi)
                    {
                        inside = !inside;
                    }
                }
                assert!(
                    inside,
                    "visibility point {:?} should be inside cross-section hull",
                    p
                );
            }
        }
    }

    #[test]
    fn test_visibility_polygon_with_identity_cam() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_3d: Vec<Vector3<f32>> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project_to_3d(*p);
                if p3.z > near_z {
                    Some(p3)
                } else {
                    None
                }
            })
            .collect();
        let screen_pts = convex_hull_screen(&cross_3d, &proj);
        if screen_pts.len() < 3 {
            return;
        }
        let scene_camera = Camera::new();
        let norm_cam = Vector4::zeros();
        let cam_3d = map_transform.project_to_3d(norm_cam);
        if cam_3d.z <= near_z {
            return;
        }
        let Some(cam_screen) = proj.project_3d(cam_3d.x, cam_3d.y, cam_3d.z) else {
            return;
        };
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 400.0));
        let (tan_x, tan_y) = compute_frustum_half_angles(rect, 3.0);
        let right3 = scene_camera.right_vector();
        let up3 = scene_camera.up_vector();
        let forward3 = scene_camera.forward_vector();
        let corner_dirs = [
            right3 * tan_x + up3 * tan_y + forward3,
            -right3 * tan_x + up3 * tan_y + forward3,
            -right3 * tan_x - up3 * tan_y + forward3,
            right3 * tan_x - up3 * tan_y + forward3,
        ];
        let bounds = (
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let mut frustum_screen = [egui::Pos2::ZERO; 4];
        let mut all_valid = true;
        for (i, dir_3d) in corner_dirs.iter().enumerate() {
            let dir_4d = scene_camera.project_camera_3d_to_world_4d(*dir_3d);
            let far_world = scene_camera.position + dir_4d * FRUSTUM_FAR_DISTANCE;
            let far_tess = normalize_to_tesseract(far_world, &bounds);
            let far_3d = map_transform.project_to_3d(far_tess);
            if let Some(fp) = proj.project_3d(far_3d.x, far_3d.y, far_3d.z) {
                frustum_screen[i] = fp.screen_pos;
            } else {
                all_valid = false;
                break;
            }
        }
        assert!(all_valid, "all frustum corners should project to screen");
        let visibility =
            clip_polygon_to_frustum_cone(&screen_pts, cam_screen.screen_pos, &frustum_screen);
        assert!(
            visibility.len() >= 3,
            "visibility polygon should be non-empty with identity camera, got {} points",
            visibility.len()
        );
    }
}
