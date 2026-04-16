use eframe::egui;

use crate::camera::Camera;
use crate::input::{
    handle_movement_keys, zone_to_movement_action, DragView, PointerAnalysis, HOLD_MOVE_SPEED,
    KEYBOARD_MOVE_SPEED, TAP_MOVE_SPEED,
};
use crate::render::{adjust_dichoptic_intensity, adjust_w_thickness};

pub struct CameraControls {
    pub rotation_3d: bool,
}

impl CameraControls {
    #[must_use]
    pub fn new(rotation_3d: bool) -> Self {
        Self { rotation_3d }
    }

    pub fn toggle_rotation_mode(&mut self) {
        self.rotation_3d = !self.rotation_3d;
    }

    pub fn handle_left_drag(
        w_thickness: &mut f32,
        dichoptic_intensity: &mut f32,
        delta: egui::Vec2,
        dt_scale: f32,
    ) {
        *w_thickness = adjust_w_thickness(*w_thickness, delta.x, dt_scale);
        *dichoptic_intensity = adjust_dichoptic_intensity(*dichoptic_intensity, delta.y, dt_scale);
    }

    pub fn handle_right_drag(&self, camera: &mut Camera, delta: egui::Vec2, dt_scale: f32) {
        if self.rotation_3d {
            camera.rotate(delta.x, delta.y, dt_scale);
        } else {
            camera.rotate_4d(delta.x, delta.y, dt_scale);
        }
    }

    pub fn handle_drag(
        &self,
        camera: &mut Camera,
        analysis: &PointerAnalysis,
        w_thickness: &mut f32,
        dichoptic_intensity: &mut f32,
    ) {
        let delta = analysis.drag_delta;
        match analysis.drag_view {
            Some(DragView::Left) => {
                Self::handle_left_drag(w_thickness, dichoptic_intensity, delta, analysis.dt_scale);
            }
            Some(DragView::Right) => {
                self.handle_right_drag(camera, delta, analysis.dt_scale);
            }
            None => {}
        }
    }

    pub fn handle_zone_movement(camera: &mut Camera, analysis: &PointerAnalysis) {
        if !analysis.is_left_view {
            if let Some(zone) = analysis.zone {
                if let Some(action) = zone_to_movement_action(zone) {
                    let speed = if analysis.is_hold {
                        HOLD_MOVE_SPEED * analysis.dt_scale
                    } else {
                        TAP_MOVE_SPEED
                    };
                    camera.apply_action(action, speed);
                }
            }
        }
    }

    pub fn handle_movement_keys(&self, ctx: &egui::Context, camera: &mut Camera, dt_scale: f32) {
        handle_movement_keys(ctx, KEYBOARD_MOVE_SPEED, dt_scale, |action, speed| {
            camera.apply_action(action, speed);
        });
    }
}

impl Default for CameraControls {
    fn default() -> Self {
        Self::new(true)
    }
}
