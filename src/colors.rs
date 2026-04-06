//! Color constants for the application

use egui::Color32;

// ============================================================================
// UI / Labels
// ============================================================================

#[inline]
pub fn label_default() -> Color32 {
    Color32::from_rgb(255, 180, 80)
}

#[inline]
pub fn label_inactive() -> Color32 {
    Color32::from_rgb(180, 180, 180)
}

// ============================================================================
// Text
// ============================================================================

#[inline]
pub fn text_highlight() -> Color32 {
    Color32::from_rgba_unmultiplied(230, 230, 230, 255)
}

#[inline]
pub fn text_dim() -> Color32 {
    Color32::from_rgba_unmultiplied(200, 200, 200, 150)
}

// ============================================================================
// Graphics (Arrows / 3D)
// ============================================================================

#[inline]
pub fn arrow_primary() -> Color32 {
    Color32::from_rgb(255, 150, 50)
}

#[inline]
pub fn arrow_glow() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 150, 50, 180)
}

#[inline]
pub fn arrow_tip() -> Color32 {
    Color32::from_rgb(255, 200, 100)
}

#[inline]
pub fn arrow_forward() -> Color32 {
    Color32::from_rgb(100, 220, 255)
}

// ============================================================================
// 4D Objects
// ============================================================================

#[inline]
pub fn object_tint_positive() -> Color32 {
    Color32::from_rgba_unmultiplied(150, 220, 150, 180)
}

#[inline]
pub fn object_tint_negative() -> Color32 {
    Color32::from_rgba_unmultiplied(150, 220, 150, 200)
}

// ============================================================================
// Backgrounds
// ============================================================================

#[inline]
pub fn viewport_bg() -> Color32 {
    Color32::from_rgb(30, 30, 40)
}

#[inline]
pub fn panel_fill() -> Color32 {
    Color32::from_rgb(35, 35, 45)
}

// ============================================================================
// Outlines
// ============================================================================

#[inline]
pub fn outline_default() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 180)
}

#[inline]
pub fn outline_thin() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 160)
}

// ============================================================================
// Debug
// ============================================================================

#[inline]
pub fn debug_boundary() -> Color32 {
    Color32::from_rgba_unmultiplied(100, 100, 100, 100)
}

#[inline]
pub fn debug_label() -> Color32 {
    Color32::from_rgba_unmultiplied(200, 200, 200, 150)
}
