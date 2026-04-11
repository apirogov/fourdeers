use eframe::egui;
use nalgebra::{UnitQuaternion, Vector4};
use std::collections::HashMap;

use crate::camera::{Camera, CameraProjection, Direction4D};
use crate::colors::LABEL_INACTIVE;
use crate::input::{
    zone_to_movement_action, DragState, DragView, TapAnalysis, TetraId, Zone, ZoneMode,
};
use crate::render::{
    draw_background, draw_center_divider, render_stereo_views, render_tap_zone_label,
    split_stereo_views, FourDSettings, StereoSettings, TesseractRenderConfig,
    TesseractRenderContext, W_THICKNESS_DRAG_SENSITIVITY, W_THICKNESS_MAX, W_THICKNESS_MIN,
};
use crate::toy::ViewAction;

const TAP_MOVE_SPEED: f32 = 0.15;
const HOLD_MOVE_SPEED: f32 = 0.08;
const KEYBOARD_MOVE_SPEED: f32 = 0.15;

pub struct SceneRenderParams<'a> {
    pub camera: &'a Camera,
    pub vertices: &'a [Vector4<f32>],
    pub indices: &'a [u16],
    pub four_d: FourDSettings,
    pub stereo: StereoSettings,
    pub show_debug: bool,
}

pub struct SceneView {
    pub show_directions: bool,
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
            show_directions: false,
            right_view_4d_rotation: false,
            zone_mode: ZoneMode::NineZones,
            visualization_rect: None,
            drag_state: DragState::new(),
            tetrahedron_rotations: HashMap::new(),
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, rect: egui::Rect, params: SceneRenderParams<'_>) {
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

        if params.show_debug {
            let right_rect = split_stereo_views(rect).1;
            let right_painter = ui.painter().with_clip_rect(right_rect);
            ctx.render_zone_labels(&right_painter, right_rect);
        }

        if self.show_directions {
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
    ) {
        let dir_label = if self.show_directions {
            "Dir:On"
        } else {
            "Dir:Off"
        };
        render_tap_zone_label(left_painter, left_rect, Zone::NorthEast, dir_label, None);

        let rot_label = if self.right_view_4d_rotation {
            "Rot:4D"
        } else {
            "Rot:3D"
        };
        let gray = Some(LABEL_INACTIVE);
        render_tap_zone_label(right_painter, right_rect, Zone::NorthEast, rot_label, gray);
    }

    pub fn handle_tap(&mut self, analysis: &TapAnalysis, camera: &mut Camera) -> ViewAction {
        if analysis.is_left_view && analysis.zone == Zone::NorthEast {
            self.show_directions = !self.show_directions;
            return ViewAction::None;
        }

        if !analysis.is_left_view && analysis.zone == Zone::Center {
            self.right_view_4d_rotation = !self.right_view_4d_rotation;
            return ViewAction::None;
        }

        if let Some(action) = Self::zone_to_action(analysis.zone, analysis.is_left_view) {
            self.apply_camera_action(camera, action, TAP_MOVE_SPEED);
        }

        ViewAction::None
    }

    pub fn handle_drag(
        &mut self,
        camera: &mut Camera,
        from: egui::Pos2,
        to: egui::Pos2,
        w_thickness: &mut f32,
    ) {
        let delta = to - from;

        match self.drag_state.drag_view {
            Some(DragView::Left) => {
                *w_thickness = (*w_thickness + delta.x * W_THICKNESS_DRAG_SENSITIVITY)
                    .clamp(W_THICKNESS_MIN, W_THICKNESS_MAX);
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
    }

    pub fn handle_hold(&mut self, analysis: &TapAnalysis, camera: &mut Camera) {
        if let Some(action) = Self::zone_to_action(analysis.zone, analysis.is_left_view) {
            self.apply_camera_action(camera, action, HOLD_MOVE_SPEED);
        }
    }

    pub fn handle_drag_start(&mut self, drag_view: DragView) {
        self.drag_state.drag_view = Some(drag_view);
    }

    pub fn clear_interaction_state(&mut self) {
        self.drag_state.clear();
    }

    pub fn handle_keyboard(&mut self, ctx: &egui::Context, camera: &mut Camera) {
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
