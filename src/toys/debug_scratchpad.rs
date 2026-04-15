//! Minimal scratchpad toy for temporary experiments.

use eframe::egui;

use crate::input::PointerAnalysis;
use crate::render::{draw_background, FourDSettings, StereoSettings};
use crate::toy::Toy;
use crate::DragView;

#[derive(Default)]
pub struct DebugScratchpadToy;

impl DebugScratchpadToy {
    #[must_use]
    pub const fn new() -> Self {
        Self
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

        let painter = ui.painter().with_clip_rect(rect);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "DebugScratchpad",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );
    }

    fn handle_pointer(&mut self, _analysis: PointerAnalysis) -> crate::toy::ViewAction {
        crate::toy::ViewAction::None
    }

    fn handle_drag(
        &mut self,
        _analysis: PointerAnalysis,
        _w_thickness: &mut f32,
        _dichoptic_intensity: &mut f32,
    ) -> crate::toy::ViewAction {
        crate::toy::ViewAction::None
    }

    fn handle_drag_start(&mut self, _drag_view: DragView) {}

    fn handle_keyboard(&mut self, _ctx: &egui::Context, _dt_scale: f32) {}

    fn set_stereo_settings(&mut self, _settings: &StereoSettings) {}

    fn set_four_d_settings(&mut self, _settings: &FourDSettings) {}
}
