//! Toy trait and common types for multi-app system

use eframe::egui;
use nalgebra::Vector4;

use crate::camera::Camera;
use crate::geometry::Bounds4D;
use crate::input::{DragView, TapAnalysis, ZoneMode};
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
    SwitchView(String),
    ToggleMenu,
    SelectWaypoint(usize),
}

/// The extension point for adding new interactive toys to the app.
///
/// Each toy provides its own scene rendering, sidebar controls, input handling,
/// and compass/map integration.
pub trait Toy {
    /// Human-readable name for display in the UI.
    fn name(&self) -> &str;
    /// Unique identifier used for toy switching and persistence.
    fn id(&self) -> &str;

    /// Reset the toy to its default state.
    fn reset(&mut self);

    /// Render the toy's sidebar controls in the menu panel.
    fn render_sidebar(&mut self, ui: &mut egui::Ui);
    /// Render the toy's 3D/4D scene into the given rect.
    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool);
    /// Handle a single-finger tap in a zone. Returns an action for the app to process.
    fn handle_tap(&mut self, analysis: &TapAnalysis) -> ViewAction;
    /// Handle an ongoing drag gesture.
    fn handle_drag(&mut self, is_left_view: bool, from: egui::Pos2, to: egui::Pos2);
    /// Handle a held tap (long press).
    fn handle_hold(&mut self, analysis: &TapAnalysis);
    /// Called when a drag gesture starts.
    fn handle_drag_start(&mut self, drag_view: DragView);

    /// Handle keyboard input (called each frame).
    fn handle_keyboard(&mut self, ctx: &egui::Context);

    /// Return the rect where the toy renders, if any.
    fn visualization_rect(&self) -> Option<egui::Rect>;

    /// The 4D vector displayed by the compass gadget, if any.
    fn compass_vector(&self) -> Option<Vector4<f32>> {
        None
    }

    /// The reference position for the compass (usually the camera position).
    fn compass_reference_position(&self) -> Option<Vector4<f32>> {
        None
    }

    /// Transform a world-space 4D vector into camera-frame coordinates for the compass.
    fn compass_world_to_camera_frame(&self, _world_vector: Vector4<f32>) -> Option<Vector4<f32>> {
        None
    }

    /// Named 4D positions shown as waypoints in compass and map views.
    fn compass_waypoints(&self) -> Vec<CompassWaypoint> {
        Vec::new()
    }

    /// The camera used for map rendering, if this toy supports the map view.
    fn map_camera(&self) -> Option<&Camera> {
        None
    }

    /// Axis-aligned bounding box of the scene geometry in 4D, for map bounds computation.
    fn scene_geometry_bounds(&self) -> Option<Bounds4D> {
        None
    }

    /// Waypoints shown on the map view (defaults to compass waypoints).
    fn map_waypoints(&self) -> Vec<CompassWaypoint> {
        self.compass_waypoints()
    }

    /// Override the zone mode for a given view half.
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

    /// Switch to a different view by id. No-op if the view doesn't exist.
    fn set_active_view(&mut self, _id: &str) {}
    /// The id of the currently active view.
    #[must_use]
    fn active_view_id(&self) -> &str {
        "scene"
    }
    /// List of available views as (id, display_name) pairs.
    #[must_use]
    fn available_views(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }
}
