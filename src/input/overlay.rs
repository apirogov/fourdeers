//! Pluggable control overlay for zone-based input

use std::collections::HashMap;

use eframe::egui;

use super::zones::{TapAnalysis, Zone, ZoneMode};

pub type TapAction = Box<dyn FnMut()>;
pub type DragHandler = Box<dyn FnMut(egui::Pos2, egui::Pos2)>;

#[derive(Default)]
pub struct ZoneBindings {
    bindings: HashMap<Zone, TapAction>,
}


impl ZoneBindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zone(mut self, zone: Zone, action: TapAction) -> Self {
        self.bindings.insert(zone, action);
        self
    }

    pub fn north(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::North, action);
        self
    }

    pub fn east(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::East, action);
        self
    }

    pub fn south(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::South, action);
        self
    }

    pub fn west(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::West, action);
        self
    }

    pub fn north_west(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::NorthWest, action);
        self
    }

    pub fn north_east(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::NorthEast, action);
        self
    }

    pub fn center(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::Center, action);
        self
    }

    pub fn south_west(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::SouthWest, action);
        self
    }

    pub fn south_east(mut self, action: TapAction) -> Self {
        self.bindings.insert(Zone::SouthEast, action);
        self
    }

    pub fn trigger(&mut self, zone: Zone) {
        if let Some(action) = self.bindings.get_mut(&zone) {
            action();
        }
    }

    pub fn has_binding(&self, zone: Zone) -> bool {
        self.bindings.contains_key(&zone)
    }
}

pub struct ControlOverlay {
    zone_mode: ZoneMode,
    left_bindings: ZoneBindings,
    right_bindings: ZoneBindings,
    left_drag_handler: Option<DragHandler>,
    right_drag_handler: Option<DragHandler>,
}

impl Default for ControlOverlay {
    fn default() -> Self {
        Self {
            zone_mode: ZoneMode::default(),
            left_bindings: ZoneBindings::new(),
            right_bindings: ZoneBindings::new(),
            left_drag_handler: None,
            right_drag_handler: None,
        }
    }
}

impl ControlOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zone_mode(mut self, mode: ZoneMode) -> Self {
        self.zone_mode = mode;
        self
    }

    pub fn left_bindings(mut self, bindings: ZoneBindings) -> Self {
        self.left_bindings = bindings;
        self
    }

    pub fn right_bindings(mut self, bindings: ZoneBindings) -> Self {
        self.right_bindings = bindings;
        self
    }

    pub fn left_drag(mut self, handler: DragHandler) -> Self {
        self.left_drag_handler = Some(handler);
        self
    }

    pub fn right_drag(mut self, handler: DragHandler) -> Self {
        self.right_drag_handler = Some(handler);
        self
    }

    pub fn get_zone_mode(&self) -> ZoneMode {
        self.zone_mode
    }

    pub fn trigger(&mut self, analysis: &TapAnalysis) {
        let bindings = if analysis.is_left_view {
            &mut self.left_bindings
        } else {
            &mut self.right_bindings
        };
        bindings.trigger(analysis.zone);
    }

    pub fn handle_drag(&mut self, is_left_view: bool, from: egui::Pos2, to: egui::Pos2) {
        let handler = if is_left_view {
            &mut self.left_drag_handler
        } else {
            &mut self.right_drag_handler
        };
        if let Some(handler) = handler {
            handler(from, to);
        }
    }

    pub fn left_bindings_mut(&mut self) -> &mut ZoneBindings {
        &mut self.left_bindings
    }

    pub fn right_bindings_mut(&mut self) -> &mut ZoneBindings {
        &mut self.right_bindings
    }
}

pub struct ControlOverlayBuilder {
    overlay: ControlOverlay,
}

impl ControlOverlayBuilder {
    pub fn new() -> Self {
        Self {
            overlay: ControlOverlay::new(),
        }
    }

    pub fn zone_mode(mut self, mode: ZoneMode) -> Self {
        self.overlay.zone_mode = mode;
        self
    }

    pub fn left_zone(mut self, zone: Zone, action: TapAction) -> Self {
        self.overlay.left_bindings.bindings.insert(zone, action);
        self
    }

    pub fn right_zone(mut self, zone: Zone, action: TapAction) -> Self {
        self.overlay.right_bindings.bindings.insert(zone, action);
        self
    }

    pub fn left_drag(mut self, handler: DragHandler) -> Self {
        self.overlay.left_drag_handler = Some(handler);
        self
    }

    pub fn right_drag(mut self, handler: DragHandler) -> Self {
        self.overlay.right_drag_handler = Some(handler);
        self
    }

    pub fn build(self) -> ControlOverlay {
        self.overlay
    }
}

impl Default for ControlOverlayBuilder {
    fn default() -> Self {
        Self::new()
    }
}
