//! Input handling for stereoscopic view tap zones.
//!
//! Provides shared keyboard movement handling for camera actions,
//! mapping arrow keys and PageUp/PageDown/Period/Comma to standard `Direction4D` variants.

use eframe::egui;

use crate::camera::Direction4D;

/// Movement speed for tap/click actions.
pub const TAP_MOVE_SPEED: f32 = 0.15;
/// Movement speed for hold/long-press actions.
pub const HOLD_MOVE_SPEED: f32 = 0.08;
/// Movement speed for keyboard actions.
pub const KEYBOARD_MOVE_SPEED: f32 = 0.15;

pub fn handle_movement_keys(
    ctx: &egui::Context,
    speed: f32,
    dt_scale: f32,
    mut apply: impl FnMut(Direction4D, f32),
) {
    let scaled = speed * dt_scale;
    ctx.input(|i| {
        if i.key_down(egui::Key::ArrowUp) {
            apply(Direction4D::Up, scaled);
        }
        if i.key_down(egui::Key::ArrowDown) {
            apply(Direction4D::Down, scaled);
        }
        if i.key_down(egui::Key::ArrowLeft) {
            apply(Direction4D::Left, scaled);
        }
        if i.key_down(egui::Key::ArrowRight) {
            apply(Direction4D::Right, scaled);
        }
        if i.key_down(egui::Key::PageUp) {
            apply(Direction4D::Forward, scaled);
        }
        if i.key_down(egui::Key::PageDown) {
            apply(Direction4D::Backward, scaled);
        }
        if i.key_down(egui::Key::Period) {
            apply(Direction4D::Kata, scaled);
        }
        if i.key_down(egui::Key::Comma) {
            apply(Direction4D::Ana, scaled);
        }
    });
}

pub mod zone_debug;
pub mod zones;

pub use zone_debug::{render_zone_debug_overlay, ZoneDebugOptions};
pub use zones::{
    analyze_pointer_initial, zone_from_rect, zone_to_movement_action, DragView, PointerAnalysis,
    TetraId, Zone, ZoneMode,
};

/// Tracks the active drag gesture state.
#[derive(Debug, Clone, Default)]
pub struct DragState {
    pub drag_view: Option<DragView>,
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
