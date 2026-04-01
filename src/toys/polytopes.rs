//! Tesseract visualization toy

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{Camera, CameraAction};
use crate::colors::label_inactive;
use crate::input::{DragView, TapAnalysis, TetraId, Zone, ZoneMode};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_views, render_tap_zone_label,
    split_stereo_views, FourDSettings, ObjectRotationAngles, StereoSettings, TesseractRenderConfig,
    TesseractRenderContext,
};
use crate::toy::{CompassWaypoint, DragState, Toy};

pub struct PolytopesToy {
    pub camera: Camera,
    polytope_type: PolytopeType,
    cached_vertices: Vec<crate::polytopes::Vertex4D>,
    cached_indices: Vec<u16>,
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

impl Default for PolytopesToy {
    fn default() -> Self {
        Self::new()
    }
}

impl PolytopesToy {
    pub fn new() -> Self {
        let polytope_type = PolytopeType::EightCell;
        let (cached_vertices, cached_indices) = create_polytope(polytope_type);
        Self {
            camera: Camera::new(),
            polytope_type,
            cached_vertices,
            cached_indices,
            rot_xy: 0.0,
            rot_xz: 0.0,
            rot_yz: 0.0,
            rot_xw: 0.0,
            rot_yw: 0.0,
            rot_zw: 0.0,
            show_controls: true,
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

    fn ensure_polytope_cached(&mut self) {
        let (vertices, indices) = create_polytope(self.polytope_type);
        self.cached_vertices = vertices;
        self.cached_indices = indices;
    }

    fn apply_camera_action(&mut self, action: CameraAction, speed: f32) {
        self.reset_tetrahedron_rotations();
        self.camera.apply_action(action, speed);
    }

    fn zone_to_action(zone: Zone, is_left_view: bool) -> Option<CameraAction> {
        if is_left_view {
            None
        } else {
            match zone {
                Zone::North => Some(CameraAction::MoveUp),
                Zone::South => Some(CameraAction::MoveDown),
                Zone::West => Some(CameraAction::MoveLeft),
                Zone::East => Some(CameraAction::MoveRight),
                Zone::NorthEast => Some(CameraAction::MoveForward),
                Zone::SouthWest => Some(CameraAction::MoveBackward),
                Zone::NorthWest => Some(CameraAction::MoveKata),
                Zone::SouthEast => Some(CameraAction::MoveAna),
                _ => None,
            }
        }
    }
}

impl Toy for PolytopesToy {
    fn name(&self) -> &str {
        "Polytopes"
    }

    fn id(&self) -> &str {
        "polytopes"
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

        let prev_type = self.polytope_type;
        egui::ComboBox::from_label("")
            .selected_text(self.polytope_type.name())
            .show_ui(ui, |ui| {
                for poly_type in PolytopeType::all() {
                    ui.selectable_value(&mut self.polytope_type, poly_type, poly_type.name());
                }
            });
        if self.polytope_type != prev_type {
            self.ensure_polytope_cached();
            self.camera.reset();
            self.rot_xy = 0.0;
            self.rot_xz = 0.0;
            self.rot_yz = 0.0;
            self.rot_xw = 0.0;
            self.rot_yw = 0.0;
            self.rot_zw = 0.0;
        }

        ui.label(format!(
            "{} vertices, {} edges",
            self.polytope_type.vertex_count(),
            self.polytope_type.edge_count()
        ));

        ui.separator();

        ui.collapsing("Controls", |ui| {
            ui.label("Arrows Up/Down: Y | Arrows Left/Right: X | PgUp/Dn: Z | ,/. : W");
            ui.separator();
            ui.checkbox(&mut self.show_controls, "Show Mouse Controls");
        });

        ui.add_space(8.0);

        ui.collapsing("Camera", |ui| {
            ui.label(format!(
                "Position: ({:.1}, {:.1}, {:.1}, {:.1})",
                self.camera.position.x,
                self.camera.position.y,
                self.camera.position.z,
                self.camera.position.w
            ));

            // X + Y
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::Slider::new(&mut self.camera.position.x, -10.0..=10.0).text(""));
                ui.label("Y:");
                ui.add(egui::Slider::new(&mut self.camera.position.y, -10.0..=10.0).text(""));
            });

            // Z + W
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(egui::Slider::new(&mut self.camera.position.z, -10.0..=10.0).text(""));
                ui.label("W:");
                ui.add(egui::Slider::new(&mut self.camera.position.w, -3.0..=3.0).text(""));
            });

            // Yaw(L) + Pitch(L)
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
                        egui::Slider::new(
                            &mut pitch_l,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text(""),
                    )
                    .changed()
                {
                    self.camera.set_pitch_l(pitch_l);
                }
            });

            // Yaw(R) + Pitch(R)
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
                        egui::Slider::new(
                            &mut pitch_r,
                            -std::f32::consts::PI..=std::f32::consts::PI,
                        )
                        .text(""),
                    )
                    .changed()
                {
                    self.camera.set_pitch_r(pitch_r);
                }
            });
        }); // Camera collapsible

        ui.separator();
        ui.add_space(4.0);

        ui.collapsing("4D Object Rotation", |ui| {
            // XY + XZ
            ui.horizontal(|ui| {
                ui.label("XY:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xy,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
                ui.label("XZ:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xz,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
            });

            // YZ + XW
            ui.horizontal(|ui| {
                ui.label("YZ:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_yz,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
                ui.label("XW:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_xw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
            });

            // YW + ZW
            ui.horizontal(|ui| {
                ui.label("YW:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_yw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
                ui.label("ZW:");
                ui.add(
                    egui::Slider::new(
                        &mut self.rot_zw,
                        -std::f32::consts::PI..=std::f32::consts::PI,
                    )
                    .text(""),
                );
            });
        });

        ui.add_space(4.0);

        ui.add_space(4.0);
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool) {
        draw_background(ui, rect);

        self.visualization_rect = Some(rect);

        draw_center_divider(ui, rect);

        let config = TesseractRenderConfig {
            rotation_angles: ObjectRotationAngles {
                xy: self.rot_xy,
                xz: self.rot_xz,
                yz: self.rot_yz,
                xw: self.rot_xw,
                yw: self.rot_yw,
                zw: self.rot_zw,
            },
            four_d: self.four_d,
            stereo: self.stereo,
        };
        let ctx = TesseractRenderContext::from_config(
            &self.cached_vertices,
            &self.cached_indices,
            &self.camera,
            config,
        );

        let transformed = ctx.transform_vertices();
        render_stereo_views(
            ui,
            rect,
            self.stereo.eye_separation,
            self.stereo.projection_distance,
            self.stereo.projection_mode,
            |painter, projector, view_rect| {
                ctx.render_edges(painter, projector, &transformed, view_rect);
            },
        );

        if show_debug {
            let right_rect = split_stereo_views(rect).1;
            let right_painter = ui.painter().with_clip_rect(right_rect);
            ctx.render_zone_labels(&right_painter, right_rect);
        }

        if self.show_controls {
            let right_rect = split_stereo_views(rect).1;
            let right_painter = ui.painter().with_clip_rect(right_rect);
            ctx.render_tetrahedron_gadget(&right_painter, right_rect, &self.tetrahedron_rotations);
        }
    }

    fn render_toy_menu(&self, painter: &egui::Painter, rect: egui::Rect) {
        let rot_label = if self.right_view_4d_rotation {
            "Rot:4D"
        } else {
            "Rot:3D"
        };
        // Indicator in top-right of right view (gray since not interactive)
        let gray = Some(label_inactive());
        render_tap_zone_label(painter, rect, Zone::NorthEast, rot_label, gray);
    }

    fn set_stereo_settings(&mut self, settings: &crate::render::StereoSettings) {
        self.stereo = *settings;
    }

    fn set_four_d_settings(&mut self, settings: &FourDSettings) {
        self.four_d = *settings;
    }

    fn handle_tap(&mut self, analysis: &TapAnalysis) {
        // Toggle 4D rotation mode on right view center tap
        if !analysis.is_left_view && analysis.zone == Zone::Center {
            self.right_view_4d_rotation = !self.right_view_4d_rotation;
            return;
        }

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
                self.apply_camera_action(CameraAction::MoveUp, move_speed);
            }
            if i.key_down(egui::Key::ArrowDown) {
                self.apply_camera_action(CameraAction::MoveDown, move_speed);
            }
            if i.key_down(egui::Key::ArrowLeft) {
                self.apply_camera_action(CameraAction::MoveLeft, move_speed);
            }
            if i.key_down(egui::Key::ArrowRight) {
                self.apply_camera_action(CameraAction::MoveRight, move_speed);
            }
            if i.key_down(egui::Key::PageUp) {
                self.apply_camera_action(CameraAction::MoveForward, move_speed);
            }
            if i.key_down(egui::Key::PageDown) {
                self.apply_camera_action(CameraAction::MoveBackward, move_speed);
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

    fn compass_vector(&self) -> Option<Vector4<f32>> {
        Some(-self.camera.position)
    }

    fn compass_reference_position(&self) -> Option<Vector4<f32>> {
        Some(self.camera.position)
    }

    fn compass_waypoints(&self) -> Vec<CompassWaypoint> {
        vec![
            CompassWaypoint {
                title: "Origin",
                position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            },
            CompassWaypoint {
                title: "TestPoint",
                position: Vector4::new(1.0, 2.0, 3.0, 4.0),
            },
        ]
    }

    fn compass_world_to_camera_frame(&self, world_vector: Vector4<f32>) -> Option<Vector4<f32>> {
        Some(self.camera.world_vector_to_camera_frame(world_vector))
    }

    fn zone_mode_for_view(&self, _is_left_view: bool) -> ZoneMode {
        self.zone_mode
    }

    fn clear_interaction_state(&mut self) {
        self.drag_state.clear();
    }
}
