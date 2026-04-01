//! 4D Map view: a tesseract representing the 4D scene volume
//!
//! The map renders a tesseract wireframe (the 8-cell / hypercube) whose vertices
//! correspond to the signed axis extremes of the scene bounds. Inside, it shows:
//! - The scene camera's current 3D slice as a green truncated cube
//! - Waypoints as small tetrahedra
//! - The scene camera position as a tetrahedron
//!
//! The map has its own Camera allowing independent 3D/4D navigation.

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::camera::{Camera, CameraAction};
use crate::colors::*;
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_views, FourDSettings, ObjectRotationAngles,
    ProjectionMode, StereoProjector, StereoSettings, TesseractRenderConfig, TesseractRenderContext,
};
use crate::rotation4d::Rotation4D;
use crate::tetrahedron::TetrahedronGadget;
use crate::toy::CompassWaypoint;

const BOUNDS_PADDING_FACTOR: f32 = 0.2;
const SLICE_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
const CAMERA_ARROW_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const MAP_CAMERA_BACK_OFFSET: f32 = 4.0;

fn slice_green_dim() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(80, 200, 80, 120)
}

fn tetra_edge_color() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(200, 200, 210, 200)
}

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

    pub fn apply_action(&mut self, action: CameraAction, speed: f32) {
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
                self.render_slice_cube(painter, projector, scene_camera, &bounds);
                self.render_waypoints(painter, projector, waypoints, &bounds);
                self.render_camera_position(painter, projector, scene_camera, &bounds);
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
        let w_half = self.w_thickness * 0.5;
        let axis_labels = ["X", "Y", "Z", "W"];

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
            let signs: [bool; 4] = [
                vertex.position[0] >= 0.0,
                vertex.position[1] >= 0.0,
                vertex.position[2] >= 0.0,
                vertex.position[3] >= 0.0,
            ];

            let pos_count = signs.iter().filter(|&&s| s).count();
            let label: String = if pos_count == 0 || pos_count == 4 {
                signs.iter().map(|&s| if s { '+' } else { '-' }).collect()
            } else {
                axis_labels
                    .iter()
                    .zip(signs.iter())
                    .filter(|(_, &positive)| !positive)
                    .map(|(ax, _)| *ax)
                    .collect::<Vec<_>>()
                    .join(",")
            };

            let normalized_w = (tv.w / w_half).clamp(-1.0, 1.0);
            let color = crate::render::w_to_color(normalized_w, 200, self.w_color_intensity);

            let font_id = egui::FontId::proportional(9.0);
            painter.text(
                p.screen_pos,
                egui::Align2::CENTER_CENTER,
                &label,
                font_id,
                color,
            );
        }
    }

    fn render_slice_cube(
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
                continue;
            }

            let (tp0, tp1, fully_in) = if in0 && in1 {
                (p0, p1, true)
            } else {
                let t = (w_half * d0.signum() - d0) / (d1 - d0);
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
                    let color = if fully_in {
                        SLICE_GREEN
                    } else {
                        slice_green_dim()
                    };
                    painter.line_segment(
                        [sp0.screen_pos, sp1.screen_pos],
                        egui::Stroke::new(2.5, color),
                    );
                }
            }
        }
    }

    fn render_waypoints(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        waypoints: &[CompassWaypoint],
        bounds: &(Vector4<f32>, Vector4<f32>),
    ) {
        let map_transform = MapViewTransform::new(&self.camera);

        for wp in waypoints {
            let norm_pos = normalize_to_tesseract(wp.position, bounds);
            let s3d = map_transform.project_to_3d(norm_pos);

            if s3d.z <= -self.projection_distance {
                continue;
            }

            let gadget = TetrahedronGadget::from_4d_vector_with_scale(Vector4::zeros(), 0.15)
                .with_tip_label(wp.title);

            render_mini_tetrahedron(painter, projector, &gadget, s3d.x, s3d.y, s3d.z);
        }
    }

    fn render_camera_position(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        scene_camera: &Camera,
        bounds: &(Vector4<f32>, Vector4<f32>),
    ) {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let map_transform = MapViewTransform::new(&self.camera);
        let s3d = map_transform.project_to_3d(norm_cam);

        if s3d.z <= -self.projection_distance {
            return;
        }

        let forward_world = scene_camera.project_camera_3d_to_world_4d(Vector3::new(0.0, 0.0, 1.0));
        let norm_forward = Vector4::new(
            forward_world[0] / bounds_extent(bounds, 0),
            forward_world[1] / bounds_extent(bounds, 1),
            forward_world[2] / bounds_extent(bounds, 2),
            forward_world[3] / bounds_extent(bounds, 3),
        );
        let norm_forward = normalize_4d(norm_forward);

        let gadget =
            TetrahedronGadget::from_4d_vector_with_scale(norm_forward, 0.2).with_tip_label("Cam");

        render_mini_tetrahedron(painter, projector, &gadget, s3d.x, s3d.y, s3d.z);

        if let Some(p) = projector.project_3d(s3d.x, s3d.y, s3d.z) {
            painter.circle_filled(p.screen_pos, 4.0, CAMERA_ARROW_COLOR);
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

fn render_mini_tetrahedron(
    painter: &egui::Painter,
    projector: &StereoProjector,
    gadget: &TetrahedronGadget,
    cx: f32,
    cy: f32,
    cz: f32,
) {
    let Some(center) = projector.project_3d(cx, cy, cz) else {
        return;
    };

    for edge in &gadget.edges {
        let v0 = gadget.get_vertex_3d(edge.vertex_indices[0]).unwrap();
        let v1 = gadget.get_vertex_3d(edge.vertex_indices[1]).unwrap();
        if let (Some(p0), Some(p1)) = (
            projector.project_3d(cx + v0.x, cy + v0.y, cz + v0.z),
            projector.project_3d(cx + v1.x, cy + v1.y, cz + v1.z),
        ) {
            painter.line_segment(
                [p0.screen_pos, p1.screen_pos],
                egui::Stroke::new(1.5, tetra_edge_color()),
            );
        }
    }

    if let Some(ref label) = gadget.tip_label {
        let font_id = egui::FontId::proportional(9.0);
        painter.text(
            center.screen_pos + egui::Vec2::new(0.0, -10.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            font_id,
            label_default(),
        );
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

fn bounds_extent(bounds: &(Vector4<f32>, Vector4<f32>), axis: usize) -> f32 {
    (bounds.1[axis] - bounds.0[axis]).max(1e-6)
}

fn normalize_4d(v: Vector4<f32>) -> Vector4<f32> {
    let len = v.norm();
    if len < 1e-6 {
        Vector4::zeros()
    } else {
        v / len
    }
}
