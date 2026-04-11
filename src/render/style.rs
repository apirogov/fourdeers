use eframe::egui;

pub const DEFAULT_W_THICKNESS: f32 = 2.5;
pub const DEFAULT_W_COLOR_INTENSITY: f32 = 0.35;
pub const DEFAULT_PROJECTION_DISTANCE: f32 = 3.0;
pub const W_THICKNESS_DRAG_SENSITIVITY: f32 = 0.02;
pub const W_THICKNESS_MIN: f32 = 0.1;
pub const W_THICKNESS_MAX: f32 = 5.0;

pub fn adjust_w_thickness(w_thickness: f32, delta_x: f32) -> f32 {
    (w_thickness + delta_x * W_THICKNESS_DRAG_SENSITIVITY).clamp(W_THICKNESS_MIN, W_THICKNESS_MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompassFrameMode {
    #[default]
    World,
    Camera,
}

impl CompassFrameMode {
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::World => Self::Camera,
            Self::Camera => Self::World,
        }
    }

    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::World => "Frame: World",
            Self::Camera => "Frame: Camera",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FourDSettings {
    pub w_thickness: f32,
    pub w_color_intensity: f32,
}

impl Default for FourDSettings {
    fn default() -> Self {
        Self {
            w_thickness: DEFAULT_W_THICKNESS,
            w_color_intensity: DEFAULT_W_COLOR_INTENSITY,
        }
    }
}

#[must_use]
pub fn w_to_color(normalized_w: f32, alpha: u8, intensity: f32) -> egui::Color32 {
    if normalized_w >= 0.0 {
        let t = normalized_w;
        let r = crate::colors::to_u8(255.0 * (1.0 - t));
        let g = crate::colors::to_u8(255.0 * (1.0 - t * intensity));
        let b = crate::colors::to_u8(255.0 * (1.0 - t) + 255.0 * t);
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    } else {
        let t = -normalized_w;
        let r = 255u8;
        let g = crate::colors::to_u8(255.0 * (1.0 - t * intensity));
        let b = crate::colors::to_u8(255.0 * (1.0 - t));
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_w_to_color_zero_w() {
        let c = w_to_color(0.0, 255, 0.35);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 255);
        assert_eq!(c.b(), 255);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_positive_w_full() {
        let c = w_to_color(1.0, 255, 0.35);
        assert_eq!(c.r(), 0);
        assert_eq!(c.b(), 255);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_negative_w_full() {
        let c = w_to_color(-1.0, 255, 0.35);
        assert_eq!(c.r(), 255);
        assert_eq!(c.b(), 0);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_w_to_color_alpha_passthrough() {
        let c = w_to_color(0.0, 128, 0.35);
        assert_eq!(c.a(), 128);
    }

    #[test]
    fn test_w_to_color_positive_w_reduces_red() {
        let c_half = w_to_color(0.5, 255, 0.35);
        assert!(c_half.r() < 255);
        assert!(c_half.r() > 0);
    }

    #[test]
    fn test_w_to_color_negative_w_reduces_blue() {
        let c_half = w_to_color(-0.5, 255, 0.35);
        assert!(c_half.b() < 255);
        assert!(c_half.b() > 0);
    }

    #[test]
    fn test_w_to_color_intensity_affects_green() {
        let c_low = w_to_color(0.5, 255, 0.1);
        let c_high = w_to_color(0.5, 255, 0.9);
        assert!(c_low.g() > c_high.g());
    }

    #[test]
    fn test_compass_frame_mode_display_label() {
        assert_eq!(CompassFrameMode::World.display_label(), "Frame: World");
        assert_eq!(CompassFrameMode::Camera.display_label(), "Frame: Camera");
    }

    #[test]
    fn test_compass_frame_mode_other() {
        assert_eq!(CompassFrameMode::World.other(), CompassFrameMode::Camera);
        assert_eq!(CompassFrameMode::Camera.other(), CompassFrameMode::World);
    }
}
