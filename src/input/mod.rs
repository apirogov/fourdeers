//! Input handling for stereoscopic view tap zones and control overlay

pub mod overlay;
pub mod zone_debug;
pub mod zones;

pub use overlay::{ControlOverlay, DragHandler, TapAction, ZoneBindings};
pub use zone_debug::{
    get_cardinal_zone_center_with_offset, get_zone_center, render_zone_debug_overlay,
    ZoneDebugOptions,
};
pub use zones::{
    analyze_tap_in_stereo_view, get_zone_from_rect, DragView, TapAnalysis, TetraId, Zone, ZoneMode,
};
