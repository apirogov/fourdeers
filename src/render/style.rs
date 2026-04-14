use eframe::egui;

pub const DEFAULT_W_THICKNESS: f32 = 2.5;
pub const DEFAULT_PROJECTION_DISTANCE: f32 = 3.0;
pub const DEFAULT_EYE_SEPARATION: f32 = 0.12;
pub const W_THICKNESS_DRAG_SENSITIVITY: f32 = 0.02;
pub const W_THICKNESS_MIN: f32 = 0.1;
pub const W_THICKNESS_MAX: f32 = 5.0;
pub const MIN_VERTEX_ALPHA: u8 = 128;

pub const W_COLOR_NEGATIVE: (f32, f32, f32) = (0.6, 0.2, 0.8);
pub const W_COLOR_MIDPOINT: (f32, f32, f32) = (1.0, 1.0, 1.0);
pub const W_COLOR_POSITIVE: (f32, f32, f32) = (1.0, 0.7, 0.0);
pub const W_COLOR_LUT_SIZE: usize = 1024;

#[must_use]
pub fn compute_vertex_alpha(w: f32, w_half: f32) -> u8 {
    let normalized = (w / w_half).clamp(-1.0, 1.0);
    let t = normalized.abs();
    let alpha = 255.0 * (1.0 - t) + MIN_VERTEX_ALPHA as f32 * t;
    alpha as u8
}

pub fn truncate_segment_to_slice(
    p0: nalgebra::Vector4<f32>,
    p1: nalgebra::Vector4<f32>,
    w_half: f32,
) -> Option<[nalgebra::Vector4<f32>; 2]> {
    let in_a = p0[3] >= -w_half && p0[3] <= w_half;
    let in_b = p1[3] >= -w_half && p1[3] <= w_half;

    if in_a && in_b {
        return Some([p0, p1]);
    }

    if !in_a && !in_b {
        let w_min = p0[3].min(p1[3]);
        let w_max = p0[3].max(p1[3]);
        if w_max < -w_half || w_min > w_half {
            return None;
        }
    }

    let w_diff = p1[3] - p0[3];
    if w_diff.abs() < 1e-10 {
        return None;
    }

    let t = if !in_a {
        if p0[3] < -w_half {
            (-w_half - p0[3]) / w_diff
        } else {
            (w_half - p0[3]) / w_diff
        }
    } else if p1[3] < -w_half {
        (-w_half - p0[3]) / w_diff
    } else {
        (w_half - p0[3]) / w_diff
    };
    let t = t.clamp(0.0, 1.0);

    let lerped = p0 + (p1 - p0) * t;
    if !in_a {
        Some([lerped, p1])
    } else {
        Some([p0, lerped])
    }
}

const fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

const fn compute_w_color_lut() -> [u32; W_COLOR_LUT_SIZE] {
    let mut arr = [0u32; W_COLOR_LUT_SIZE];
    let (pn, pm, pp) = (W_COLOR_NEGATIVE, W_COLOR_MIDPOINT, W_COLOR_POSITIVE);
    let mut i = 0;
    while i < W_COLOR_LUT_SIZE {
        let t = i as f32 / (W_COLOR_LUT_SIZE - 1) as f32;
        let (r, g, b) = if t < 0.5 {
            let t2 = t * 2.0;
            (
                lerp(pn.0, pm.0, t2),
                lerp(pn.1, pm.1, t2),
                lerp(pn.2, pm.2, t2),
            )
        } else {
            let t2 = (t - 0.5) * 2.0;
            (
                lerp(pm.0, pp.0, t2),
                lerp(pm.1, pp.1, t2),
                lerp(pm.2, pp.2, t2),
            )
        };
        let ri = (r.clamp(0.0, 1.0) * 255.0) as u8;
        let gi = (g.clamp(0.0, 1.0) * 255.0) as u8;
        let bi = (b.clamp(0.0, 1.0) * 255.0) as u8;
        arr[i] = ((ri as u32) << 16) | ((gi as u32) << 8) | (bi as u32);
        i += 1;
    }
    arr
}

const W_COLOR_LUT: [u32; W_COLOR_LUT_SIZE] = compute_w_color_lut();

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
}

impl Default for FourDSettings {
    fn default() -> Self {
        Self {
            w_thickness: DEFAULT_W_THICKNESS,
        }
    }
}

#[must_use]
pub fn w_to_color(normalized_w: f32, alpha: u8) -> egui::Color32 {
    let t = ((normalized_w + 1.0) / 2.0).clamp(0.0, 1.0);
    let idx = (t * (W_COLOR_LUT_SIZE - 1) as f32) as usize;
    let packed = W_COLOR_LUT[idx];
    egui::Color32::from_rgba_unmultiplied(
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
        alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector4;

    #[test]
    fn test_w_to_color_zero_w() {
        let c = w_to_color(0.0, 255);
        assert_eq!(c.a(), 255);
        assert!(c.r() > 200);
        assert!(c.g() > 200);
        assert!(c.b() > 200);
    }

    #[test]
    fn test_w_to_color_positive_w_full() {
        let c = w_to_color(1.0, 255);
        assert_eq!(c.a(), 255);
        assert!(c.r() > c.b(), "positive w should have more red than blue");
    }

    #[test]
    fn test_w_to_color_negative_w_full() {
        let c = w_to_color(-1.0, 255);
        assert_eq!(c.a(), 255);
        assert!(c.b() > c.r(), "negative w should have more blue than red");
    }

    #[test]
    fn test_w_to_color_alpha_passthrough() {
        let c = w_to_color(0.0, 128);
        assert_eq!(c.a(), 128);
    }

    #[test]
    fn test_w_to_color_mid_positive_w() {
        let c_half = w_to_color(0.5, 255);
        assert!(c_half.r() > c_half.b(), "mid positive should be reddish");
    }

    #[test]
    fn test_w_to_color_mid_negative_w() {
        let c_half = w_to_color(-0.5, 255);
        assert!(c_half.b() > c_half.r(), "mid negative should be bluish");
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

    #[test]
    fn test_truncate_both_inside() {
        let p0 = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 0.5);
        let w_half = 1.25;
        let result = truncate_segment_to_slice(p0, p1, w_half);
        assert!(result.is_some());
    }

    #[test]
    fn test_truncate_both_outside_spans_through() {
        let p0 = Vector4::new(0.0, 0.0, 0.0, -3.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 3.0);
        let w_half = 1.25;
        let result = truncate_segment_to_slice(p0, p1, w_half);
        assert!(
            result.is_some(),
            "edge spanning through slice should not be filtered"
        );
    }

    #[test]
    fn test_truncate_both_outside_no_overlap() {
        let p0 = Vector4::new(0.0, 0.0, 0.0, 5.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 6.0);
        let w_half = 1.25;
        let result = truncate_segment_to_slice(p0, p1, w_half);
        assert!(
            result.is_none(),
            "edge entirely outside slice should be filtered"
        );
    }

    #[test]
    fn test_truncate_one_inside_one_outside() {
        let p0 = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 3.0);
        let w_half = 1.25;
        let result = truncate_segment_to_slice(p0, p1, w_half);
        assert!(result.is_some());
        let truncated = result.unwrap();
        assert!(truncated[0][3] >= -w_half && truncated[0][3] <= w_half);
    }
}
