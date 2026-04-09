//! Input handling for stereoscopic view tap zones.
//!
//! Provides shared keyboard movement handling for camera actions,
//! mapping arrow keys and PageUp/PageDown/Period/Comma to standard `Direction4D` variants.

use eframe::egui;

use crate::camera::Direction4D;

pub fn handle_movement_keys(
    ctx: &egui::Context,
    speed: f32,
    mut apply: impl FnMut(Direction4D, f32),
) {
    ctx.input(|i| {
        if i.key_down(egui::Key::ArrowUp) {
            apply(Direction4D::Up, speed);
        }
        if i.key_down(egui::Key::ArrowDown) {
            apply(Direction4D::Down, speed);
        }
        if i.key_down(egui::Key::ArrowLeft) {
            apply(Direction4D::Left, speed);
        }
        if i.key_down(egui::Key::ArrowRight) {
            apply(Direction4D::Right, speed);
        }
        if i.key_down(egui::Key::PageUp) {
            apply(Direction4D::Forward, speed);
        }
        if i.key_down(egui::Key::PageDown) {
            apply(Direction4D::Backward, speed);
        }
        if i.key_down(egui::Key::Period) {
            apply(Direction4D::Kata, speed);
        }
        if i.key_down(egui::Key::Comma) {
            apply(Direction4D::Ana, speed);
        }
    });
}

pub mod zone_debug;
pub mod zones;

pub use zone_debug::{render_zone_debug_overlay, ZoneDebugOptions};
pub use zones::{
    analyze_tap_in_stereo_view_with_modes, zone_from_rect, zone_to_movement_action, DragView,
    TapAnalysis, TetraId, Zone, ZoneMode,
};

/// Tracks the active drag gesture state.
pub struct DragState {
    pub drag_view: Option<DragView>,
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}

impl DragState {
    #[must_use]
    pub const fn new() -> Self {
        Self { drag_view: None }
    }

    pub const fn clear(&mut self) {
        self.drag_view = None;
    }
}
