//! Tesseract visualization toy

use eframe::egui;
use nalgebra::UnitQuaternion;
use std::collections::HashMap;

use crate::camera::{Camera, CameraAction};
use crate::input::{DragView, TapAnalysis, TetraId, Zone, ZoneMode};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    draw_background, draw_center_divider, split_stereo_views, StereoSettings,
    TesseractRenderContext,
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
    w_min: f32,
    w_max: f32,
    show_debug: bool,
    show_controls: bool,
    zone_mode: ZoneMode,
    visualization_rect: Option<egui::Rect>,
    pub drag_state: DragState,
    tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>,
    stereo: StereoSettings,
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
            w_min: -2.0,
            w_max: 2.0,
            show_debug: false,
            show_controls: false,
            zone_mode: ZoneMode::NineZones,
            visualization_rect: None,
            drag_state: DragState::new(),
            tetrahedron_rotations: HashMap::new(),
            stereo: StereoSettings::new(),
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

        egui::ComboBox::from_label("Polytope")
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

        ui.label("Arrows: Move | PgUp/Dn: Up/Down | ,/. : W-slice");
        ui.label("Mouse: Look");
        ui.separator();

        ui.checkbox(&mut self.show_debug, "Show Debug Overlay");
        ui.checkbox(&mut self.show_controls, "Show Controls");

        ui.add_space(8.0);
        ui.heading("Position & Orientation");

        let is_small_screen = ui.available_width() < 250.0;

        if is_small_screen {
            ui.vertical(|ui| {
                ui.label("X Position");
                ui.add(egui::Slider::new(&mut self.camera.x, -10.0..=10.0).text(""));
                ui.label("Y Position");
                ui.add(egui::Slider::new(&mut self.camera.y, -10.0..=10.0).text(""));
                ui.label("Z Position");
                ui.add(egui::Slider::new(&mut self.camera.z, -10.0..=10.0).text(""));
                ui.label("W-slice:");
                ui.add(egui::Slider::new(&mut self.camera.w, -3.0..=3.0).text(""));
            });
        } else {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("X Position");
                    ui.add(egui::Slider::new(&mut self.camera.x, -10.0..=10.0).text(""));
                });
                ui.vertical(|ui| {
                    ui.label("Y Position");
                    ui.add(egui::Slider::new(&mut self.camera.y, -10.0..=10.0).text(""));
                });
            });

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Z Position");
                    ui.add(egui::Slider::new(&mut self.camera.z, -10.0..=10.0).text(""));
                });
                ui.vertical(|ui| {
                    ui.label("W-slice:");
                    ui.add(egui::Slider::new(&mut self.camera.w, -3.0..=3.0).text(""));
                });
            });
        }

        let mut yaw = self.camera.yaw();
        let mut pitch = self.camera.pitch();

        ui.label("Camera Orientation:");
        ui.horizontal(|ui| {
            ui.label("Yaw");
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
            ui.label("Pitch");
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

        ui.horizontal(|ui| {
            ui.label(format!(
                "Position: ({:.2}, {:.2}, {:.2})",
                self.camera.x, self.camera.y, self.camera.z
            ));
            ui.label(format!("W: {:.2}", self.camera.w));
        });

        ui.separator();
        ui.add_space(4.0);

        ui.collapsing("Keyboard Controls", |ui| {
            ui.label("Arrow keys: Move forward/back/strafe");
            ui.label("PageUp/Down: Move up/down");
            ui.label(",/.: W-slice movement");
            ui.label("");
            ui.label("Tap & hold zones for movement");
            ui.label("Drag to rotate camera");
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

        ui.collapsing("Slice Settings", |ui| {
            ui.add(egui::Slider::new(&mut self.stereo.w_thickness, 0.1..=2.0).text("W Thickness"));
        });

        ui.add_space(4.0);

        ui.collapsing("Stereoscopic", |ui| {
            ui.add(
                egui::Slider::new(&mut self.stereo.eye_separation, 0.0..=1.0)
                    .text("Eye Separation"),
            );
            ui.add(
                egui::Slider::new(&mut self.stereo.projection_distance, 1.0..=10.0)
                    .text("Projection Distance"),
            );
            ui.horizontal(|ui| {
                ui.label("Projection:");
                let persp_label =
                    if self.stereo.projection_mode == crate::render::ProjectionMode::Perspective {
                        "● Perspective"
                    } else {
                        "○ Perspective"
                    };
                let ortho_label =
                    if self.stereo.projection_mode == crate::render::ProjectionMode::Orthographic {
                        "● Orthographic"
                    } else {
                        "○ Orthographic"
                    };
                if ui.button(persp_label).clicked() {
                    self.stereo.projection_mode = crate::render::ProjectionMode::Perspective;
                }
                if ui.button(ortho_label).clicked() {
                    self.stereo.projection_mode = crate::render::ProjectionMode::Orthographic;
                }
            });
        });

        ui.add_space(4.0);

        ui.collapsing("W Coloring", |ui| {
            ui.horizontal(|ui| {
                ui.label("Range:");
                ui.add(egui::DragValue::new(&mut self.w_min).speed(0.1));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.w_max).speed(0.1));
            });
        });

        ui.separator();
        ui.label(format!("Geometry: {}", self.polytope_type.name()));
        ui.label(format!(
            "{} vertices, {} edges",
            self.polytope_type.vertex_count(),
            self.polytope_type.edge_count()
        ));
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
            self.w_min,
            self.w_max,
            &self.stereo,
        );

        ctx.render_eye_view(
            ui,
            left_rect,
            -1.0,
            true,
            show_debug || self.show_debug,
            self.show_controls,
            &self.tetrahedron_rotations,
            Some(self.right_view_4d_rotation),
        );
        ctx.render_eye_view(
            ui,
            right_rect,
            1.0,
            false,
            show_debug || self.show_debug,
            self.show_controls,
            &self.tetrahedron_rotations,
            Some(self.right_view_4d_rotation),
        );
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
