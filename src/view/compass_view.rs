use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::camera::ROTATION_SENSITIVITY;
use crate::input::{PointerAnalysis, Zone};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_views, render_tap_zone_label,
    render_tetrahedron_with_projector, CompassFrameMode, ProjectionMode, StereoSettings,
};
use crate::tetrahedron::{format_magnitude, magnitude_4d, TetrahedronGadget};
use crate::toy::{CompassWaypoint, ViewAction};

pub struct CompassView {
    pub rotation: UnitQuaternion<f32>,
    pub waypoint_index: usize,
    pub frame_mode: CompassFrameMode,
}

impl CompassView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rotation: UnitQuaternion::identity(),
            waypoint_index: 0,
            frame_mode: CompassFrameMode::World,
        }
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        vector_4d: Vector4<f32>,
        waypoint_title: &str,
        stereo: StereoSettings,
    ) {
        draw_background(ui, rect);
        draw_center_divider(ui, rect);

        let magnitude_label = format_magnitude(magnitude_4d(vector_4d));
        let gadget =
            TetrahedronGadget::from_4d_vector_with_quaternion(vector_4d, self.rotation, 1.0)
                .with_base_label(magnitude_label)
                .with_tip_label(waypoint_title);

        render_stereo_views(
            ui,
            rect,
            stereo.eye_separation,
            stereo.projection_distance,
            ProjectionMode::Orthographic,
            |painter, projector, _view_rect| {
                let mut batch = crate::render::batch::LineBatch::new(1.0);
                render_tetrahedron_with_projector(
                    &mut batch,
                    painter,
                    &gadget,
                    projector,
                    self.frame_mode,
                );
                batch.submit(painter);
            },
        );
    }

    pub fn render_overlays(
        &self,
        left_painter: &egui::Painter,
        left_rect: egui::Rect,
        right_painter: &egui::Painter,
        right_rect: egui::Rect,
        _w_thickness: f32,
    ) {
        let frame_label = self.frame_mode.display_label();
        render_tap_zone_label(left_painter, left_rect, Zone::South, frame_label, None);
        render_tap_zone_label(right_painter, right_rect, Zone::North, "Jump", None);
        render_tap_zone_label(right_painter, right_rect, Zone::South, "Prev", None);
        render_tap_zone_label(right_painter, right_rect, Zone::SouthEast, "Next", None);
    }

    pub fn handle_pointer(
        &mut self,
        analysis: &PointerAnalysis,
        waypoints_len: usize,
    ) -> ViewAction {
        if let Some(zone) = analysis.zone {
            // Only allow toggle actions on tap (not hold)
            if !analysis.is_hold {
                if analysis.is_left_view {
                    if zone == Zone::South {
                        self.frame_mode = self.frame_mode.other();
                        return ViewAction::None;
                    }
                } else {
                    if zone == Zone::South {
                        self.cycle_waypoint(-1, waypoints_len);
                    }
                    if zone == Zone::SouthEast {
                        self.cycle_waypoint(1, waypoints_len);
                    }
                }
            }
        }

        ViewAction::None
    }

    pub fn handle_drag(&mut self, analysis: &PointerAnalysis) -> ViewAction {
        let delta = analysis.drag_delta;
        let s = ROTATION_SENSITIVITY * analysis.dt_scale;
        let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), delta.x * s);
        let pitch_rot = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * s);
        let incremental = pitch_rot * yaw_rot;
        self.rotation = incremental * self.rotation;
        ViewAction::None
    }

    pub fn clamped_waypoint_index(&mut self, waypoints_len: usize) -> usize {
        if waypoints_len == 0 {
            return 0;
        }
        if self.waypoint_index >= waypoints_len {
            self.waypoint_index = 0;
        }
        self.waypoint_index
    }

    pub fn cycle_waypoint(&mut self, direction: i32, waypoints_len: usize) {
        if waypoints_len == 0 {
            return;
        }
        let len = waypoints_len as i32;
        let idx = self.waypoint_index as i32;
        self.waypoint_index = (idx + direction).rem_euclid(len) as usize;
    }

    pub fn reset_waypoint(&mut self) {
        self.waypoint_index = 0;
    }

    pub fn set_waypoint_index(&mut self, index: usize) {
        self.waypoint_index = index;
    }

    pub fn current_waypoint(&mut self, waypoints: &[CompassWaypoint]) -> Option<CompassWaypoint> {
        if waypoints.is_empty() {
            return None;
        }
        let idx = self.clamped_waypoint_index(waypoints.len());
        Some(waypoints[idx].clone())
    }
}

impl Default for CompassView {
    fn default() -> Self {
        Self::new()
    }
}
