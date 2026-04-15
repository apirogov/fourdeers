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
pub const DICHOPTIC_CHROMA_STRENGTH: f32 = 0.35;
pub const DICHOPTIC_LUMINANCE_STRENGTH: f32 = 0.08;
pub const W_SLICE_EXTENT_SIGMA: f32 = 3.0;

pub const W_COLOR_NEGATIVE: (f32, f32, f32) = (0.5, 0.0, 1.0);
pub const W_COLOR_MIDPOINT: (f32, f32, f32) = (1.0, 1.0, 1.0);
pub const W_COLOR_POSITIVE: (f32, f32, f32) = (1.0, 0.45, 0.0);
pub const W_COLOR_LUT_SIZE: usize = 1024;

#[must_use]
pub fn compute_vertex_alpha(w: f32, sigma: f32) -> u8 {
    let ratio = w / sigma;
    let alpha = 255.0 * (-0.5 * ratio * ratio).exp();
    alpha as u8
}

pub fn truncate_segment_to_slice(
    p0: nalgebra::Vector4<f32>,
    p1: nalgebra::Vector4<f32>,
    sigma: f32,
) -> Option<[nalgebra::Vector4<f32>; 2]> {
    let extent = W_SLICE_EXTENT_SIGMA * sigma;
    let in_a = p0[3] >= -extent && p0[3] <= extent;
    let in_b = p1[3] >= -extent && p1[3] <= extent;

    if in_a && in_b {
        return Some([p0, p1]);
    }

    if !in_a && !in_b {
        let w_min = p0[3].min(p1[3]);
        let w_max = p0[3].max(p1[3]);
        if w_max < -extent || w_min > extent {
            return None;
        }
    }

    let w_diff = p1[3] - p0[3];
    if w_diff.abs() < 1e-10 {
        return None;
    }

    let t = if !in_a {
        if p0[3] < -extent {
            (-extent - p0[3]) / w_diff
        } else {
            (extent - p0[3]) / w_diff
        }
    } else if p1[3] < -extent {
        (-extent - p0[3]) / w_diff
    } else {
        (extent - p0[3]) / w_diff
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

    let r = unified.r() as f32 / 255.0;
    let g = unified.g() as f32 / 255.0;
    let b = unified.b() as f32 / 255.0;

    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let rg = r - g;
    let by = b - (r + g) * 0.5;

    let chroma = (rg * rg + by * by).sqrt();
    let s = dichoptic_intensity.clamp(0.0, 1.0);

    let (new_y, new_rg, new_by) = if chroma > 1e-6 {
        let perp_rg = -by / chroma;
        let perp_by = rg / chroma;
        let chroma_delta = s * chroma * DICHOPTIC_CHROMA_STRENGTH;
        let lum_delta = s * DICHOPTIC_LUMINANCE_STRENGTH;
        (
            y + eye_sign * lum_delta,
            rg + eye_sign * chroma_delta * perp_rg,
            by + eye_sign * chroma_delta * perp_by,
        )
    } else {
        let lum_delta = s * DICHOPTIC_LUMINANCE_STRENGTH;
        (y + eye_sign * lum_delta, rg, by)
    };

    let nr = new_y + 0.644 * new_rg - 0.114 * new_by;
    let ng = new_y - 0.356 * new_rg - 0.114 * new_by;
    let nb = new_y + 0.144 * new_rg + 0.886 * new_by;

    egui::Color32::from_rgba_unmultiplied(
        (nr.clamp(0.0, 1.0) * 255.0) as u8,
        (ng.clamp(0.0, 1.0) * 255.0) as u8,
        (nb.clamp(0.0, 1.0) * 255.0) as u8,
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
        let p1 = Vector4::new(1.0, 0.0, 0.0, 5.0);
        let w_half = 1.25;
        let result = truncate_segment_to_slice(p0, p1, w_half);
        assert!(result.is_some());
        let truncated = result.unwrap();
        let extent = W_SLICE_EXTENT_SIGMA * w_half;
        assert!(
            truncated[0][3] >= -extent && truncated[0][3] <= extent,
            "truncated[0].w={} should be within ±{}",
            truncated[0][3],
            extent
        );
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
    fn test_sub_slice_edge_alpha_is_gaussian() {
        let w_half = 2.5;
        for p in [0.0, 0.5, 1.0] {
            let (_, sub_half) = eye_w_params(w_half, p, -1.0);
            let alpha = compute_vertex_alpha(sub_half, sub_half);
            let expected = compute_vertex_alpha(w_half, w_half);
            assert_eq!(
                alpha, expected,
                "edge alpha at p={} should match Gaussian at sigma",
                p
            );
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
            alpha > 128,
            "overlap should be above half alpha, got {}",
            alpha
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
    fn test_dichoptic_midpoint_near_white() {
        for intensity in [0.0, 0.5, 1.0] {
            for eye_sign in [-1.0, 1.0] {
                let c = w_to_color_dichoptic(0.0, 255, eye_sign, intensity);
                assert!(
                    (c.r() as i32 - c.g() as i32).abs() < 10,
                    "at W=0 R≈G: r={} g={} eye={} int={}",
                    c.r(),
                    c.g(),
                    eye_sign,
                    intensity
                );
                assert!(
                    (c.r() as i32 - c.b() as i32).abs() < 10,
                    "at W=0 R≈B: r={} b={} eye={} int={}",
                    c.r(),
                    c.b(),
                    eye_sign,
                    intensity
                );
            }
        }
    }

    #[test]
    fn test_dichoptic_eyes_differ_at_nonzero_w() {
        for nw in [-1.0, -0.5, 0.5, 1.0] {
            let left = w_to_color_dichoptic(nw, 255, -1.0, 1.0);
            let right = w_to_color_dichoptic(nw, 255, 1.0, 1.0);
            assert_ne!(left, right, "eyes should differ at nw={}", nw);
        }
    }

    #[test]
    fn test_dichoptic_eyes_average_near_unified() {
        for nw in [-1.0, -0.5, 0.5, 1.0] {
            let unified = w_to_color(nw, 255);
            let left = w_to_color_dichoptic(nw, 255, -1.0, 1.0);
            let right = w_to_color_dichoptic(nw, 255, 1.0, 1.0);
            let avg_r = (left.r() as f32 + right.r() as f32) / 2.0;
            let avg_g = (left.g() as f32 + right.g() as f32) / 2.0;
            let avg_b = (left.b() as f32 + right.b() as f32) / 2.0;
            assert!(
                (avg_r - unified.r() as f32).abs() <= 40.0,
                "avg R ({}) ≈ unified R ({}) at nw={}",
                avg_r,
                unified.r(),
                nw
            );
            assert!(
                (avg_g - unified.g() as f32).abs() <= 40.0,
                "avg G ({}) ≈ unified G ({}) at nw={}",
                avg_g,
                unified.g(),
                nw
            );
            assert!(
                (avg_b - unified.b() as f32).abs() <= 40.0,
                "avg B ({}) ≈ unified B ({}) at nw={}",
                avg_b,
                unified.b(),
                nw
            );
        }
    }

    #[test]
    fn test_dichoptic_divergence_increases_with_intensity() {
        let nw = -1.0;
        let c0 = w_to_color_dichoptic(nw, 255, -1.0, 0.0);
        let c_half = w_to_color_dichoptic(nw, 255, -1.0, 0.5);
        let c_full = w_to_color_dichoptic(nw, 255, -1.0, 1.0);
        let left_right_diff = |c: egui::Color32, eye: f32| -> f32 {
            let other = w_to_color_dichoptic(nw, 255, -eye, 1.0);
            (c.r() as f32 - other.r() as f32).abs()
                + (c.g() as f32 - other.g() as f32).abs()
                + (c.b() as f32 - other.b() as f32).abs()
        };
        let d0 = left_right_diff(c0, -1.0);
        let d_half = left_right_diff(c_half, -1.0);
        let d_full = left_right_diff(c_full, -1.0);
        assert!(
            d_half > d0,
            "half intensity divergence ({}) > zero ({})",
            d_half,
            d0
        );
        assert!(
            d_full > d_half,
            "full intensity divergence ({}) > half ({})",
            d_full,
            d_half
        );
    }

    #[test]
    fn test_dichoptic_luminance_imbalance_is_bounded() {
        for nw in [-1.0, 0.5, 1.0] {
            let left = w_to_color_dichoptic(nw, 255, -1.0, 1.0);
            let right = w_to_color_dichoptic(nw, 255, 1.0, 1.0);
            let lum_left =
                0.299 * left.r() as f32 + 0.587 * left.g() as f32 + 0.114 * left.b() as f32;
            let lum_right =
                0.299 * right.r() as f32 + 0.587 * right.g() as f32 + 0.114 * right.b() as f32;
            let diff = (lum_left - lum_right).abs();
            assert!(
                diff > 0.0 && diff < 60.0,
                "luminance diff should be nonzero but bounded: diff={} at nw={}",
                diff,
                nw
            );
        }
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

    #[test]
    fn test_gaussian_alpha_center_is_full() {
        assert_eq!(compute_vertex_alpha(0.0, 2.5), 255);
        assert_eq!(compute_vertex_alpha(0.0, 1.0), 255);
    }

    #[test]
    fn test_gaussian_alpha_sigma_is_60_percent() {
        let alpha = compute_vertex_alpha(2.5, 2.5);
        assert!(
            (alpha as f32 - 255.0 * (-0.5f32).exp()).abs() <= 1.5,
            "alpha at w=sigma should be ~155, got {}",
            alpha
        );
    }

    #[test]
    fn test_gaussian_alpha_near_center_higher_than_linear() {
        let sigma = 2.5;
        let gaussian = compute_vertex_alpha(sigma * 0.5, sigma);
        let linear_approx = 255 - (255 - 128) / 2;
        assert!(
            gaussian > linear_approx as u8,
            "Gaussian at 0.5σ ({}) should be higher than old linear ({})",
            gaussian,
            linear_approx
        );
    }

    #[test]
    fn test_gaussian_alpha_decreases_with_distance() {
        let sigma = 2.5;
        let a0 = compute_vertex_alpha(0.0, sigma);
        let a1 = compute_vertex_alpha(1.0, sigma);
        let a2 = compute_vertex_alpha(2.0, sigma);
        let a3 = compute_vertex_alpha(3.0, sigma);
        assert!(a0 > a1, "0 > 1: {} > {}", a0, a1);
        assert!(a1 > a2, "1 > 2: {} > {}", a1, a2);
        assert!(a2 > a3, "2 > 3: {} > {}", a2, a3);
    }

    #[test]
    fn test_truncate_at_three_sigma() {
        let sigma = 1.0;
        let extent = W_SLICE_EXTENT_SIGMA * sigma;
        let inside = Vector4::new(0.0, 0.0, 0.0, extent - 0.01);
        let outside = Vector4::new(0.0, 0.0, 0.0, extent + 0.01);
        assert!(
            truncate_segment_to_slice(inside, outside, sigma).is_some(),
            "edge crossing boundary should be truncated, not dropped"
        );
        assert!(
            truncate_segment_to_slice(outside, outside, sigma).is_none(),
            "edge entirely beyond 3σ should be filtered"
        );
    }

    #[test]
    fn test_gaussian_alpha_tail_visible_beyond_sigma() {
        let sigma = 2.5;
        let alpha_2s = compute_vertex_alpha(2.0 * sigma, sigma);
        assert!(
            alpha_2s > 0,
            "alpha at 2σ should still be visible, got {}",
            alpha_2s
        );
    }
}
