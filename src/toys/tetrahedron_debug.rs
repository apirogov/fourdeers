//! Tetrahedron debug toy for testing stereo tetrahedron visualization

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};
use std::collections::HashMap;

use crate::camera::{Camera, CameraAction};
use crate::geometry::apply_so4_rotation;
use crate::input::{DragView, TapAnalysis, TetraId, Zone, ZoneMode};
use crate::polytopes::create_polytope;
use crate::render::{
    draw_background, draw_center_divider, render_stereo_tetrahedron_overlay, render_tap_zone_label,
    split_stereo_views, w_to_color, FourDSettings, StereoProjector, StereoSettings,
};
use crate::tetrahedron::magnitude_4d;
use crate::toy::{DragState, Toy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Camera,
    StereoTetrahedron,
}

pub struct TetrahedronDebugToy {
    pub camera: Camera,
    view_mode: ViewMode,
    tetrahedron_rotation: UnitQuaternion<f32>,
    visualization_rect: Option<egui::Rect>,
    pub drag_state: DragState,
    glome_rot_xy: f32,
    glome_rot_xz: f32,
    glome_rot_yz: f32,
    glome_rot_xw: f32,
    glome_rot_yw: f32,
    glome_rot_zw: f32,
    stereo: StereoSettings,
    four_d: FourDSettings,
    tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>,
    right_view_4d_rotation: bool,
    show_controls: bool,
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
            stereo: StereoSettings::new(),
            four_d: FourDSettings::default(),
            tetrahedron_rotations: HashMap::new(),
            right_view_4d_rotation: false,
            show_controls: false,
        }
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
        self.camera.apply_action(action, speed);
    }

    fn zone_to_action(zone: Zone, is_left_view: bool) -> Option<CameraAction> {
        if !zone.is_cardinal() {
            return None;
        }

        let action = if is_left_view {
            match zone {
                Zone::North => CameraAction::MoveUp,
                Zone::South => CameraAction::MoveDown,
                Zone::West => CameraAction::MoveLeft,
                Zone::East => CameraAction::MoveRight,
                _ => unreachable!(),
            }
        } else {
            match zone {
                Zone::North => CameraAction::MoveForward,
                Zone::South => CameraAction::MoveBackward,
                Zone::West => CameraAction::MoveAna,
                Zone::East => CameraAction::MoveKata,
                _ => unreachable!(),
            }
        };
        Some(action)
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

        let (vertices, indices) = create_polytope(crate::polytopes::PolytopeType::SixteenCell);
        let inv_q_left = self.camera.rotation_4d.q_left().inverse();
        let camera_4d_rotation_inverse = self.camera.rotation_4d.inverse();
        let w_half = self.four_d.w_thickness * 0.5;

        for (eye_idx, (view_rect, eye_sign)) in [(left_rect, -1.0f32), (right_rect, 1.0f32)]
            .iter()
            .enumerate()
        {
            let _is_left_view = eye_idx == 0;
            let center = view_rect.center();
            let scale = view_rect.height().min(view_rect.width()) * 0.35;
            let eye_offset = *eye_sign * self.stereo.eye_separation * 0.5;

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

                let p0_cam = inv_q_left.transform_vector(&p0_rel);
                let p1_cam = inv_q_left.transform_vector(&p1_rel);

                let dist = self.stereo.projection_distance;

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

                let normalized_w = (w_avg / w_half).clamp(-1.0, 1.0);
                let color = w_to_color(normalized_w, alpha, self.four_d.w_color_intensity);

                painter.line_segment([s0, s1], egui::Stroke::new(2.5, color));
            }
        }
    }
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

        ui.checkbox(&mut self.show_controls, "Show Controls");

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
                ui.label("XY:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_xy, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
            ui.horizontal(|ui| {
                ui.label("XZ:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_xz, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
            ui.horizontal(|ui| {
                ui.label("YZ:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_yz, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
            ui.horizontal(|ui| {
                ui.label("XW:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_xw, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
            ui.horizontal(|ui| {
                ui.label("YW:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_yw, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
            ui.horizontal(|ui| {
                ui.label("ZW:");
                ui.add(
                    egui::Slider::new(&mut self.glome_rot_zw, 0.0..=std::f32::consts::TAU).text(""),
                );
            });
        }

        if self.view_mode == ViewMode::StereoTetrahedron {
            ui.label("Arrows: Rotate | PgUp/Dn: Up/Down | ,/. : W-slice");
            ui.label("Mouse: Look");
            ui.separator();
        }

        let mut pitch_l = self.camera.pitch_l();

        ui.horizontal(|ui| {
            ui.label("Pitch(L):");
            if ui
                .add(
                    egui::Slider::new(&mut pitch_l, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_pitch_l(pitch_l);
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

        ui.label("Stereo Tetrahedron Mode");
        ui.label("Vector: camera position to origin");
        ui.label("");
        ui.label("Controls:");
        ui.label("  West (left) tap: Back to Camera");
        ui.label("  East (right) tap: Reset rotation");
        ui.label("  Right drag: Rotate tetrahedron");

        ui.separator();
        let to_origin = self.get_to_origin_vector();
        let magnitude = magnitude_4d(to_origin);
        ui.label(format!("Distance to origin: {:.2}", magnitude));
        ui.label(format!(
            "Vector: ({:.2}, {:.2}, {:.2}, {:.2})",
            to_origin.x, to_origin.y, to_origin.z, to_origin.w
        ));

        ui.separator();
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _show_debug: bool) {
        draw_background(ui, rect);
        self.visualization_rect = Some(rect);

        match self.view_mode {
            ViewMode::Camera => {
                self.render_glome(ui, rect);
            }
            ViewMode::StereoTetrahedron => {
                draw_center_divider(ui, rect);
                let to_origin = self.get_to_origin_vector();
                let scale = rect.height().min(rect.width()) * 0.25;
                let projector = StereoProjector::new(
                    rect.center(),
                    scale,
                    self.stereo.eye_separation,
                    self.stereo.projection_mode,
                );
                render_stereo_tetrahedron_overlay(
                    ui,
                    rect,
                    to_origin,
                    &self.tetrahedron_rotation,
                    &projector,
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

    fn render_toy_menu(&self, painter: &egui::Painter, rect: egui::Rect) {
        let rot_label = if self.right_view_4d_rotation {
            "Rot:4D"
        } else {
            "Rot:3D"
        };
        render_tap_zone_label(painter, rect, Zone::Center, rot_label, None);
    }

    fn set_stereo_settings(&mut self, settings: &crate::render::StereoSettings) {
        self.stereo = settings.clone();
    }

    fn set_four_d_settings(&mut self, settings: &FourDSettings) {
        self.four_d = settings.clone();
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

        if analysis.is_left_view && analysis.zone == Zone::SouthWest {
            self.right_view_4d_rotation = !self.right_view_4d_rotation;
            return;
        }

        if !analysis.is_left_view && analysis.zone == Zone::Center {
            self.right_view_4d_rotation = !self.right_view_4d_rotation;
            return;
        }

        self.drag_state.last_tap_pos = Some(egui::Pos2::new(
            analysis.view_rect.min.x + analysis.norm_x * analysis.view_rect.width(),
            analysis.view_rect.min.y + analysis.norm_y * analysis.view_rect.height(),
        ));
        self.drag_state.last_tap_zone = Some(analysis.zone);
        self.drag_state.last_tap_view_left = analysis.is_left_view;

        if let Some(action) = Self::zone_to_action(analysis.zone, analysis.is_left_view) {
            self.apply_camera_action(action, 0.15);
        }
    }

    fn handle_drag(&mut self, _is_left_view: bool, from: egui::Pos2, to: egui::Pos2) {
        if self.view_mode == ViewMode::StereoTetrahedron {
            let delta = to - from;
            let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), delta.x * 0.005);
            let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * 0.005);
            let incremental = pitch_rot * yaw_rot;
            self.tetrahedron_rotation = incremental * self.tetrahedron_rotation;
            self.drag_state.is_dragging = true;
            return;
        }

        let delta = to - from;

        match self.drag_state.drag_view {
            Some(DragView::Left) => {
                self.camera.rotate(delta.x, delta.y);
                self.reset_tetrahedron_rotations();
            }
            Some(DragView::Right) => {
                if self.right_view_4d_rotation {
                    self.camera.rotate_4d(delta.x, delta.y);
                } else {
                    self.camera.rotate(delta.x, delta.y);
                }
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

        if let Some(action) = Self::zone_to_action(analysis.zone, analysis.is_left_view) {
            self.apply_camera_action(action, 0.08);
        }
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
                self.apply_camera_action(CameraAction::MoveLeft, move_speed);
            }
            if i.key_down(egui::Key::ArrowRight) {
                self.apply_camera_action(CameraAction::MoveRight, move_speed);
            }
            if i.key_down(egui::Key::PageUp) {
                self.apply_camera_action(CameraAction::MoveUp, move_speed);
            }
            if i.key_down(egui::Key::PageDown) {
                self.apply_camera_action(CameraAction::MoveDown, move_speed);
            }
            if i.key_down(egui::Key::Period) {
                self.apply_camera_action(CameraAction::MoveKata, move_speed);
            }
            if i.key_down(egui::Key::Comma) {
                self.apply_camera_action(CameraAction::MoveAna, move_speed);
            }
        });
    }

    fn get_visualization_rect(&self) -> Option<egui::Rect> {
        self.visualization_rect
    }

    fn set_visualization_rect(&mut self, rect: egui::Rect) {
        self.visualization_rect = Some(rect);
    }

    fn get_zone_mode(&self) -> ZoneMode {
        ZoneMode::NineZones
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
