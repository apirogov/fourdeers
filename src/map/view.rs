use eframe::egui;

use crate::camera::Camera;
use crate::colors::LABEL_INACTIVE;
use crate::geometry::Bounds4D;
use crate::input::{
    zone_to_movement_action, DragView, PointerAnalysis, Zone, HOLD_MOVE_SPEED, TAP_MOVE_SPEED,
};
use crate::map::{compute_bounds, MapRenderer};
use crate::render::{adjust_w_thickness, render_tap_zone_label, CompassFrameMode};
use crate::toy::ViewAction;

pub struct MapView {
    pub renderer: MapRenderer,
    pub frame_mode: CompassFrameMode,
    pub rotation_3d: bool,
}

impl MapView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            renderer: MapRenderer::new(),
            frame_mode: CompassFrameMode::World,
            rotation_3d: true,
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

        let rot_label = if self.rotation_3d { "Rot:3D" } else { "Rot:4D" };
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
        if analysis.is_left_view {
            if let Some(zone) = analysis.zone {
                // Only allow toggle actions on tap (not hold)
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
            // Rotation toggle only on tap (not hold)
            if !analysis.is_hold && analysis.zone == Some(Zone::Center) {
                self.rotation_3d = !self.rotation_3d;
                return ViewAction::None;
            }

            if !analysis.is_hold {
                if let Some(idx) = self.renderer.find_tapped_waypoint(analysis.tap_pos) {
                    return ViewAction::SelectWaypoint(idx);
                }
            }

            if let Some(zone) = analysis.zone {
                if let Some(action) = zone_to_movement_action(zone) {
                    let speed = if analysis.is_hold {
                        HOLD_MOVE_SPEED
                    } else {
                        TAP_MOVE_SPEED
                    };
                    self.renderer.apply_action(action, speed);
                }
            }
        }

        ViewAction::None
    }

    pub fn handle_drag(&mut self, analysis: &PointerAnalysis, w_thickness: &mut f32) -> ViewAction {
        let delta = analysis.drag_delta;

        match analysis.drag_view {
            Some(DragView::Left) => {
                *w_thickness = adjust_w_thickness(*w_thickness, delta.x);
            }
            Some(DragView::Right) => {
                if self.rotation_3d {
                    self.renderer.rotate_3d(delta.x, delta.y);
                } else {
                    self.renderer.rotate_4d(delta.x, delta.y);
                }
            }
            None => {}
        }
        ViewAction::None
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F) {
                self.frame_mode = self.frame_mode.other();
            }
            if i.key_pressed(egui::Key::L) {
                self.renderer.toggle_labels();
            }
        });
    }
}

impl Default for MapView {
    fn default() -> Self {
        Self::new()
    }
}
