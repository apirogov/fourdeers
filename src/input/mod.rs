//! Input handling for stereoscopic view tap zones and control overlay

pub mod overlay;
pub mod zones;

pub use overlay::{ControlOverlay, ControlOverlayBuilder, DragHandler, TapAction, ZoneBindings};
pub use zones::{
    analyze_tap_in_stereo_view, get_zone_from_rect, DragView, TapAnalysis, TetraId, Zone,
};
