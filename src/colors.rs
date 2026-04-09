//! Color constants for the application
//!
//! All values are pre-computed `const` using `from_rgba_premultiplied` for alpha
//! colors, avoiding runtime overhead. Where alpha varies dynamically, callers
//! use `Color32::from_rgba_unmultiplied` directly.

use egui::Color32;

/// Clamp a float to `0..=255` and convert to `u8` for color channel construction.
#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub const fn to_u8(v: f32) -> u8 {
    if v < 0.0 {
        0
    } else if v > 255.0 {
        255
    } else {
        v as u8
    }
}

// ============================================================================
// UI / Labels
// ============================================================================

pub const LABEL_DEFAULT: Color32 = Color32::from_rgb(255, 180, 80);
pub const LABEL_INACTIVE: Color32 = Color32::from_rgb(180, 180, 180);

// ============================================================================
// Text
// ============================================================================

pub const TEXT_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(230, 230, 230, 255);
pub const TEXT_DIM: Color32 = Color32::from_rgba_premultiplied(118, 118, 118, 150);

// ============================================================================
// Graphics (Arrows / 3D)
// ============================================================================

pub const ARROW_PRIMARY: Color32 = Color32::from_rgb(255, 150, 50);
pub const ARROW_GLOW: Color32 = Color32::from_rgba_premultiplied(180, 106, 35, 180);
pub const ARROW_TIP: Color32 = Color32::from_rgb(255, 200, 100);
pub const ARROW_FORWARD: Color32 = Color32::from_rgb(100, 220, 255);

// ============================================================================
// 4D Objects
// ============================================================================

pub const OBJECT_TINT_POSITIVE: Color32 = Color32::from_rgba_premultiplied(106, 155, 106, 180);
pub const OBJECT_TINT_NEGATIVE: Color32 = Color32::from_rgba_premultiplied(118, 173, 118, 200);

// ============================================================================
// Backgrounds
// ============================================================================

pub const VIEWPORT_BG: Color32 = Color32::from_rgb(30, 30, 40);
pub const PANEL_FILL: Color32 = Color32::from_rgb(35, 35, 45);

// ============================================================================
// Outlines
// ============================================================================

pub const OUTLINE_DEFAULT: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 180);
pub const OUTLINE_THIN: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 160);

// ============================================================================
// Debug
// ============================================================================

pub const DEBUG_BOUNDARY: Color32 = Color32::from_rgba_premultiplied(39, 39, 39, 100);
pub const DEBUG_LABEL: Color32 = Color32::from_rgba_premultiplied(118, 118, 118, 150);

// ============================================================================
// Map-specific
// ============================================================================

pub const AXIS_LABEL_YELLOW: Color32 = Color32::from_rgb(255, 230, 50);
pub const SLICE_GREEN: Color32 = Color32::from_rgb(80, 200, 80);
pub const DIM_GRAY: Color32 = Color32::from_rgb(200, 200, 210);
pub const VISIBILITY_DARK_GREEN: Color32 = Color32::from_rgb(15, 70, 15);
pub const SLICE_GREEN_FILL: Color32 = Color32::from_rgba_premultiplied(9, 28, 9, 40);
pub const VISIBILITY_DARK_GREEN_FILL: Color32 = Color32::from_rgba_premultiplied(6, 27, 6, 100);
