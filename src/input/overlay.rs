//! Pluggable control overlay for zone-based input

use eframe::egui;

use super::zones::{TapAnalysis, Zone};

pub type TapAction = Box<dyn FnMut()>;
pub type DragHandler = Box<dyn FnMut(egui::Pos2, egui::Pos2)>;

pub struct ZoneBindings {
    north: Option<TapAction>,
    east: Option<TapAction>,
    south: Option<TapAction>,
    west: Option<TapAction>,
}

impl Default for ZoneBindings {
    fn default() -> Self {
        Self {
            north: None,
            east: None,
            south: None,
            west: None,
        }
    }
}

impl ZoneBindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn north(mut self, action: TapAction) -> Self {
        self.north = Some(action);
        self
    }

    pub fn east(mut self, action: TapAction) -> Self {
        self.east = Some(action);
        self
    }

    pub fn south(mut self, action: TapAction) -> Self {
        self.south = Some(action);
        self
    }

    pub fn west(mut self, action: TapAction) -> Self {
        self.west = Some(action);
        self
    }

    pub fn trigger(&mut self, zone: Zone) {
        let action = match zone {
            Zone::North => &mut self.north,
            Zone::East => &mut self.east,
            Zone::South => &mut self.south,
            Zone::West => &mut self.west,
        };
        if let Some(action) = action {
            action();
        }
    }
}

pub struct ControlOverlay {
    left_bindings: ZoneBindings,
    right_bindings: ZoneBindings,
    left_drag_handler: Option<DragHandler>,
    right_drag_handler: Option<DragHandler>,
}

impl Default for ControlOverlay {
    fn default() -> Self {
        Self {
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

    pub fn left_zone(mut self, zone: Zone, action: TapAction) -> Self {
        match zone {
            Zone::North => self.overlay.left_bindings.north = Some(action),
            Zone::East => self.overlay.left_bindings.east = Some(action),
            Zone::South => self.overlay.left_bindings.south = Some(action),
            Zone::West => self.overlay.left_bindings.west = Some(action),
        }
        self
    }

    pub fn right_zone(mut self, zone: Zone, action: TapAction) -> Self {
        match zone {
            Zone::North => self.overlay.right_bindings.north = Some(action),
            Zone::East => self.overlay.right_bindings.east = Some(action),
            Zone::South => self.overlay.right_bindings.south = Some(action),
            Zone::West => self.overlay.right_bindings.west = Some(action),
        }
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
