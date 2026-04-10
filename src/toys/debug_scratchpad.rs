//! Minimal scratchpad toy for temporary experiments.

use eframe::egui;

use crate::render::{draw_background, FourDSettings, StereoSettings};
use crate::toy::Toy;
use crate::{DragView, TapAnalysis};

pub struct DebugScratchpadToy {
    visualization_rect: Option<egui::Rect>,
}

impl Default for DebugScratchpadToy {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugScratchpadToy {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            visualization_rect: None,
        }
    }
}

impl Toy for DebugScratchpadToy {
    fn name(&self) -> &'static str {
        "DebugScratchpad"
    }

    fn id(&self) -> &'static str {
        "debug_scratchpad"
    }

    fn reset(&mut self) {}

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("DebugScratchpad");
        ui.label("This toy is intentionally empty.");
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _show_debug: bool) {
        draw_background(ui, rect);
        self.visualization_rect = Some(rect);

        let painter = ui.painter().with_clip_rect(rect);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "DebugScratchpad",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );
    }

    fn handle_tap(&mut self, _analysis: &TapAnalysis) -> crate::toy::ViewAction {
        crate::toy::ViewAction::None
    }

    fn handle_drag(&mut self, _is_left_view: bool, _from: egui::Pos2, _to: egui::Pos2) {}

    fn handle_hold(&mut self, _analysis: &TapAnalysis) {}

    fn handle_drag_start(&mut self, _drag_view: DragView) {}

    fn handle_keyboard(&mut self, _ctx: &egui::Context) {}

    fn visualization_rect(&self) -> Option<egui::Rect> {
        self.visualization_rect
    }

    fn set_stereo_settings(&mut self, _settings: &StereoSettings) {}

    fn set_four_d_settings(&mut self, _settings: &FourDSettings) {}
}
