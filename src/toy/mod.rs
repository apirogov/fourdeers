//! Toy trait and common types for multi-app system

use eframe::egui;
use std::any::Any;

use crate::input::{DragView, TapAnalysis, TetraId, Zone};

pub mod manager;
pub mod registry;

pub use manager::ToyManager;

pub trait Toy: Any {
    fn name(&self) -> &str;
    fn id(&self) -> &str;

    fn reset(&mut self);

    fn render_sidebar(&mut self, ui: &mut egui::Ui);
    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool);
    fn render_overlay(&mut self, ui: &mut egui::Ui, left_rect: egui::Rect, right_rect: egui::Rect);

    fn handle_tap(&mut self, analysis: &TapAnalysis);
    fn handle_drag(&mut self, is_left_view: bool, from: egui::Pos2, to: egui::Pos2);
    fn handle_hold(&mut self, analysis: &TapAnalysis);
    fn handle_drag_start(&mut self, drag_view: DragView);

    fn handle_keyboard(&mut self, ctx: &egui::Context);

    fn get_visualization_rect(&self) -> Option<egui::Rect>;
    fn set_visualization_rect(&mut self, rect: egui::Rect);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct DragState {
    pub is_dragging: bool,
    pub is_drag_mode: bool,
    pub drag_view: Option<crate::input::DragView>,
    pub last_mouse_pos: Option<egui::Pos2>,
    pub dragging_tetrahedron: Option<TetraId>,
    pub last_tetra_drag_pos: Option<egui::Pos2>,
    pub last_tap_pos: Option<egui::Pos2>,
    pub last_tap_zone: Option<Zone>,
    pub last_tap_view_left: bool,
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            is_dragging: false,
            is_drag_mode: false,
            drag_view: None,
            last_mouse_pos: None,
            dragging_tetrahedron: None,
            last_tetra_drag_pos: None,
            last_tap_pos: None,
            last_tap_zone: None,
            last_tap_view_left: false,
        }
    }
}

impl DragState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.is_dragging = false;
        self.drag_view = None;
        self.last_mouse_pos = None;
        self.dragging_tetrahedron = None;
        self.last_tetra_drag_pos = None;
    }
}
