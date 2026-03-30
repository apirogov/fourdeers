//! Toy trait and common types for multi-app system

use eframe::egui;
use nalgebra::Vector4;

use crate::input::{DragView, TapAnalysis, ZoneMode};
use crate::render::{FourDSettings, StereoSettings};

pub mod manager;
pub mod registry;

pub use manager::ToyManager;

pub trait Toy {
    fn name(&self) -> &str;
    fn id(&self) -> &str;

    fn reset(&mut self);

    fn render_sidebar(&mut self, ui: &mut egui::Ui);
    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool);
    fn handle_tap(&mut self, analysis: &TapAnalysis);
    fn handle_drag(&mut self, is_left_view: bool, from: egui::Pos2, to: egui::Pos2);
    fn handle_hold(&mut self, analysis: &TapAnalysis);
    fn handle_drag_start(&mut self, drag_view: DragView);

    fn handle_keyboard(&mut self, ctx: &egui::Context);

    fn get_visualization_rect(&self) -> Option<egui::Rect>;
    fn compass_vector(&self) -> Option<Vector4<f32>> {
        None
    }
    fn zone_mode_for_view(&self, _is_left_view: bool) -> ZoneMode {
        ZoneMode::default()
    }

    fn clear_interaction_state(&mut self) {}

    fn render_toy_menu(&self, _painter: &egui::Painter, _rect: egui::Rect) {}
    fn set_stereo_settings(&mut self, _settings: &StereoSettings) {}
    fn set_four_d_settings(&mut self, _settings: &FourDSettings) {}
}

#[derive(Default)]
pub struct DragState {
    pub drag_view: Option<crate::input::DragView>,
}

impl DragState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.drag_view = None;
    }
}
