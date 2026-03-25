//! Tetrahedron debug toy for testing stereo tetrahedron visualization

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};
use std::collections::HashMap;

use crate::camera::Camera;
use crate::geometry::{apply_so4_rotation, create_glome};
use crate::input::{analyze_tap_in_stereo_view, DragView, TapAnalysis, TetraId, Zone};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_tetrahedron_overlay, split_stereo_views,
    TesseractRenderContext,
};
use crate::tetrahedron::get_tetrahedron_layout;
use crate::toy::{DragState, Toy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Camera,
    StereoTetrahedron,
}

pub struct TetrahedronDebugToy {
    camera: Camera,
    view_mode: ViewMode,
    tetrahedron_rotation: UnitQuaternion<f32>,
    visualization_rect: Option<egui::Rect>,
    drag_state: DragState,
    glome_rot_xy: f32,
    glome_rot_xz: f32,
    glome_rot_yz: f32,
    glome_rot_xw: f32,
    glome_rot_yw: f32,
    glome_rot_zw: f32,
    w_thickness: f32,
    eye_separation: f32,
    projection_distance: f32,
    tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>,
}

impl Default for TetrahedronDebugToy {
    fn default() -> Self {
        Self::new()
    }
}

impl TetrahedronDebugToy {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            view_mode: ViewMode::Camera,
            tetrahedron_rotation: UnitQuaternion::identity(),
            visualization_rect: None,
            drag_state: DragState::new(),
            glome_rot_xy: 0.0,
            glome_rot_xz: 0.0,
            glome_rot_yz: 0.0,
            glome_rot_xw: 0.0,
            glome_rot_yw: 0.0,
            glome_rot_zw: 0.0,
            w_thickness: 2.5,
            eye_separation: 0.3,
            projection_distance: 3.0,
            tetrahedron_rotations: HashMap::new(),
        }
    }

    fn get_tetrahedron_center(view_rect: egui::Rect, zone: Zone) -> (f32, f32) {
        let layout = get_tetrahedron_layout(view_rect);
        match zone {
            Zone::North => (view_rect.center().x, view_rect.min.y + layout.edge_offset),
            Zone::South => (view_rect.center().x, view_rect.max.y - layout.edge_offset),
            Zone::West => (view_rect.min.x + layout.edge_offset, view_rect.center().y),
            Zone::East => (view_rect.max.x - layout.edge_offset, view_rect.center().y),
        }
    }

    fn is_mouse_over_tetrahedron(pos: egui::Pos2, view_rect: egui::Rect, zone: Zone) -> bool {
        let (center_x, center_y) = Self::get_tetrahedron_center(view_rect, zone);
        let layout = get_tetrahedron_layout(view_rect);
        let hit_radius = layout.scale * 1.5;
        let dx = pos.x - center_x;
        let dy = pos.y - center_y;
        (dx * dx + dy * dy) <= hit_radius * hit_radius
    }

    fn get_tetrahedron_rotation(&self, id: TetraId) -> UnitQuaternion<f32> {
        self.tetrahedron_rotations
            .get(&id)
            .copied()
            .unwrap_or_else(UnitQuaternion::identity)
    }

    fn set_tetrahedron_rotation(&mut self, id: TetraId, rotation: UnitQuaternion<f32>) {
        self.tetrahedron_rotations.insert(id, rotation);
    }

    fn reset_tetrahedron_rotations(&mut self) {
        self.tetrahedron_rotations.clear();
    }

    fn get_to_origin_vector(&self) -> Vector4<f32> {
        Vector4::new(
            -self.camera.x,
            -self.camera.y,
            -self.camera.z,
            -self.camera.w,
        )
    }

    fn apply_camera_action(&mut self, action: CameraAction, speed: f32) {
        self.reset_tetrahedron_rotations();

        let forward = self.camera.forward_vector();
        let right = self.camera.right_vector();
        let up = self.camera.up_vector();
        let basis_4d = self.camera.rotation_4d.basis_vectors();

        let project_3d_to_4d = |v3: (f32, f32, f32)| -> [f32; 4] {
            [
                v3.0 * basis_4d[0][0] + v3.1 * basis_4d[1][0] + v3.2 * basis_4d[2][0],
                v3.0 * basis_4d[0][1] + v3.1 * basis_4d[1][1] + v3.2 * basis_4d[2][1],
                v3.0 * basis_4d[0][2] + v3.1 * basis_4d[1][2] + v3.2 * basis_4d[2][2],
                v3.0 * basis_4d[0][3] + v3.1 * basis_4d[1][3] + v3.2 * basis_4d[2][3],
            ]
        };

        match action {
            CameraAction::MoveForward => {
                let v4 = project_3d_to_4d(forward);
                self.camera.x += v4[0] * speed;
                self.camera.y += v4[1] * speed;
                self.camera.z += v4[2] * speed;
                self.camera.w += v4[3] * speed;
            }
            CameraAction::MoveBackward => {
                let v4 = project_3d_to_4d(forward);
                self.camera.x -= v4[0] * speed;
                self.camera.y -= v4[1] * speed;
                self.camera.z -= v4[2] * speed;
                self.camera.w -= v4[3] * speed;
            }
            CameraAction::StrafeLeft => {
                let v4 = project_3d_to_4d((-right.0, -right.1, -right.2));
                self.camera.x += v4[0] * speed;
                self.camera.y += v4[1] * speed;
                self.camera.z += v4[2] * speed;
                self.camera.w += v4[3] * speed;
            }
            CameraAction::StrafeRight => {
                let v4 = project_3d_to_4d(right);
                self.camera.x += v4[0] * speed;
                self.camera.y += v4[1] * speed;
                self.camera.z += v4[2] * speed;
                self.camera.w += v4[3] * speed;
            }
            CameraAction::MoveUp => {
                let v4 = project_3d_to_4d(up);
                self.camera.x += v4[0] * speed;
                self.camera.y += v4[1] * speed;
                self.camera.z += v4[2] * speed;
                self.camera.w += v4[3] * speed;
            }
            CameraAction::MoveDown => {
                let v4 = project_3d_to_4d((-up.0, -up.1, -up.2));
                self.camera.x += v4[0] * speed;
                self.camera.y += v4[1] * speed;
                self.camera.z += v4[2] * speed;
                self.camera.w += v4[3] * speed;
            }
            CameraAction::MoveSliceOrthogonalPos => {
                let w_dir = basis_4d[3];
                self.camera.x += w_dir[0] * speed;
                self.camera.y += w_dir[1] * speed;
                self.camera.z += w_dir[2] * speed;
                self.camera.w += w_dir[3] * speed;
            }
            CameraAction::MoveSliceOrthogonalNeg => {
                let w_dir = basis_4d[3];
                self.camera.x -= w_dir[0] * speed;
                self.camera.y -= w_dir[1] * speed;
                self.camera.z -= w_dir[2] * speed;
                self.camera.w -= w_dir[3] * speed;
            }
        }
    }

    fn zone_to_action(zone: Zone, is_left_view: bool) -> CameraAction {
        if is_left_view {
            match zone {
                Zone::North => CameraAction::MoveUp,
                Zone::South => CameraAction::MoveDown,
                Zone::West => CameraAction::StrafeLeft,
                Zone::East => CameraAction::StrafeRight,
            }
        } else {
            match zone {
                Zone::North => CameraAction::MoveForward,
                Zone::South => CameraAction::MoveBackward,
                Zone::West => CameraAction::MoveSliceOrthogonalNeg,
                Zone::East => CameraAction::MoveSliceOrthogonalPos,
            }
        }
    }

    fn render_glome(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let (left_rect, right_rect) = split_stereo_views(rect);
        self.visualization_rect = Some(rect);

        draw_center_divider(ui, rect);

        let (sin_xy, cos_xy) = self.glome_rot_xy.sin_cos();
        let (sin_xz, cos_xz) = self.glome_rot_xz.sin_cos();
        let (sin_yz, cos_yz) = self.glome_rot_yz.sin_cos();
        let (sin_xw, cos_xw) = self.glome_rot_xw.sin_cos();
        let (sin_yw, cos_yw) = self.glome_rot_yw.sin_cos();
        let (sin_zw, cos_zw) = self.glome_rot_zw.sin_cos();

        let (vertices, indices) = create_glome();
        let inv_orientation = self.camera.orientation.inverse();
        let camera_4d_rotation_inverse = self.camera.rotation_4d.inverse();
        let w_half = self.w_thickness * 0.5;

        for (eye_idx, (view_rect, eye_sign)) in [(left_rect, -1.0f32), (right_rect, 1.0f32)]
            .iter()
            .enumerate()
        {
            let is_left_view = eye_idx == 0;
            let center = view_rect.center();
            let scale = view_rect.height().min(view_rect.width()) * 0.35;
            let eye_offset = *eye_sign * self.eye_separation * 0.5;

            let painter = ui.painter().with_clip_rect(*view_rect);

            for chunk in indices.chunks(2) {
                if chunk.len() != 2 {
                    continue;
                }

                let v0 = &vertices[chunk[0] as usize];
                let v1 = &vertices[chunk[1] as usize];

                let p0_object = apply_so4_rotation(
                    v0.position,
                    sin_xy,
                    cos_xy,
                    sin_xz,
                    cos_xz,
                    sin_yz,
                    cos_yz,
                    sin_xw,
                    cos_xw,
                    sin_yw,
                    cos_yw,
                    sin_zw,
                    cos_zw,
                );
                let p1_object = apply_so4_rotation(
                    v1.position,
                    sin_xy,
                    cos_xy,
                    sin_xz,
                    cos_xz,
                    sin_yz,
                    cos_yz,
                    sin_xw,
                    cos_xw,
                    sin_yw,
                    cos_yw,
                    sin_zw,
                    cos_zw,
                );

                let p0_world = p0_object
                    - Vector4::new(self.camera.x, self.camera.y, self.camera.z, self.camera.w);
                let p1_world = p1_object
                    - Vector4::new(self.camera.x, self.camera.y, self.camera.z, self.camera.w);

                let p0_4d = camera_4d_rotation_inverse.rotate_vector(p0_world);
                let p1_4d = camera_4d_rotation_inverse.rotate_vector(p1_world);

                let w0_in_slice = p0_4d.w >= -w_half && p0_4d.w <= w_half;
                let w1_in_slice = p1_4d.w >= -w_half && p1_4d.w <= w_half;

                if !w0_in_slice && !w1_in_slice {
                    continue;
                }

                let p0_rel = Vector3::new(p0_4d.x, p0_4d.y, p0_4d.z);
                let p1_rel = Vector3::new(p1_4d.x, p1_4d.y, p1_4d.z);

                let p0_cam = inv_orientation.transform_vector(&p0_rel);
                let p1_cam = inv_orientation.transform_vector(&p1_rel);

                let dist = self.projection_distance;

                let s0 = if p0_cam.z > -dist + 0.1 {
                    let scale0 = scale / (p0_cam.z + dist);
                    Some(egui::Pos2::new(
                        center.x + (p0_cam.x + eye_offset) * scale0,
                        center.y - p0_cam.y * scale0,
                    ))
                } else {
                    None
                };

                let s1 = if p1_cam.z > -dist + 0.1 {
                    let scale1 = scale / (p1_cam.z + dist);
                    Some(egui::Pos2::new(
                        center.x + (p1_cam.x + eye_offset) * scale1,
                        center.y - p1_cam.y * scale1,
                    ))
                } else {
                    None
                };

                let Some((s0, s1)) = s0.zip(s1) else {
                    continue;
                };

                let w_avg = (p0_4d.w + p1_4d.w) / 2.0;
                let alpha = if w0_in_slice && w1_in_slice { 255 } else { 100 };
                let color = if w_avg >= 0.0 {
                    egui::Color32::from_rgba_unmultiplied(0, 255, 255, alpha)
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha)
                };

                painter.line_segment([s0, s1], egui::Stroke::new(2.5, color));
            }
        }
    }

    fn render_zone_tetrahedra(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let ctx = TesseractRenderContext::new(
            &self.camera,
            self.glome_rot_xy,
            self.glome_rot_xz,
            self.glome_rot_yz,
            self.glome_rot_xw,
            self.glome_rot_yw,
            self.glome_rot_zw,
            self.w_thickness,
            -2.0,
            2.0,
            self.eye_separation,
            self.projection_distance,
        );

        let (left_rect, right_rect) = split_stereo_views(rect);
        ctx.render_eye_view(
            ui,
            left_rect,
            -1.0,
            true,
            false,
            &self.tetrahedron_rotations,
        );
        ctx.render_eye_view(
            ui,
            right_rect,
            1.0,
            false,
            false,
            &self.tetrahedron_rotations,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    MoveUp,
    MoveDown,
    MoveSliceOrthogonalPos,
    MoveSliceOrthogonalNeg,
}

impl Toy for TetrahedronDebugToy {
    fn name(&self) -> &str {
        "Tetrahedron Debug"
    }

    fn id(&self) -> &str {
        "tetrahedron_debug"
    }

    fn reset(&mut self) {
        self.camera.reset();
        self.glome_rot_xy = 0.0;
        self.glome_rot_xz = 0.0;
        self.glome_rot_yz = 0.0;
        self.glome_rot_xw = 0.0;
        self.glome_rot_yw = 0.0;
        self.glome_rot_zw = 0.0;
        self.tetrahedron_rotations.clear();
        self.tetrahedron_rotation = UnitQuaternion::identity();
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label("Debug tool for stereo tetrahedron visualization");
        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.view_mode == ViewMode::Camera, "Camera View")
                .clicked()
            {
                self.view_mode = ViewMode::Camera;
            }
            if ui
                .selectable_label(
                    self.view_mode == ViewMode::StereoTetrahedron,
                    "Stereo Tetrahedron",
                )
                .clicked()
            {
                self.view_mode = ViewMode::StereoTetrahedron;
            }
        });

        ui.separator();

        if self.view_mode == ViewMode::Camera {
            ui.label("Arrows: Move | PgUp/Dn: Up/Down | ,/. : W-slice");
            ui.label("Mouse: Look");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::Slider::new(&mut self.camera.x, -10.0..=10.0).text(""));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::Slider::new(&mut self.camera.y, -10.0..=10.0).text(""));
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(egui::Slider::new(&mut self.camera.z, -10.0..=10.0).text(""));
            });
            ui.horizontal(|ui| {
                ui.label("W:");
                ui.add(egui::Slider::new(&mut self.camera.w, -5.0..=5.0).text(""));
            });

            ui.separator();

            let mut yaw = self.camera.yaw();
            let mut pitch = self.camera.pitch();
            ui.horizontal(|ui| {
                ui.label("Yaw:");
                if ui
                    .add(
                        egui::Slider::new(&mut yaw, -std::f32::consts::PI..=std::f32::consts::PI)
                            .text(""),
                    )
                    .changed()
                {
                    self.camera.set_yaw_pitch(yaw, pitch);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Pitch:");
                if ui
                    .add(
                        egui::Slider::new(&mut pitch, -std::f32::consts::PI..=std::f32::consts::PI)
                            .text(""),
                    )
                    .changed()
                {
                    self.camera.set_yaw_pitch(yaw, pitch);
                }
            });

            ui.separator();
            ui.collapsing("Glome Rotation", |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_xy,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("XY"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_xz,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("XZ"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_yz,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("YZ"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_xw,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("XW"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_yw,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("YW"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.glome_rot_zw,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text("ZW"),
                    );
                });
            });

            ui.collapsing("Slice Settings", |ui| {
                ui.add(egui::Slider::new(&mut self.w_thickness, 0.1..=5.0).text("W Thickness"));
            });
        } else {
            ui.label("Stereo Tetrahedron Mode");
            ui.label("Vector: camera position to origin");
            ui.label("");
            ui.label("Controls:");
            ui.label("  West (left) tap: Back to Camera");
            ui.label("  East (right) tap: Reset rotation");
            ui.label("  Right drag: Rotate tetrahedron");

            ui.separator();
            let to_origin = self.get_to_origin_vector();
            let magnitude = (to_origin.x.powi(2)
                + to_origin.y.powi(2)
                + to_origin.z.powi(2)
                + to_origin.w.powi(2))
            .sqrt();
            ui.label(format!("Distance to origin: {:.2}", magnitude));
            ui.label(format!(
                "Vector: ({:.2}, {:.2}, {:.2}, {:.2})",
                to_origin.x, to_origin.y, to_origin.z, to_origin.w
            ));

            ui.separator();
            ui.add(egui::Slider::new(&mut self.eye_separation, 0.0..=1.0).text("Eye Separation"));
        }

        ui.separator();
        ui.collapsing("Stereoscopic", |ui| {
            ui.add(
                egui::Slider::new(&mut self.projection_distance, 1.0..=10.0)
                    .text("Projection Distance"),
            );
        });
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _show_debug: bool) {
        draw_background(ui, rect);
        self.visualization_rect = Some(rect);

        match self.view_mode {
            ViewMode::Camera => {
                self.render_glome(ui, rect);
                self.render_zone_tetrahedra(ui, rect);
            }
            ViewMode::StereoTetrahedron => {
                draw_center_divider(ui, rect);
                let to_origin = self.get_to_origin_vector();
                render_stereo_tetrahedron_overlay(
                    ui,
                    rect,
                    to_origin,
                    &self.tetrahedron_rotation,
                    self.eye_separation,
                );
            }
        }
    }

    fn render_overlay(
        &mut self,
        _ui: &mut egui::Ui,
        _left_rect: egui::Rect,
        _right_rect: egui::Rect,
    ) {
    }

    fn handle_tap(&mut self, analysis: &TapAnalysis) {
        if self.view_mode == ViewMode::StereoTetrahedron {
            if analysis.is_left_view && analysis.zone == Zone::West {
                self.view_mode = ViewMode::Camera;
                return;
            }
            if !analysis.is_left_view && analysis.zone == Zone::East {
                self.tetrahedron_rotation = UnitQuaternion::identity();
                return;
            }
            return;
        }

        self.drag_state.last_tap_pos = Some(egui::Pos2::new(
            analysis.view_rect.min.x + analysis.norm_x * analysis.view_rect.width(),
            analysis.view_rect.min.y + analysis.norm_y * analysis.view_rect.height(),
        ));
        self.drag_state.last_tap_zone = Some(analysis.zone);
        self.drag_state.last_tap_view_left = analysis.is_left_view;

        let action = Self::zone_to_action(analysis.zone, analysis.is_left_view);
        self.apply_camera_action(action, 0.15);
    }

    fn handle_drag(&mut self, _is_left_view: bool, from: egui::Pos2, to: egui::Pos2) {
        if self.view_mode == ViewMode::StereoTetrahedron {
            if let Some(last_pos) = self.drag_state.last_mouse_pos {
                let delta = to - last_pos;
                let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -delta.x * 0.005);
                let pitch_rot =
                    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * 0.005);
                let incremental = pitch_rot * yaw_rot;
                self.tetrahedron_rotation = incremental * self.tetrahedron_rotation;
            }
            self.drag_state.last_mouse_pos = Some(to);
            self.drag_state.is_dragging = true;
            return;
        }

        if let Some(tetra_id) = self.drag_state.dragging_tetrahedron {
            if let Some(last_pos) = self.drag_state.last_tetra_drag_pos {
                let delta = to - last_pos;
                let current_rot = self.get_tetrahedron_rotation(tetra_id);

                let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -delta.x * 0.005);
                let pitch_rot =
                    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * 0.005);

                let incremental = pitch_rot * yaw_rot;
                let new_rot = incremental * current_rot;

                self.set_tetrahedron_rotation(tetra_id, new_rot);
            }
            self.drag_state.last_tetra_drag_pos = Some(to);
            self.drag_state.is_dragging = true;
            return;
        }

        let delta = to - from;

        if let Some(visualization_rect) = self.visualization_rect {
            if visualization_rect.contains(from) {
                if let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, from) {
                    if Self::is_mouse_over_tetrahedron(from, analysis.view_rect, analysis.zone) {
                        let tetra_id = TetraId {
                            is_left_view: analysis.is_left_view,
                            zone: analysis.zone,
                        };
                        self.drag_state.dragging_tetrahedron = Some(tetra_id);
                        self.drag_state.last_tetra_drag_pos = Some(to);
                        self.drag_state.is_dragging = true;
                        return;
                    }
                }
            }
        }

        match self.drag_state.drag_view {
            Some(DragView::Left) => {
                self.camera.rotate(delta.x, delta.y);
                self.reset_tetrahedron_rotations();
            }
            Some(DragView::Right) => {
                self.camera.rotate_4d(delta.x, delta.y);
                self.reset_tetrahedron_rotations();
            }
            None => {}
        }
        self.drag_state.is_dragging = true;
    }

    fn handle_hold(&mut self, analysis: &TapAnalysis) {
        if self.view_mode == ViewMode::StereoTetrahedron {
            return;
        }

        let action = Self::zone_to_action(analysis.zone, analysis.is_left_view);
        self.apply_camera_action(action, 0.08);
    }

    fn handle_drag_start(&mut self, drag_view: DragView) {
        self.drag_state.drag_view = Some(drag_view);
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if self.view_mode == ViewMode::StereoTetrahedron {
            return;
        }

        let move_speed = 0.15;

        ctx.input(|i| {
            if i.key_down(egui::Key::ArrowUp) {
                self.apply_camera_action(CameraAction::MoveForward, move_speed);
            }
            if i.key_down(egui::Key::ArrowDown) {
                self.apply_camera_action(CameraAction::MoveBackward, move_speed);
            }
            if i.key_down(egui::Key::ArrowLeft) {
                self.apply_camera_action(CameraAction::StrafeLeft, move_speed);
            }
            if i.key_down(egui::Key::ArrowRight) {
                self.apply_camera_action(CameraAction::StrafeRight, move_speed);
            }
            if i.key_down(egui::Key::PageUp) {
                self.apply_camera_action(CameraAction::MoveUp, move_speed);
            }
            if i.key_down(egui::Key::PageDown) {
                self.apply_camera_action(CameraAction::MoveDown, move_speed);
            }
            if i.key_down(egui::Key::Period) {
                self.apply_camera_action(CameraAction::MoveSliceOrthogonalPos, move_speed);
            }
            if i.key_down(egui::Key::Comma) {
                self.apply_camera_action(CameraAction::MoveSliceOrthogonalNeg, move_speed);
            }
        });
    }

    fn get_visualization_rect(&self) -> Option<egui::Rect> {
        self.visualization_rect
    }

    fn set_visualization_rect(&mut self, rect: egui::Rect) {
        self.visualization_rect = Some(rect);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
