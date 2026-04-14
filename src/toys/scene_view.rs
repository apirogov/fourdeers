use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{Camera, CameraProjection, Direction4D};
use crate::colors::LABEL_INACTIVE;
use crate::gpu::GpuRenderer;
use crate::input::{
    zone_to_movement_action, DragState, DragView, PointerAnalysis, TetraId, Zone, ZoneMode,
    HOLD_MOVE_SPEED,
};
use crate::input::{KEYBOARD_MOVE_SPEED, TAP_MOVE_SPEED};
use crate::render::{
    adjust_w_thickness, create_stereo_projectors, draw_background, draw_center_divider,
    render_stereo_views, render_tap_zone_label, split_stereo_views, FourDSettings, StereoSettings,
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
    pub right_view_4d_rotation: bool,
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
            right_view_4d_rotation: false,
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
        gpu: Option<&GpuRenderer>,
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

        if let Some(gpu) = gpu {
            let views = create_stereo_projectors(
                rect,
                params.stereo.eye_separation,
                params.stereo.projection_distance,
                params.stereo.projection_mode,
            );

            let screen_size = [rect.width(), rect.height()];

            let (left_verts, left_idx) =
                ctx.collect_edge_vertices(&views.left_projector, &transformed, views.left_rect);
            let left_painter = ui.painter().with_clip_rect(views.left_rect);
            gpu.submit(
                &left_painter,
                views.left_rect,
                left_verts,
                left_idx,
                screen_size,
            );

            let (right_verts, right_idx) =
                ctx.collect_edge_vertices(&views.right_projector, &transformed, views.right_rect);
            let right_painter = ui.painter().with_clip_rect(views.right_rect);
            gpu.submit(
                &right_painter,
                views.right_rect,
                right_verts,
                right_idx,
                screen_size,
            );
        } else {
            render_stereo_views(
                ui,
                rect,
                params.stereo.eye_separation,
                params.stereo.projection_distance,
                params.stereo.projection_mode,
                |painter, projector, view_rect| {
                    ctx.render_edges(painter, projector, &transformed, view_rect);
                },
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

            let rot_label = if self.right_view_4d_rotation {
                "Rot:4D"
            } else {
                "Rot:3D"
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
            // Only allow toggle actions on tap (not hold)
            if !analysis.is_hold {
                if analysis.is_left_view && zone == Zone::North {
                    self.info_level = (self.info_level + 1) % 3;
                    return ViewAction::None;
                }

                if !analysis.is_left_view && zone == Zone::Center {
                    self.right_view_4d_rotation = !self.right_view_4d_rotation;
                    return ViewAction::None;
                }
            }

            // Movement actions work on both tap and hold
            if let Some(action) = Self::zone_to_action(zone, analysis.is_left_view) {
                let speed = if analysis.is_hold {
                    HOLD_MOVE_SPEED
                } else {
                    TAP_MOVE_SPEED
                };
                self.apply_camera_action(camera, action, speed);
            }
        }

        ViewAction::None
    }

    pub fn handle_drag(
        &mut self,
        analysis: &PointerAnalysis,
        camera: &mut Camera,
        w_thickness: &mut f32,
    ) -> ViewAction {
        let delta = analysis.drag_delta;

        match analysis.drag_view {
            Some(DragView::Left) => {
                *w_thickness = adjust_w_thickness(*w_thickness, delta.x);
            }
            Some(DragView::Right) => {
                if self.right_view_4d_rotation {
                    camera.rotate_4d(delta.x, delta.y);
                } else {
                    camera.rotate(delta.x, delta.y);
                }
                self.tetrahedron_rotations.clear();
            }
            None => {}
        }
        ViewAction::None
    }

    pub fn handle_drag_start(&mut self, drag_view: DragView) {
        self.drag_state.drag_view = Some(drag_view);
    }

    pub fn clear_interaction_state(&mut self) {
        self.drag_state.clear();
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context, camera: &mut Camera) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::U) {
                self.info_level = (self.info_level + 1) % 3;
            }
        });

        crate::input::handle_movement_keys(ctx, KEYBOARD_MOVE_SPEED, |action, speed| {
            self.apply_camera_action(camera, action, speed);
        });
    }

    pub const fn zone_mode(&self) -> ZoneMode {
        self.zone_mode
    }

    fn apply_camera_action(&mut self, camera: &mut Camera, action: Direction4D, speed: f32) {
        self.tetrahedron_rotations.clear();
        camera.apply_action(action, speed);
    }

    const fn zone_to_action(zone: Zone, is_left_view: bool) -> Option<Direction4D> {
        if is_left_view {
            None
        } else {
            zone_to_movement_action(zone)
        }
    }
}
impl Default for SceneView {
    fn default() -> Self {
        Self::new()
    }
}
