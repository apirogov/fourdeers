//! Application state

use eframe::egui;
use nalgebra::UnitQuaternion;

use crate::camera::Camera;
use crate::input::{CameraAction, Zone};

/// Which view is being dragged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragView {
    Left,
    Right,
}

/// Identifier for each tetrahedron gadget (4 per view)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TetraId {
    pub is_left_view: bool,
    pub zone: Zone,
}

/// Application state for UI
pub struct AppState {
    pub camera: Camera,

    pub rot_xy: f32,
    pub rot_xz: f32,
    pub rot_yz: f32,
    pub rot_xw: f32,
    pub rot_yw: f32,
    pub rot_zw: f32,

    pub is_dragging: bool,
    pub drag_view: Option<DragView>,
    pub last_mouse_pos: Option<egui::Pos2>,

    pub w_thickness: f32,
    pub eye_separation: f32,
    pub projection_distance: f32,
    pub w_min: f32,
    pub w_max: f32,

    pub show_debug: bool,

    pub last_tap_pos: Option<egui::Pos2>,
    pub last_tap_zone: Option<Zone>,
    pub last_tap_view_left: bool,

    pub visualization_rect: Option<egui::Rect>,

    pub held_action: Option<CameraAction>,
    pub is_drag_mode: bool,

    /// User-adjustable rotation quaternion for each tetrahedron
    pub tetrahedron_rotations: std::collections::HashMap<TetraId, UnitQuaternion<f32>>,
    /// Whether we're currently dragging a tetrahedron
    pub dragging_tetrahedron: Option<TetraId>,
    /// Last mouse position for tetrahedron dragging
    pub last_tetra_drag_pos: Option<egui::Pos2>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            rot_xy: 0.0,
            rot_xz: 0.0,
            rot_yz: 0.0,
            rot_xw: 0.0,
            rot_yw: 0.0,
            rot_zw: 0.0,
            is_dragging: false,
            drag_view: None,
            last_mouse_pos: None,
            w_thickness: 2.5,
            eye_separation: 0.3,
            projection_distance: 3.0,
            w_min: -2.0,
            w_max: 2.0,
            show_debug: false,
            last_tap_pos: None,
            last_tap_zone: None,
            last_tap_view_left: false,
            visualization_rect: None,
            held_action: None,
            is_drag_mode: false,
            tetrahedron_rotations: std::collections::HashMap::new(),
            dragging_tetrahedron: None,
            last_tetra_drag_pos: None,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.camera.reset();
        self.rot_xy = 0.0;
        self.rot_xz = 0.0;
        self.rot_yz = 0.0;
        self.rot_xw = 0.0;
        self.rot_yw = 0.0;
        self.rot_zw = 0.0;
    }

    /// Get tetrahedron rotation quaternion, defaulting to identity
    pub fn get_tetrahedron_rotation(&self, id: TetraId) -> UnitQuaternion<f32> {
        self.tetrahedron_rotations
            .get(&id)
            .copied()
            .unwrap_or_else(UnitQuaternion::identity)
    }

    /// Set tetrahedron rotation quaternion
    pub fn set_tetrahedron_rotation(&mut self, id: TetraId, rotation: UnitQuaternion<f32>) {
        self.tetrahedron_rotations.insert(id, rotation);
    }

    /// Reset all tetrahedron rotations (called when camera moves)
    pub fn reset_tetrahedron_rotations(&mut self) {
        self.tetrahedron_rotations.clear();
    }
}
