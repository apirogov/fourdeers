//! Input handling for stereoscopic view tap zones.
//!
//! Provides shared keyboard movement handling for camera actions,
//! mapping arrow keys and PageUp/PageDown/Period/Comma to standard `CameraAction` variants.

use eframe::egui;

use crate::camera::CameraAction;

pub fn handle_movement_keys(
    ctx: &egui::Context,
    speed: f32,
    mut apply: impl FnMut(CameraAction, f32),
) {
    ctx.input(|i| {
        if i.key_down(egui::Key::ArrowUp) {
            apply(CameraAction::MoveUp, speed);
        }
        if i.key_down(egui::Key::ArrowDown) {
            apply(CameraAction::MoveDown, speed);
        }
        if i.key_down(egui::Key::ArrowLeft) {
            apply(CameraAction::MoveLeft, speed);
        }
        if i.key_down(egui::Key::ArrowRight) {
            apply(CameraAction::MoveRight, speed);
        }
        if i.key_down(egui::Key::PageUp) {
            apply(CameraAction::MoveForward, speed);
        }
        if i.key_down(egui::Key::PageDown) {
            apply(CameraAction::MoveBackward, speed);
        }
        if i.key_down(egui::Key::Period) {
            apply(CameraAction::MoveKata, speed);
        }
        if i.key_down(egui::Key::Comma) {
            apply(CameraAction::MoveAna, speed);
        }
    });
}

pub mod zone_debug;
pub mod zones;

pub use zone_debug::{render_zone_debug_overlay, ZoneDebugOptions};
pub use zones::{
    analyze_tap_in_stereo_view_with_modes, get_zone_from_rect, zone_to_movement_action, DragView,
    TapAnalysis, TetraId, Zone, ZoneMode,
};

/// Tracks the active drag gesture state.
#[derive(Default)]
pub struct DragState {
    pub drag_view: Option<DragView>,
}

impl DragState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn clear(&mut self) {
        self.drag_view = None;
    }
}
