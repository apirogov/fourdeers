//! Rendering utilities for stereo 3D visualization

pub mod tesseract;
pub mod tetra;
pub mod ui;

pub use tesseract::{
    ObjectRotationAngles, TesseractRenderConfig, TesseractRenderContext, TransformedVertex,
};
pub use tetra::{
    compass_vertex_label, render_tetrahedron, render_tetrahedron_with_projector, TetraLabelMode,
    TetraStyle,
};
pub use ui::{
    draw_arrow_head, draw_background, draw_center_divider, render_common_menu_half,
    render_dual_outlined_text, render_outlined_text, render_tap_zone_label,
};

use eframe::egui;

pub const STEREO_SCALE_FACTOR: f32 = 0.35;
pub const NEAR_PLANE_THRESHOLD: f32 = 0.1;
pub(super) const ARROW_STROKE_WIDTH: f32 = 2.0;
pub const TESSERACT_EDGE_STROKE_WIDTH: f32 = 2.5;
pub(super) const BASE_LABEL_FONT_SIZE: f32 = 11.0;
pub(super) const BASE_LABEL_OFFSET_Y: f32 = 18.0;
pub(super) const ARROW_END_DOT_RADIUS: f32 = 3.0;

#[must_use]
pub fn split_stereo_views(rect: egui::Rect) -> (egui::Rect, egui::Rect) {
    let left_rect = egui::Rect {
        min: rect.min,
        max: egui::pos2(rect.center().x, rect.max.y),
    };
    let right_rect = egui::Rect {
        min: egui::pos2(rect.center().x, rect.min.y),
        max: rect.max,
    };
    (left_rect, right_rect)
}

pub fn render_stereo_views(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    eye_separation: f32,
    projection_distance: f32,
    mode: ProjectionMode,
    render_fn: impl Fn(&egui::Painter, &StereoProjector, egui::Rect),
) {
    let (left_rect, right_rect) = split_stereo_views(rect);
    let scale = rect.height().min(rect.width() * 0.5) * STEREO_SCALE_FACTOR;

    let left_projector = StereoProjector::for_eye(
        left_rect.center(),
        scale,
        eye_separation,
        projection_distance,
        mode,
        -1.0,
    );
    let left_painter = ui.painter().with_clip_rect(left_rect);
    render_fn(&left_painter, &left_projector, left_rect);

    let right_projector = StereoProjector::for_eye(
        right_rect.center(),
        scale,
        eye_separation,
        projection_distance,
        mode,
        1.0,
    );
    let right_painter = ui.painter().with_clip_rect(right_rect);
    render_fn(&right_painter, &right_projector, right_rect);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    #[default]
    Perspective,
    Orthographic,
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

pub const DEFAULT_W_THICKNESS: f32 = 2.5;
pub const DEFAULT_W_COLOR_INTENSITY: f32 = 0.35;
pub const DEFAULT_PROJECTION_DISTANCE: f32 = 3.0;

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

#[derive(Debug, Clone, Copy)]
pub struct StereoSettings {
    pub eye_separation: f32,
    pub projection_distance: f32,
    pub projection_mode: ProjectionMode,
}

impl Default for StereoSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoSettings {
    #[must_use]
    pub fn new() -> Self {
        Self {
            eye_separation: 0.12,
            projection_distance: DEFAULT_PROJECTION_DISTANCE,
            projection_mode: ProjectionMode::Perspective,
        }
    }

    #[must_use]
    pub const fn with_projection_distance(mut self, distance: f32) -> Self {
        self.projection_distance = distance;
        self
    }
}

/// Compute the color for a normalized W coordinate.
///
/// Positive W fades toward blue, negative W fades toward red, with intensity
/// controlling how much the green channel is affected.
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

#[derive(Debug, Clone, Copy)]
pub struct StereoProjector {
    center: egui::Pos2,
    scale: f32,
    eye_offset: f32,
    projection_distance: f32,
    mode: ProjectionMode,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectedPoint {
    pub screen_pos: egui::Pos2,
    pub depth: f32,
}

impl StereoProjector {
    #[must_use]
    pub const fn new(
        center: egui::Pos2,
        scale: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> Self {
        Self {
            center,
            scale,
            eye_offset: 0.0,
            projection_distance,
            mode,
        }
    }

    #[must_use]
    pub fn for_eye(
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
        eye_sign: f32,
    ) -> Self {
        Self {
            center,
            scale,
            eye_offset: eye_sign * eye_separation * 0.5,
            projection_distance,
            mode,
        }
    }

    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    #[must_use]
    pub fn project_3d(&self, x: f32, y: f32, z: f32) -> Option<ProjectedPoint> {
        let x_shifted = x - self.eye_offset;

        let scale_factor = match self.mode {
            ProjectionMode::Perspective => {
                let z_offset = self.projection_distance + z;
                if z_offset <= NEAR_PLANE_THRESHOLD {
                    return None;
                }
                self.projection_distance / z_offset
            }
            ProjectionMode::Orthographic => 1.0,
        };

        let final_scale = self.scale * scale_factor;
        Some(ProjectedPoint {
            screen_pos: egui::Pos2::new(
                self.center.x + x_shifted * final_scale,
                self.center.y - y * final_scale,
            ),
            depth: z,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_approx_eq;

    fn cube_vertices(x: f32, y: f32, z: f32) -> Vec<(f32, f32, f32)> {
        let size = 0.5;
        let rx: f32 = 0.3;
        let ry: f32 = 0.2;
        let rz: f32 = 0.1;
        let cos_x = rx.cos();
        let sin_x = rx.sin();
        let cos_y = ry.cos();
        let sin_y = ry.sin();
        let cos_z = rz.cos();
        let sin_z = rz.sin();

        let corners = [
            (-1.0, -1.0, -1.0),
            (1.0, -1.0, -1.0),
            (1.0, 1.0, -1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (1.0, -1.0, 1.0),
            (1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
        ];

        corners
            .iter()
            .map(|(cx, cy, cz)| {
                let px = cx * size;
                let py = cy * size;
                let pz = cz * size;
                let y1 = py * cos_x - pz * sin_x;
                let z1 = py * sin_x + pz * cos_x;
                let px1 = px * cos_y + z1 * sin_y;
                let z2 = -px * sin_y + z1 * cos_y;
                let px2 = px1 * cos_z - y1 * sin_z;
                let py2 = px1 * sin_z + y1 * cos_z;
                (x + px2, y + py2, z + z2)
            })
            .collect()
    }

    fn make_eye_projector(
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
        eye_sign: f32,
    ) -> StereoProjector {
        StereoProjector::for_eye(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            eye_sign,
        )
    }

    fn project_cube_for_eyes(
        vertices: &[(f32, f32, f32)],
        center: egui::Pos2,
        scale: f32,
        eye_separation: f32,
        projection_distance: f32,
        mode: ProjectionMode,
    ) -> (Vec<Option<ProjectedPoint>>, Vec<Option<ProjectedPoint>>) {
        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            mode,
            1.0,
        );
        let left: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| left_proj.project_3d(*x, *y, *z))
            .collect();
        let right: Vec<_> = vertices
            .iter()
            .map(|(x, y, z)| right_proj.project_3d(*x, *y, *z))
            .collect();
        (left, right)
    }

    #[test]
    fn test_stereo_eyes_produce_different_x_coordinates() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(
            &vertices,
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            let l = l.expect("left should project");
            let r = r.expect("right should project");
            assert!(
                (l.screen_pos.x - r.screen_pos.x).abs() > 0.01,
                "Vertex {}: left.x ({:.4}) should differ from right.x ({:.4})",
                i,
                l.screen_pos.x,
                r.screen_pos.x
            );
        }
    }

    #[test]
    fn test_stereo_eyes_have_same_y_coordinates() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.1;
        let projection_distance = 5.0;

        let vertices = cube_vertices(0.0, 0.0, -2.0);
        let (left, right) = project_cube_for_eyes(
            &vertices,
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
        );

        for (l, r) in left.iter().zip(right.iter()) {
            let l = l.expect("left should project");
            let r = r.expect("right should project");
            assert_approx_eq(l.screen_pos.y, r.screen_pos.y, 1e-6);
        }
    }

    #[test]
    fn test_parallax_increases_with_depth() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let far_left = left_proj.project_3d(0.0, 0.0, -4.0).unwrap();
        let far_right = right_proj.project_3d(0.0, 0.0, -4.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = left_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_right = right_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_parallax = (near_left.screen_pos.x - near_right.screen_pos.x).abs();

        assert!(
            far_parallax > near_parallax,
            "Far parallax ({:.4}) should be greater than near parallax ({:.4})",
            far_parallax,
            near_parallax
        );
    }

    #[test]
    fn test_orthographic_parallax_constant_across_depth() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Orthographic,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Orthographic,
            1.0,
        );

        let far_left = left_proj.project_3d(0.0, 0.0, -10.0).unwrap();
        let far_right = right_proj.project_3d(0.0, 0.0, -10.0).unwrap();
        let far_parallax = (far_left.screen_pos.x - far_right.screen_pos.x).abs();

        let near_left = left_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_right = right_proj.project_3d(0.0, 0.0, -1.0).unwrap();
        let near_parallax = (near_left.screen_pos.x - near_right.screen_pos.x).abs();

        assert_approx_eq(far_parallax, near_parallax, 1e-6);
    }

    #[test]
    fn test_no_eye_has_no_parallax() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let mono = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        let eye = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let with_eye = eye.project_3d(0.5, 0.3, -2.0).unwrap();
        let no_eye = mono.project_3d(0.5, 0.3, -2.0).unwrap();

        assert!(
            (with_eye.screen_pos.x - no_eye.screen_pos.x).abs() > 0.01,
            "With-eye projection should differ from no-eye"
        );
    }

    #[test]
    fn test_behind_camera_returns_none() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let projection_distance = 5.0;

        let projector = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        assert!(projector.project_3d(0.0, 0.0, -5.1).is_none());
    }

    #[test]
    fn test_left_eye_sees_right_right_eye_sees_left() {
        let center = egui::Pos2::new(100.0, 100.0);
        let scale = 50.0;
        let eye_separation = 0.2;
        let projection_distance = 5.0;

        let mono = StereoProjector::new(
            center,
            scale,
            projection_distance,
            ProjectionMode::Perspective,
        );
        let left_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            -1.0,
        );
        let right_proj = make_eye_projector(
            center,
            scale,
            eye_separation,
            projection_distance,
            ProjectionMode::Perspective,
            1.0,
        );

        let left = left_proj.project_3d(0.0, 0.0, -2.0).unwrap();
        let right = right_proj.project_3d(0.0, 0.0, -2.0).unwrap();
        let mono_p = mono.project_3d(0.0, 0.0, -2.0).unwrap();

        assert!(
            left.screen_pos.x > mono_p.screen_pos.x,
            "Left eye ({:.4}) should be right of mono ({:.4}) — camera shifted left sees object shifted right",
            left.screen_pos.x,
            mono_p.screen_pos.x
        );
        assert!(
            right.screen_pos.x < mono_p.screen_pos.x,
            "Right eye ({:.4}) should be left of mono ({:.4}) — camera shifted right sees object shifted left",
            right.screen_pos.x,
            mono_p.screen_pos.x
        );
    }

    #[test]
    fn test_render_transform_matches_quaternion_pipeline() {
        use crate::camera::Camera;
        use nalgebra::Vector3;

        for (rot4d_x, rot4d_y, rot3d_x, rot3d_y) in [
            (0.0f32, 0.0f32, 0.0f32, 0.0f32),
            (50.0, 30.0, 0.0, 0.0),
            (0.0, 0.0, 20.0, -10.0),
            (50.0, 30.0, 20.0, -10.0),
            (-40.0, 70.0, 15.0, 25.0),
        ] {
            let mut camera = Camera::new();
            camera.rotate_4d(rot4d_x, rot4d_y);
            camera.rotate(rot3d_x, rot3d_y);

            let test_verts: Vec<nalgebra::Vector4<f32>> = vec![
                nalgebra::Vector4::new(1.0, 0.0, 0.0, 0.0),
                nalgebra::Vector4::new(0.0, 1.0, 0.0, 0.0),
                nalgebra::Vector4::new(0.0, 0.0, 1.0, 0.0),
                nalgebra::Vector4::new(0.0, 0.0, 0.0, 1.0),
                nalgebra::Vector4::new(1.0, 2.0, -3.0, 4.0),
                nalgebra::Vector4::new(-1.0, -2.0, 3.0, -4.0),
            ];
            let indices: Vec<u16> = vec![];

            let config = TesseractRenderConfig {
                rotation_angles: ObjectRotationAngles::default(),
                four_d: FourDSettings::default(),
                stereo: StereoSettings::default(),
            };

            let ctx = TesseractRenderContext::from_config(&test_verts, &indices, &camera, config);
            let transformed = ctx.transform_vertices();

            let qr_inv = camera.rotation_4d.inverse_q_right_only();
            let inv_q_left = camera.rotation_4d.q_left().inverse();

            for (i, v) in test_verts.iter().enumerate() {
                let p_4d = qr_inv.rotate_vector(*v - camera.position);

                let p3 = Vector3::new(p_4d.x, p_4d.y, p_4d.z);
                let expected_xyz = inv_q_left.transform_vector(&p3);
                let expected_w = p_4d.w;

                let t = &transformed[i];
                assert_approx_eq(t.x, expected_xyz.x, 1e-4);
                assert_approx_eq(t.y, expected_xyz.y, 1e-4);
                assert_approx_eq(t.z, expected_xyz.z, 1e-4);
                assert_approx_eq(t.w, expected_w, 1e-4);
            }
        }
    }

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
