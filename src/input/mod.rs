//! Input handling for stereoscopic view tap zones

pub mod zone_debug;
pub mod zones;

pub use zone_debug::{render_zone_debug_overlay, ZoneDebugOptions};
pub use zones::{
    analyze_tap_in_stereo_view_with_modes, get_zone_from_rect, DragView, TapAnalysis, TetraId,
    Zone, ZoneMode,
};
