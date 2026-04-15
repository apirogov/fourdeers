use eframe::egui;

pub const DEFAULT_W_THICKNESS: f32 = 2.5;
pub const DEFAULT_PROJECTION_DISTANCE: f32 = 3.0;
pub const DEFAULT_EYE_SEPARATION: f32 = 0.12;
pub const W_THICKNESS_DRAG_SENSITIVITY: f32 = 0.02;
pub const W_THICKNESS_MIN: f32 = 0.1;
pub const W_THICKNESS_MAX: f32 = 5.0;
pub const W_EYE_OFFSET_DRAG_SENSITIVITY: f32 = 0.01;
pub const W_EYE_OFFSET_MAX: f32 = 1.0;
pub const W_EYE_SPREAD: f32 = 0.45;
pub const DICHOPTIC_DRAG_SENSITIVITY: f32 = 0.01;
pub const DICHOPTIC_INTENSITY_MAX: f32 = 1.0;
pub const DICHOPTIC_BLUE: (f32, f32, f32) = (0.1, 0.3, 1.0);
pub const DICHOPTIC_RED: (f32, f32, f32) = (1.0, 0.15, 0.0);
pub const DICHOPTIC_YELLOW: (f32, f32, f32) = (1.0, 0.85, 0.0);
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

pub fn adjust_w_thickness(w_thickness: f32, delta_x: f32, dt_scale: f32) -> f32 {
    (w_thickness + delta_x * W_THICKNESS_DRAG_SENSITIVITY * dt_scale)
        .clamp(W_THICKNESS_MIN, W_THICKNESS_MAX)
}

pub fn adjust_w_eye_offset(w_eye_offset: f32, delta_y: f32, dt_scale: f32) -> f32 {
    (w_eye_offset - delta_y * W_EYE_OFFSET_DRAG_SENSITIVITY * dt_scale).clamp(0.0, W_EYE_OFFSET_MAX)
}

#[must_use]
pub fn eye_w_params(w_half: f32, w_eye_offset: f32, eye_sign: f32) -> (f32, f32) {
    let shift = w_eye_offset * w_half * W_EYE_SPREAD * eye_sign;
    let sub_half = w_half * (1.0 - w_eye_offset * W_EYE_SPREAD);
    (shift, sub_half)
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
    pub w_eye_offset: f32,
    pub dichoptic_intensity: f32,
}

impl Default for FourDSettings {
    fn default() -> Self {
        Self {
            w_thickness: DEFAULT_W_THICKNESS,
            w_eye_offset: 0.0,
            dichoptic_intensity: 0.0,
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

pub fn adjust_dichoptic_intensity(intensity: f32, delta_y: f32, dt_scale: f32) -> f32 {
    (intensity - delta_y * DICHOPTIC_DRAG_SENSITIVITY * dt_scale)
        .clamp(0.0, DICHOPTIC_INTENSITY_MAX)
}

#[must_use]
pub fn w_to_color_dichoptic(
    normalized_w: f32,
    alpha: u8,
    eye_sign: f32,
    dichoptic_intensity: f32,
) -> egui::Color32 {
    let unified = w_to_color(normalized_w, alpha);
    if dichoptic_intensity <= 0.0 {
        return unified;
    }

    let t = normalized_w.abs();
    let (dr, dg, db) = if normalized_w < 0.0 {
        if eye_sign < 0.0 {
            DICHOPTIC_BLUE
        } else {
            DICHOPTIC_RED
        }
    } else if eye_sign < 0.0 {
        DICHOPTIC_RED
    } else {
        DICHOPTIC_YELLOW
    };

    let target_r = lerp(W_COLOR_MIDPOINT.0, dr, t);
    let target_g = lerp(W_COLOR_MIDPOINT.1, dg, t);
    let target_b = lerp(W_COLOR_MIDPOINT.2, db, t);

    let s = dichoptic_intensity.clamp(0.0, 1.0);
    let fr = lerp(unified.r() as f32 / 255.0, target_r, s);
    let fg = lerp(unified.g() as f32 / 255.0, target_g, s);
    let fb = lerp(unified.b() as f32 / 255.0, target_b, s);

    egui::Color32::from_rgba_unmultiplied(
        (fr.clamp(0.0, 1.0) * 255.0) as u8,
        (fg.clamp(0.0, 1.0) * 255.0) as u8,
        (fb.clamp(0.0, 1.0) * 255.0) as u8,
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

    #[test]
    fn test_sub_slice_center_alpha_constant_across_panning() {
        let w_half = 2.5;
        for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let (_, sub_half) = eye_w_params(w_half, p, -1.0);
            let alpha = compute_vertex_alpha(0.0, sub_half);
            assert_eq!(alpha, 255, "alpha at sub-slice center with p={}", p);
        }
    }

    #[test]
    fn test_sub_slice_edge_alpha_is_min() {
        let w_half = 2.5;
        for p in [0.0, 0.5, 1.0] {
            let (_, sub_half) = eye_w_params(w_half, p, -1.0);
            let alpha = compute_vertex_alpha(sub_half, sub_half);
            assert_eq!(alpha, MIN_VERTEX_ALPHA, "edge alpha at p={}", p);
        }
    }

    #[test]
    fn test_zero_panning_matches_current_behavior() {
        let w_half = 2.5;
        let (shift, sub_half) = eye_w_params(w_half, 0.0, -1.0);
        assert_eq!(shift, 0.0);
        assert_eq!(sub_half, w_half);
        for w in [0.0, 0.5, 1.0, 2.0, 2.5] {
            let shifted = w - shift;
            let alpha = compute_vertex_alpha(shifted, sub_half);
            let expected = compute_vertex_alpha(w, w_half);
            assert_eq!(alpha, expected, "at w={}", w);
        }
    }

    #[test]
    fn test_max_panning_left_eye_fades_positive_w() {
        let w_half = 2.5;
        let (shift, sub_half) = eye_w_params(w_half, 1.0, -1.0);
        let neg_shifted = -1.0 - shift;
        let pos_shifted = 1.0 - shift;
        let alpha_neg = compute_vertex_alpha(neg_shifted, sub_half);
        let alpha_pos = compute_vertex_alpha(pos_shifted, sub_half);
        assert!(
            alpha_neg > alpha_pos,
            "neg_alpha ({}) should > pos_alpha ({})",
            alpha_neg,
            alpha_pos
        );
    }

    #[test]
    fn test_panning_overlap_region_soft_fade() {
        let w_half = 2.5;
        let (shift, sub_half) = eye_w_params(w_half, 1.0, -1.0);
        let shifted_w = 0.0 - shift;
        let alpha = compute_vertex_alpha(shifted_w, sub_half);
        assert!(
            alpha > MIN_VERTEX_ALPHA,
            "overlap should be above min alpha"
        );
        assert!(alpha < 255, "overlap should be below max alpha");
    }

    #[test]
    fn test_adjust_w_thickness_scales_with_dt() {
        let base = adjust_w_thickness(2.5, 10.0, 1.0);
        let doubled = adjust_w_thickness(2.5, 10.0, 2.0);
        let half = adjust_w_thickness(2.5, 10.0, 0.5);
        assert!(
            doubled > base,
            "doubled dt_scale should produce larger change"
        );
        assert!(half < base, "half dt_scale should produce smaller change");
        assert!((base - 2.5).abs() > 0.0, "base should have some change");
        assert!(
            ((doubled - 2.5) - 2.0 * (base - 2.5)).abs() < 1e-6,
            "doubled should be exactly 2x base change"
        );
    }

    #[test]
    fn test_adjust_w_eye_offset_scales_with_dt() {
        let base = adjust_w_eye_offset(0.0, -10.0, 1.0);
        let doubled = adjust_w_eye_offset(0.0, -10.0, 2.0);
        let half = adjust_w_eye_offset(0.0, -10.0, 0.5);
        assert!(
            doubled > base,
            "doubled dt_scale should produce larger offset"
        );
        assert!(half < base, "half dt_scale should produce smaller offset");
        assert!(
            ((doubled - base) - (base - 0.0)).abs() < 1e-6,
            "doubled change should be exactly 2x base change"
        );
    }

    #[test]
    fn test_dichoptic_zero_intensity_matches_unified() {
        for nw in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let unified = w_to_color(nw, 255);
            let left = w_to_color_dichoptic(nw, 255, -1.0, 0.0);
            let right = w_to_color_dichoptic(nw, 255, 1.0, 0.0);
            assert_eq!(unified, left, "left eye at nw={}", nw);
            assert_eq!(unified, right, "right eye at nw={}", nw);
        }
    }

    #[test]
    fn test_dichoptic_midpoint_always_white() {
        for intensity in [0.0, 0.5, 1.0] {
            for eye_sign in [-1.0, 1.0] {
                let c = w_to_color_dichoptic(0.0, 255, eye_sign, intensity);
                assert!(c.r() > 240, "r at eye={} intensity={}", eye_sign, intensity);
                assert!(c.g() > 240, "g at eye={} intensity={}", eye_sign, intensity);
                assert!(c.b() > 240, "b at eye={} intensity={}", eye_sign, intensity);
            }
        }
    }

    #[test]
    fn test_dichoptic_full_negative_w_left_eye_is_blue() {
        let c = w_to_color_dichoptic(-1.0, 255, -1.0, 1.0);
        assert!(
            c.b() > c.r() && c.b() > c.g(),
            "left eye negative W should be blue-dominant, got r={} g={} b={}",
            c.r(),
            c.g(),
            c.b()
        );
    }

    #[test]
    fn test_dichoptic_full_negative_w_right_eye_is_red() {
        let c = w_to_color_dichoptic(-1.0, 255, 1.0, 1.0);
        assert!(
            c.r() > c.g() && c.r() > c.b(),
            "right eye negative W should be red-dominant, got r={} g={} b={}",
            c.r(),
            c.g(),
            c.b()
        );
    }

    #[test]
    fn test_dichoptic_full_positive_w_left_eye_is_red() {
        let c = w_to_color_dichoptic(1.0, 255, -1.0, 1.0);
        assert!(
            c.r() > c.g() && c.r() > c.b(),
            "left eye positive W should be red-dominant, got r={} g={} b={}",
            c.r(),
            c.g(),
            c.b()
        );
    }

    #[test]
    fn test_dichoptic_full_positive_w_right_eye_is_yellow() {
        let c = w_to_color_dichoptic(1.0, 255, 1.0, 1.0);
        assert!(
            c.r() > 200 && c.g() > 150,
            "right eye positive W should be yellow (high R+G), got r={} g={} b={}",
            c.r(),
            c.g(),
            c.b()
        );
    }

    #[test]
    fn test_dichoptic_half_intensity_is_midpoint() {
        let unified = w_to_color(-1.0, 255);
        let full = w_to_color_dichoptic(-1.0, 255, -1.0, 1.0);
        let half = w_to_color_dichoptic(-1.0, 255, -1.0, 0.5);
        let expected_r = (unified.r() as f32 + full.r() as f32) / 2.0;
        assert!(
            (half.r() as f32 - expected_r).abs() <= 1.5,
            "half intensity R should be midpoint: got {} expected ~{}",
            half.r(),
            expected_r
        );
    }

    #[test]
    fn test_adjust_dichoptic_intensity_scales_with_dt() {
        let base = adjust_dichoptic_intensity(0.5, -10.0, 1.0);
        let doubled = adjust_dichoptic_intensity(0.5, -10.0, 2.0);
        assert!(
            doubled > base,
            "doubled dt_scale should produce larger change"
        );
    }
}
