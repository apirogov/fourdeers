//! Tesseract visualization toy

use eframe::egui;
use nalgebra::UnitQuaternion;
use std::collections::HashMap;

use crate::camera::{Camera, CameraAction};
use crate::input::{DragView, TapAnalysis, TetraId, Zone, ZoneMode};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    draw_background, draw_center_divider, render_tap_zone_label, split_stereo_views, FourDSettings,
    StereoSettings, TesseractRenderContext,
};
use crate::toy::{DragState, Toy};

pub struct TesseractToy {
    pub camera: Camera,
    polytope_type: PolytopeType,
    rot_xy: f32,
    rot_xz: f32,
    rot_yz: f32,
    rot_xw: f32,
    rot_yw: f32,
    rot_zw: f32,
    show_controls: bool,
    zone_mode: ZoneMode,
    visualization_rect: Option<egui::Rect>,
    pub drag_state: DragState,
    tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>,
    stereo: StereoSettings,
    four_d: FourDSettings,
    right_view_4d_rotation: bool,
}

impl Default for TesseractToy {
    fn default() -> Self {
        Self::new()
    }
}

impl TesseractToy {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            polytope_type: PolytopeType::EightCell,
            rot_xy: 0.0,
            rot_xz: 0.0,
            rot_yz: 0.0,
            rot_xw: 0.0,
            rot_yw: 0.0,
            rot_zw: 0.0,
            show_controls: false,
            zone_mode: ZoneMode::NineZones,
            visualization_rect: None,
            drag_state: DragState::new(),
            tetrahedron_rotations: HashMap::new(),
            stereo: StereoSettings::new(),
            four_d: FourDSettings::default(),
            right_view_4d_rotation: false,
        }
    }

    fn reset_tetrahedron_rotations(&mut self) {
        self.tetrahedron_rotations.clear();
    }

    fn apply_camera_action(&mut self, action: CameraAction, speed: f32) {
        self.reset_tetrahedron_rotations();
        self.camera.apply_action(action, speed);
    }

    fn zone_to_action(zone: Zone, is_left_view: bool) -> Option<CameraAction> {
        if is_left_view {
            match zone {
                Zone::North => Some(CameraAction::MoveUp),
                Zone::South => Some(CameraAction::MoveDown),
                Zone::West => Some(CameraAction::MoveLeft),
                Zone::East => Some(CameraAction::MoveRight),
                _ => None,
            }
        } else {
            match zone {
                Zone::North => Some(CameraAction::MoveUp),
                Zone::South => Some(CameraAction::MoveDown),
                Zone::West => Some(CameraAction::MoveLeft),
                Zone::East => Some(CameraAction::MoveRight),
                Zone::NorthEast => Some(CameraAction::MoveSliceForward),
                Zone::SouthWest => Some(CameraAction::MoveSliceBackward),
                Zone::NorthWest => Some(CameraAction::MoveKata),
                Zone::SouthEast => Some(CameraAction::MoveAna),
                _ => None,
            }
        }
    }
}

impl Toy for TesseractToy {
    fn name(&self) -> &str {
        "Polytopes"
    }

    fn id(&self) -> &str {
        "tesseract"
    }

    fn reset(&mut self) {
        self.camera.reset();
        self.rot_xy = 0.0;
        self.rot_xz = 0.0;
        self.rot_yz = 0.0;
        self.rot_xw = 0.0;
        self.rot_yw = 0.0;
        self.rot_zw = 0.0;
        self.tetrahedron_rotations.clear();
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label("4D Polytope Visualization");

        egui::ComboBox::from_label("")
            .selected_text(self.polytope_type.name())
            .show_ui(ui, |ui| {
                for poly_type in PolytopeType::all() {
                    ui.selectable_value(&mut self.polytope_type, poly_type, poly_type.name());
                }
            });

        ui.label(format!(
            "{} vertices, {} edges",
            self.polytope_type.vertex_count(),
            self.polytope_type.edge_count()
        ));

        ui.separator();

        ui.collapsing("Controls", |ui| {
            ui.label("Arrows: Move | PgUp/Dn: Up/Down | ,/. : W-slice");
            ui.separator();
            ui.checkbox(&mut self.show_controls, "Show Mouse Controls");
        });

        ui.add_space(8.0);
        ui.heading("Camera");

        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::Slider::new(&mut self.camera.x, -10.0..=10.0).text(""));
            ui.label("Y:");
            ui.add(egui::Slider::new(&mut self.camera.y, -10.0..=10.0).text(""));
        });
        ui.horizontal(|ui| {
            ui.label("Z:");
            ui.add(egui::Slider::new(&mut self.camera.z, -10.0..=10.0).text(""));
            ui.label("W:");
            ui.add(egui::Slider::new(&mut self.camera.w, -3.0..=3.0).text(""));
        });

        ui.horizontal(|ui| {
            let mut yaw_l = self.camera.yaw_l();
            ui.label("Yaw(L)");
            if ui
                .add(
                    egui::Slider::new(&mut yaw_l, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_yaw_l(yaw_l);
            }

            let mut pitch_l = self.camera.pitch_l();
            ui.label("Pitch(L)");
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

        ui.horizontal(|ui| {
            let mut yaw_r = self.camera.yaw_r();
            ui.label("Yaw(R)");
            if ui
                .add(
                    egui::Slider::new(&mut yaw_r, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_yaw_r(yaw_r);
            }

            let mut pitch_r = self.camera.pitch_r();
            ui.label("Pitch(R)");
            if ui
                .add(
                    egui::Slider::new(&mut pitch_r, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_pitch_r(pitch_r);
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "Position: ({:.2}, {:.2}, {:.2})",
                self.camera.x, self.camera.y, self.camera.z
            ));
            ui.label(format!("W: {:.2}", self.camera.w));
        });

        ui.separator();
        ui.add_space(4.0);

        ui.collapsing("4D Object Rotation", |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xy,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("XY"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xz,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("XZ"),
                );
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_yz,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("YZ"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("XW"),
                );
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_yw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("YW"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_zw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text("ZW"),
                );
            });
        });

        ui.add_space(4.0);

        ui.add_space(4.0);
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool) {
        draw_background(ui, rect);

        let (left_rect, right_rect) = split_stereo_views(rect);
        self.visualization_rect = Some(rect);

        draw_center_divider(ui, rect);

        let (vertices, indices) = create_polytope(self.polytope_type);

        let ctx = TesseractRenderContext::with_stereo_settings(
            vertices,
            indices,
            &self.camera,
            self.rot_xy,
            self.rot_xz,
            self.rot_yz,
            self.rot_xw,
            self.rot_yw,
            self.rot_zw,
            self.four_d.w_color_intensity,
            &self.stereo,
        );

        ctx.render_eye_view(
            ui,
            left_rect,
            -1.0,
            true,
            show_debug,
            self.show_controls,
            &self.tetrahedron_rotations,
        );
        ctx.render_eye_view(
            ui,
            right_rect,
            1.0,
            false,
            show_debug,
            self.show_controls,
            &self.tetrahedron_rotations,
        );
    }

    fn render_toy_menu(&self, painter: &egui::Painter, rect: egui::Rect) {
        let rot_label = if self.right_view_4d_rotation {
            "Rot:4D"
        } else {
            "Rot:3D"
        };
        render_tap_zone_label(painter, rect, Zone::Center, rot_label);
    }

    fn set_stereo_settings(&mut self, settings: &crate::render::StereoSettings) {
        self.stereo = settings.clone();
    }

    fn set_four_d_settings(&mut self, settings: &FourDSettings) {
        self.four_d = settings.clone();
    }

    fn render_overlay(
        &mut self,
        _ui: &mut egui::Ui,
        _left_rect: egui::Rect,
        _right_rect: egui::Rect,
    ) {
        // Overlay rendering is handled in render_scene for now
    }

    fn handle_tap(&mut self, analysis: &TapAnalysis) {
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
        if let Some(action) = Self::zone_to_action(analysis.zone, analysis.is_left_view) {
            self.apply_camera_action(action, 0.08);
        }
    }

    fn handle_drag_start(&mut self, drag_view: DragView) {
        self.drag_state.drag_view = Some(drag_view);
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
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
        self.zone_mode
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
