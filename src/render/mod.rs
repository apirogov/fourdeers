//! Rendering utilities for stereo 3D visualization

pub mod batch;
pub mod projection;
pub mod style;
pub mod tesseract;
pub mod tetra;
pub mod ui;

pub use projection::{
    create_stereo_projectors, split_stereo_views, ProjectedPoint, ProjectionMode, StereoProjector,
    StereoSettings, StereoViewPair,
};
pub use style::{w_to_color, CompassFrameMode, FourDSettings};
pub use tesseract::{TesseractRenderConfig, TesseractRenderContext, TransformedVertex};
pub use tetra::{
    compass_vertex_label, render_tetrahedron, render_tetrahedron_with_projector, TetraLabelMode,
    TetraStyle,
};
pub use ui::{
    draw_arrow_head, draw_background, draw_center_divider, render_common_menu_half,
    render_dual_outlined_text, render_outlined_text, render_tap_zone_label,
};

pub(crate) use projection::NEAR_PLANE_THRESHOLD;
pub use style::{DEFAULT_PROJECTION_DISTANCE, DEFAULT_W_COLOR_INTENSITY, DEFAULT_W_THICKNESS};

pub const STEREO_SCALE_FACTOR: f32 = 0.35;
pub const TESSERACT_EDGE_STROKE_WIDTH: f32 = 2.5;
pub(super) const ARROW_STROKE_WIDTH: f32 = 2.0;
pub(super) const BASE_LABEL_FONT_SIZE: f32 = 11.0;
pub(super) const BASE_LABEL_OFFSET_Y: f32 = 18.0;
pub(super) const ARROW_END_DOT_RADIUS: f32 = 3.0;

pub fn render_stereo_views(
    ui: &mut eframe::egui::Ui,
    rect: eframe::egui::Rect,
    eye_separation: f32,
    projection_distance: f32,
    mode: ProjectionMode,
    render_fn: impl Fn(&eframe::egui::Painter, &StereoProjector, eframe::egui::Rect),
) {
    projection::render_stereo_views(
        ui,
        rect,
        eye_separation,
        projection_distance,
        mode,
        render_fn,
    )
}

#[cfg(test)]
mod tests {
    use crate::camera::{Camera, CameraProjection};
    use crate::render::{
        FourDSettings, StereoSettings, TesseractRenderConfig, TesseractRenderContext,
    };
    use crate::test_utils::assert_approx_eq;
    use nalgebra::Vector3;

    #[test]
    fn test_render_transform_matches_quaternion_pipeline() {
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
                four_d: FourDSettings::default(),
                stereo: StereoSettings::default(),
            };

            let projection = CameraProjection::new(&camera);
            let ctx = TesseractRenderContext::from_config(
                &test_verts,
                &indices,
                &camera,
                projection,
                config,
            );
            let transformed = ctx.transform_vertices();

            let qr_inv = camera.rotation_4d().inverse_q_right_only();
            let inv_q_left = camera.rotation_4d().q_left().inverse();

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
}
