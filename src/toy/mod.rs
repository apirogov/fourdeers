//! Toy trait and common types for multi-app system

use eframe::egui;
use nalgebra::Vector4;

use crate::input::{DragView, PointerAnalysis, ZoneMode};
use crate::render::{FourDSettings, StereoSettings};

pub mod manager;
pub mod registry;

pub use manager::ToyManager;

/// A named 4D position shown in the compass view.
#[derive(Clone)]
pub struct CompassWaypoint {
    pub title: &'static str,
    pub position: Vector4<f32>,
}

/// Action returned by a view's input handler for the app to process.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ViewAction {
    #[default]
    None,
    ToggleMenu,
    SelectWaypoint(usize),
}

/// The extension point for adding new interactive toys to the app.
///
/// Each toy provides its own scene rendering, sidebar controls, input handling,
/// and view management. The toy owns its views (scene, map, compass) and dispatches
/// internally based on the active view.
pub trait Toy {
    /// Human-readable name for display in the UI.
    fn name(&self) -> &str;
    /// Unique identifier used for toy switching and persistence.
    fn id(&self) -> &str;

    /// Reset the toy to its default state.
    fn reset(&mut self);

    /// Render the toy's sidebar controls in the menu panel.
    fn render_sidebar(&mut self, ui: &mut egui::Ui);
    /// Render the toy's active view into the given rect.
    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool);
    /// Handle pointer events (tap, hold) with unified analysis.
    fn handle_pointer(&mut self, analysis: PointerAnalysis) -> ViewAction;
    /// Handle an ongoing drag gesture, directly mutating w_thickness.
    fn handle_drag(&mut self, analysis: PointerAnalysis, w_thickness: &mut f32) -> ViewAction;
    /// Called when a drag gesture starts.
    fn handle_drag_start(&mut self, drag_view: DragView);

    /// Handle keyboard input (called each frame).
    fn handle_keyboard(&mut self, ctx: &egui::Context);

    /// Override the zone mode for a given view half (used for debug overlay).
    fn zone_mode_for_view(&self, _is_left_view: bool) -> ZoneMode {
        ZoneMode::default()
    }

    /// Clear any ongoing interaction state (e.g. after a mode switch).
    fn clear_interaction_state(&mut self) {}

    /// Render overlay labels (navigation, view-specific controls) on both stereo halves.
    fn render_view_overlays(
        &self,
        _left_painter: &egui::Painter,
        _left_rect: egui::Rect,
        _right_painter: &egui::Painter,
        _right_rect: egui::Rect,
    ) {
    }

    /// Apply stereo settings from the shared controls.
    fn set_stereo_settings(&mut self, _settings: &StereoSettings) {}
    /// Apply 4D visualization settings from the shared controls.
    fn set_four_d_settings(&mut self, _settings: &FourDSettings) {}
}
