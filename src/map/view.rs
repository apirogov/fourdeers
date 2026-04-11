use eframe::egui;

use crate::camera::{Camera, Direction4D};
use crate::colors::LABEL_INACTIVE;
use crate::geometry::Bounds4D;
use crate::input::{zone_from_rect, zone_to_movement_action, Zone, ZoneMode};
use crate::map::{compute_bounds, MapRenderer};
use crate::render::{render_tap_zone_label, CompassFrameMode};
use crate::toy::ViewAction;

const MAP_HOLD_SPEED: f32 = 0.08;
const MAP_TAP_SPEED: f32 = 0.3;
const MAP_KEYBOARD_SPEED: f32 = 0.05;

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
    ) {
        let frame_label = self.frame_mode.display_label();
        render_tap_zone_label(left_painter, left_rect, Zone::South, frame_label, None);

        let labels_label = if self.renderer.labels_visible() {
            "Labels: On"
        } else {
            "Labels: Off"
        };
        render_tap_zone_label(left_painter, left_rect, Zone::NorthEast, labels_label, None);
        render_tap_zone_label(left_painter, left_rect, Zone::SouthEast, "Reset", None);

        let rot_label = if self.rotation_3d { "Rot:3D" } else { "Rot:4D" };
        render_tap_zone_label(
            right_painter,
            right_rect,
            Zone::NorthEast,
            rot_label,
            Some(LABEL_INACTIVE),
        );
    }

    pub fn handle_tap(
        &mut self,
        left_zone: Option<Zone>,
        right_rect: egui::Rect,
        pos: egui::Pos2,
        scene_camera: Option<&Camera>,
        waypoints: &[crate::toy::CompassWaypoint],
        geometry_bounds: Option<Bounds4D>,
    ) -> ViewAction {
        if let Some(wp_index) = self.renderer.find_tapped_waypoint(pos) {
            return ViewAction::SelectWaypoint(wp_index);
        }

        match left_zone {
            Some(Zone::South) => {
                self.frame_mode = self.frame_mode.other();
                return ViewAction::None;
            }
            Some(Zone::NorthEast) => {
                self.renderer.toggle_labels();
                return ViewAction::None;
            }
            Some(Zone::SouthEast) => {
                if let Some(camera) = scene_camera {
                    let bounds = compute_bounds(camera, waypoints, geometry_bounds);
                    self.renderer.reset_to_fit(camera, &bounds);
                }
                return ViewAction::None;
            }
            _ => {}
        }

        if right_rect.contains(pos) {
            let zone = zone_from_rect(right_rect, pos, ZoneMode::NineZones);
            if zone == Some(Zone::Center) {
                self.rotation_3d = !self.rotation_3d;
                return ViewAction::None;
            }
        }

        if let Some(action) = Self::tap_to_movement_action(right_rect, pos) {
            self.renderer.apply_action(action, MAP_TAP_SPEED);
        }

        ViewAction::None
    }

    pub fn handle_drag(&mut self, from: egui::Pos2, to: egui::Pos2) {
        let delta = to - from;
        if self.rotation_3d {
            self.renderer.rotate_3d(delta.x, delta.y);
        } else {
            self.renderer.rotate_4d(delta.x, delta.y);
        }
    }

    pub fn handle_hold(&mut self, right_rect: egui::Rect, pos: egui::Pos2) {
        if let Some(action) = Self::tap_to_movement_action(right_rect, pos) {
            self.renderer.apply_action(action, MAP_HOLD_SPEED);
        }
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let renderer = &mut self.renderer;
        crate::input::handle_movement_keys(ctx, MAP_KEYBOARD_SPEED, |action, speed| {
            renderer.apply_action(action, speed);
        });
    }

    fn tap_to_movement_action(right_rect: egui::Rect, pos: egui::Pos2) -> Option<Direction4D> {
        if !right_rect.contains(pos) {
            return None;
        }
        let zone = zone_from_rect(right_rect, pos, ZoneMode::NineZones)?;
        zone_to_movement_action(zone)
    }
}

impl Default for MapView {
    fn default() -> Self {
        Self::new()
    }
}
