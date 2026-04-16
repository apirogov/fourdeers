use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{Camera, CameraProjection};
use crate::colors::LABEL_INACTIVE;
use crate::input::{CameraControls, DragState, DragView, PointerAnalysis, TetraId, Zone, ZoneMode};
use crate::render::{
    create_stereo_projectors, draw_background, draw_center_divider, eye_w_params,
    render_tap_zone_label, split_stereo_views, FourDSettings, StereoSettings,
    TesseractRenderConfig, TesseractRenderContext,
};
use crate::toy::ViewAction;

pub struct SceneRenderParams<'a> {
    pub camera: &'a Camera,
    pub vertices: &'a [Vector4<f32>],
    pub indices: &'a [u16],
    pub four_d: FourDSettings,
    pub stereo: StereoSettings,
    pub show_debug: bool,
}

pub struct SceneView {
    pub info_level: u8,
    pub controls: CameraControls,
    pub zone_mode: ZoneMode,
    pub visualization_rect: Option<egui::Rect>,
    pub drag_state: DragState,
    pub tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>,
}

impl SceneView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            info_level: 0,
            controls: CameraControls::new(false),
            zone_mode: ZoneMode::NineZones,
            visualization_rect: None,
            drag_state: DragState::new(),
            tetrahedron_rotations: HashMap::new(),
        }
    }

    pub(crate) fn render(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        params: SceneRenderParams<'_>,
    ) {
        draw_background(ui, rect);

        self.visualization_rect = Some(rect);

        draw_center_divider(ui, rect);

        let config = TesseractRenderConfig {
            four_d: params.four_d,
            stereo: params.stereo,
        };
        let projection = CameraProjection::new(params.camera);
        let ctx = TesseractRenderContext::from_config(
            params.vertices,
            params.indices,
            params.camera,
            projection,
            config,
        );

        let transformed = ctx.transform_vertices();

        let views = create_stereo_projectors(
            rect,
            params.stereo.eye_separation,
            params.stereo.projection_distance,
            params.stereo.projection_mode,
        );

        let w_half = params.four_d.w_thickness * 0.5;
        let w_eye_offset = params.four_d.w_eye_offset;
        let dichoptic_intensity = params.four_d.dichoptic_intensity;

        for (eye_idx, (projector, view_rect)) in [
            (&views.left_projector, views.left_rect),
            (&views.right_projector, views.right_rect),
        ]
        .iter()
        .enumerate()
        {
            let eye_sign = if eye_idx == 0 { -1.0 } else { 1.0 };
            let (w_shift, sub_w_half) = eye_w_params(w_half, w_eye_offset, eye_sign);
            let painter = ui.painter().with_clip_rect(*view_rect);
            ctx.render_edges(
                &painter,
                projector,
                &transformed,
                painter.clip_rect(),
                w_shift,
                sub_w_half,
                eye_sign,
                dichoptic_intensity,
            );
        }

        if params.show_debug {
            let right_rect = split_stereo_views(rect).1;
            let right_painter = ui.painter().with_clip_rect(right_rect);
            ctx.render_zone_labels(&right_painter, right_rect);
        }

        if self.info_level == 2 {
            let right_rect = split_stereo_views(rect).1;
            let right_painter = ui.painter().with_clip_rect(right_rect);
            ctx.render_tetrahedron_gadget(&right_painter, right_rect, &self.tetrahedron_rotations);
        }
    }

    pub fn render_overlays(
        &self,
        left_painter: &egui::Painter,
        left_rect: egui::Rect,
        right_painter: &egui::Painter,
        right_rect: egui::Rect,
        w_thickness: f32,
        camera: &Camera,
    ) {
        let info_label = match self.info_level {
            0 => "UI:Min",
            1 => "UI:Mid",
            _ => "UI:Max",
        };
        render_tap_zone_label(left_painter, left_rect, Zone::North, info_label, None);

        if self.info_level >= 1 {
            let gray = Some(LABEL_INACTIVE);

            let rot_label = if self.controls.rotation_3d {
                "Rot:3D"
            } else {
                "Rot:4D"
            };
            render_tap_zone_label(right_painter, right_rect, Zone::NorthEast, rot_label, gray);

            let w_label = format!("WØ: {:.1}", w_thickness);
            render_tap_zone_label(right_painter, right_rect, Zone::SouthEast, &w_label, gray);

            let pos = camera.position;
            let pos_label = format!(
                "X: {:.1} Y: {:.1} Z: {:.1} W: {:.1}",
                pos.x, pos.y, pos.z, pos.w
            );
            render_tap_zone_label(right_painter, right_rect, Zone::South, &pos_label, gray);
        }
    }

    pub fn handle_pointer(
        &mut self,
        analysis: &PointerAnalysis,
        camera: &mut Camera,
    ) -> ViewAction {
        if let Some(zone) = analysis.zone {
            if !analysis.is_hold {
                if analysis.is_left_view && zone == Zone::North {
                    self.info_level = (self.info_level + 1) % 3;
                    return ViewAction::None;
                }

                if !analysis.is_left_view && zone == Zone::Center {
                    self.controls.toggle_rotation_mode();
                    return ViewAction::None;
                }
            }

            CameraControls::handle_zone_movement(camera, analysis);
            self.tetrahedron_rotations.clear();
        }

        ViewAction::None
    }

    pub fn handle_drag(
        &mut self,
        analysis: &PointerAnalysis,
        camera: &mut Camera,
        w_thickness: &mut f32,
        dichoptic_intensity: &mut f32,
    ) -> ViewAction {
        if matches!(analysis.drag_view, Some(DragView::Right)) {
            self.tetrahedron_rotations.clear();
        }
        self.controls
            .handle_drag(camera, analysis, w_thickness, dichoptic_intensity);
        ViewAction::None
    }

    pub fn handle_drag_start(&mut self, drag_view: DragView) {
        self.drag_state.drag_view = Some(drag_view);
    }

    pub fn clear_interaction_state(&mut self) {
        self.drag_state.clear();
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context, camera: &mut Camera, dt_scale: f32) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::U) {
                self.info_level = (self.info_level + 1) % 3;
            }
        });

        self.controls.handle_movement_keys(ctx, camera, dt_scale);
        if ctx.input(|i| {
            i.key_down(egui::Key::ArrowUp)
                || i.key_down(egui::Key::ArrowDown)
                || i.key_down(egui::Key::ArrowLeft)
                || i.key_down(egui::Key::ArrowRight)
                || i.key_down(egui::Key::PageUp)
                || i.key_down(egui::Key::PageDown)
                || i.key_down(egui::Key::Period)
                || i.key_down(egui::Key::Comma)
        }) {
            self.tetrahedron_rotations.clear();
        }
    }

    pub const fn zone_mode(&self) -> ZoneMode {
        self.zone_mode
    }
}
impl Default for SceneView {
    fn default() -> Self {
        Self::new()
    }
}
