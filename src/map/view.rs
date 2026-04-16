use eframe::egui;

use crate::camera::Camera;
use crate::colors::LABEL_INACTIVE;
use crate::geometry::Bounds4D;
use crate::input::{CameraControls, PointerAnalysis, Zone};
use crate::map::{compute_bounds, MapRenderer};
use crate::render::{render_tap_zone_label, CompassFrameMode};
use crate::toy::ViewAction;

pub struct MapView {
    pub renderer: MapRenderer,
    pub frame_mode: CompassFrameMode,
    pub controls: CameraControls,
}

impl MapView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            renderer: MapRenderer::new(),
            frame_mode: CompassFrameMode::World,
            controls: CameraControls::new(true),
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        params: &crate::map::MapRenderParams<'_>,
    ) {
        self.renderer.render(ui, rect, params);
    }

    pub fn render_overlays(
        &self,
        left_painter: &egui::Painter,
        left_rect: egui::Rect,
        right_painter: &egui::Painter,
        right_rect: egui::Rect,
        w_thickness: f32,
    ) {
        let frame_label = self.frame_mode.display_label();
        render_tap_zone_label(left_painter, left_rect, Zone::South, frame_label, None);

        let labels_label = if self.renderer.labels_visible() {
            "Labels: On"
        } else {
            "Labels: Off"
        };
        render_tap_zone_label(left_painter, left_rect, Zone::North, labels_label, None);
        render_tap_zone_label(left_painter, left_rect, Zone::SouthEast, "Reset", None);

        let rot_label = if self.controls.rotation_3d {
            "Rot:3D"
        } else {
            "Rot:4D"
        };
        render_tap_zone_label(
            right_painter,
            right_rect,
            Zone::NorthEast,
            rot_label,
            Some(LABEL_INACTIVE),
        );

        let w_label = format!("WØ: {:.1}", w_thickness);
        render_tap_zone_label(
            right_painter,
            right_rect,
            Zone::SouthEast,
            &w_label,
            Some(LABEL_INACTIVE),
        );
    }

    pub fn handle_pointer(
        &mut self,
        analysis: &PointerAnalysis,
        scene_camera: Option<&Camera>,
        waypoints: &[crate::toy::CompassWaypoint],
        geometry_bounds: Option<Bounds4D>,
    ) -> ViewAction {
        if !analysis.is_hold {
            if let Some(idx) = self.renderer.find_tapped_waypoint(analysis.tap_pos) {
                return ViewAction::SelectWaypoint(idx);
            }
        }

        if analysis.is_left_view {
            if let Some(zone) = analysis.zone {
                if !analysis.is_hold {
                    match zone {
                        Zone::South => {
                            self.frame_mode = self.frame_mode.other();
                            return ViewAction::None;
                        }
                        Zone::North => {
                            self.renderer.toggle_labels();
                            return ViewAction::None;
                        }
                        Zone::SouthEast => {
                            if let Some(camera) = scene_camera {
                                let bounds = compute_bounds(camera, waypoints, geometry_bounds);
                                self.renderer.reset_to_fit(camera, &bounds);
                            }
                            return ViewAction::None;
                        }
                        _ => {}
                    }
                }
            }
        } else {
            if !analysis.is_hold && analysis.zone == Some(Zone::Center) {
                self.controls.toggle_rotation_mode();
                return ViewAction::None;
            }

            CameraControls::handle_zone_movement(self.renderer.camera_mut(), analysis);
        }

        ViewAction::None
    }

    pub fn handle_drag(
        &mut self,
        analysis: &PointerAnalysis,
        w_thickness: &mut f32,
        dichoptic_intensity: &mut f32,
    ) -> ViewAction {
        self.controls.handle_drag(
            self.renderer.camera_mut(),
            analysis,
            w_thickness,
            dichoptic_intensity,
        );
        ViewAction::None
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context, dt_scale: f32) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F) {
                self.frame_mode = self.frame_mode.other();
            }
            if i.key_pressed(egui::Key::L) {
                self.renderer.toggle_labels();
            }
        });

        self.controls
            .handle_movement_keys(ctx, self.renderer.camera_mut(), dt_scale);
    }
}

impl Default for MapView {
    fn default() -> Self {
        Self::new()
    }
}
