//! Tesseract visualization toy

use eframe::egui;
use nalgebra::Vector4;

use crate::camera::Camera;
use crate::geometry::Bounds4D;
use crate::input::{
    analyze_tap_in_stereo_view_with_modes, zone_from_rect, DragView, Zone, ZoneMode,
};
use crate::map::{MapRenderParams, MapView};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    adjust_w_thickness, render_tap_zone_label, split_stereo_views, CompassFrameMode, FourDSettings,
    StereoSettings,
};
use crate::toy::{CompassWaypoint, Toy, ViewAction};
use crate::toys::scene_view::{SceneRenderParams, SceneView};
use crate::view::CompassView;

const POSITION_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -10.0..=10.0;
const W_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -3.0..=3.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ActiveViewId {
    #[default]
    Scene,
    Map,
    Compass,
}

#[derive(Default)]
pub struct PolytopesToy {
    camera: Camera,
    polytope_type: PolytopeType,
    cached_vertices: Vec<Vector4<f32>>,
    cached_indices: Vec<u16>,
    stereo: StereoSettings,
    four_d: FourDSettings,
    scene_view: SceneView,
    map: MapView,
    compass: CompassView,
    active_view: ActiveViewId,
}

impl PolytopesToy {
    #[must_use]
    pub fn new() -> Self {
        let polytope_type = PolytopeType::EightCell;
        let (cached_vertices, cached_indices) = create_polytope(polytope_type);
        Self {
            camera: Camera::new(),
            polytope_type,
            cached_vertices,
            cached_indices,
            stereo: StereoSettings::new(),
            four_d: FourDSettings::default(),
            scene_view: SceneView::new(),
            map: MapView::new(),
            compass: CompassView::new(),
            active_view: ActiveViewId::default(),
        }
    }

    fn ensure_polytope_cached(&mut self) {
        let (vertices, indices) = create_polytope(self.polytope_type);
        self.cached_vertices = vertices;
        self.cached_indices = indices;
    }

    fn toggle_view(&mut self, view: ActiveViewId) {
        self.active_view = if self.active_view == view {
            ActiveViewId::Scene
        } else {
            view
        };
    }

    fn render_camera_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Position: ({:.1}, {:.1}, {:.1}, {:.1})",
            self.camera.position.x,
            self.camera.position.y,
            self.camera.position.z,
            self.camera.position.w
        ));

        ui.horizontal(|ui| {
            ui.label("X:");
            ui.add(egui::Slider::new(&mut self.camera.position.x, POSITION_SLIDER_RANGE).text(""));
            ui.label("Y:");
            ui.add(egui::Slider::new(&mut self.camera.position.y, POSITION_SLIDER_RANGE).text(""));
        });

        ui.horizontal(|ui| {
            ui.label("Z:");
            ui.add(egui::Slider::new(&mut self.camera.position.z, POSITION_SLIDER_RANGE).text(""));
            ui.label("W:");
            ui.add(egui::Slider::new(&mut self.camera.position.w, W_SLIDER_RANGE).text(""));
        });

        ui.horizontal(|ui| {
            let mut yaw_l = self.camera.yaw_l();
            ui.label("Yaw(L)");
            if ui
                .add(
                    egui::Slider::new(&mut yaw_l, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_yaw_l(yaw_l);
            }
            let mut pitch_l = self.camera.pitch_l();
            ui.label("Pitch(L)");
            if ui
                .add(
                    egui::Slider::new(&mut pitch_l, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_pitch_l(pitch_l);
            }
        });

        ui.horizontal(|ui| {
            let mut yaw_r = self.camera.yaw_r();
            ui.label("Yaw(R)");
            if ui
                .add(
                    egui::Slider::new(&mut yaw_r, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_yaw_r(yaw_r);
            }
            let mut pitch_r = self.camera.pitch_r();
            ui.label("Pitch(R)");
            if ui
                .add(
                    egui::Slider::new(&mut pitch_r, -std::f32::consts::PI..=std::f32::consts::PI)
                        .text(""),
                )
                .changed()
            {
                self.camera.set_pitch_r(pitch_r);
            }
        });
    }

    fn render_compass(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let waypoints = self.compass_waypoints();
        let reference = self.camera.position;
        let (mut vector_4d, waypoint_title) =
            if let Some(waypoint) = self.compass.current_waypoint(&waypoints) {
                (waypoint.position - reference, waypoint.title)
            } else {
                (-reference, "Compass")
            };

        if matches!(self.compass.frame_mode, CompassFrameMode::Camera) {
            vector_4d = self.camera.world_vector_to_camera_frame(vector_4d);
        }

        self.compass
            .render(ui, rect, vector_4d, waypoint_title, self.stereo);
    }

    fn render_map(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let waypoints = self.compass_waypoints();
        let geometry_bounds = self.scene_geometry_bounds();
        let params = MapRenderParams {
            scene_camera: &self.camera,
            waypoints: &waypoints,
            stereo: self.stereo,
            frame_mode: self.map.frame_mode,
            geometry_bounds,
            four_d: self.four_d,
        };
        self.map.render(ui, rect, &params);
    }

    fn handle_scene_tap(&mut self, pos: egui::Pos2, vis_rect: egui::Rect) -> ViewAction {
        let left_zone_mode = self.scene_view.zone_mode();
        if let Some(analysis) =
            analyze_tap_in_stereo_view_with_modes(vis_rect, pos, left_zone_mode, left_zone_mode)
        {
            self.scene_view.handle_tap(&analysis, &mut self.camera)
        } else {
            ViewAction::None
        }
    }

    fn handle_map_tap(&mut self, left_zone: Option<Zone>, right_rect: egui::Rect, pos: egui::Pos2) {
        let waypoints = self.compass_waypoints();
        let geometry_bounds = self.scene_geometry_bounds();
        let action = self.map.handle_tap(
            left_zone,
            right_rect,
            pos,
            Some(&self.camera),
            &waypoints,
            geometry_bounds,
        );
        if let ViewAction::SelectWaypoint(idx) = action {
            self.compass.waypoint_index = idx;
            self.active_view = ActiveViewId::Compass;
        }
    }

    fn handle_scene_hold(&mut self, pos: egui::Pos2, vis_rect: egui::Rect) {
        let left_zone_mode = self.scene_view.zone_mode();
        if let Some(analysis) =
            analyze_tap_in_stereo_view_with_modes(vis_rect, pos, left_zone_mode, left_zone_mode)
        {
            self.scene_view.handle_hold(&analysis, &mut self.camera);
        }
    }

    fn compass_waypoints(&self) -> Vec<CompassWaypoint> {
        vec![
            CompassWaypoint {
                title: "Origin",
                position: Vector4::new(0.0, 0.0, 0.0, 0.0),
            },
            CompassWaypoint {
                title: "TestPoint",
                position: Vector4::new(1.0, 2.0, 3.0, 4.0),
            },
        ]
    }

    fn scene_geometry_bounds(&self) -> Option<Bounds4D> {
        if self.cached_vertices.is_empty() {
            return None;
        }
        let mut bounds = Bounds4D::from_point(self.cached_vertices[0]);
        for v in &self.cached_vertices[1..] {
            bounds = bounds.expanded_to(*v);
        }
        Some(bounds)
    }
}

impl Toy for PolytopesToy {
    fn name(&self) -> &'static str {
        "Polytopes"
    }

    fn id(&self) -> &'static str {
        "polytopes"
    }

    fn reset(&mut self) {
        self.camera.reset();
        self.scene_view.tetrahedron_rotations.clear();
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label("4D Polytope Visualization");

        let prev_type = self.polytope_type;
        egui::ComboBox::from_label("")
            .selected_text(self.polytope_type.to_string())
            .show_ui(ui, |ui| {
                for poly_type in PolytopeType::all() {
                    ui.selectable_value(&mut self.polytope_type, poly_type, poly_type.to_string());
                }
            });
        if self.polytope_type != prev_type {
            self.ensure_polytope_cached();
            self.camera.reset();
        }

        ui.label(format!(
            "{} vertices, {} edges",
            self.polytope_type.vertex_count(),
            self.polytope_type.edge_count()
        ));

        ui.separator();

        ui.collapsing("Controls", |ui| {
            ui.label("Arrows Up/Down: Y | Arrows Left/Right: X | PgUp/Dn: Z | ,/. : W");
        });

        ui.add_space(8.0);

        ui.collapsing("Camera", |ui| {
            self.render_camera_controls(ui);
        });

        ui.separator();
        ui.add_space(4.0);

        ui.add_space(4.0);
    }

    fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, show_debug: bool) {
        match self.active_view {
            ActiveViewId::Scene => {
                self.scene_view.render(
                    ui,
                    rect,
                    SceneRenderParams {
                        camera: &self.camera,
                        vertices: &self.cached_vertices,
                        indices: &self.cached_indices,
                        four_d: self.four_d,
                        stereo: self.stereo,
                        show_debug,
                    },
                );
            }
            ActiveViewId::Map => {
                self.render_map(ui, rect);
            }
            ActiveViewId::Compass => {
                self.render_compass(ui, rect);
            }
        }
    }

    fn render_view_overlays(
        &self,
        left_painter: &egui::Painter,
        left_rect: egui::Rect,
        right_painter: &egui::Painter,
        right_rect: egui::Rect,
    ) {
        let map_label = if self.active_view == ActiveViewId::Map {
            "Close"
        } else {
            "Map"
        };
        render_tap_zone_label(left_painter, left_rect, Zone::West, map_label, None);

        let compass_label = if self.active_view == ActiveViewId::Compass {
            "Close"
        } else {
            "Compass"
        };
        render_tap_zone_label(
            left_painter,
            left_rect,
            Zone::SouthWest,
            compass_label,
            None,
        );

        match self.active_view {
            ActiveViewId::Scene => {
                self.scene_view.render_overlays(
                    left_painter,
                    left_rect,
                    right_painter,
                    right_rect,
                    self.four_d.w_thickness,
                    &self.camera,
                );
            }
            ActiveViewId::Map => {
                self.map.render_overlays(
                    left_painter,
                    left_rect,
                    right_painter,
                    right_rect,
                    self.four_d.w_thickness,
                );
            }
            ActiveViewId::Compass => {
                self.compass.render_overlays(
                    left_painter,
                    left_rect,
                    right_painter,
                    right_rect,
                    self.four_d.w_thickness,
                );
            }
        }
    }

    fn set_four_d_settings(&mut self, settings: &FourDSettings) {
        self.four_d = *settings;
    }

    fn set_stereo_settings(&mut self, settings: &StereoSettings) {
        self.stereo = *settings;
    }

    fn handle_tap(&mut self, pos: egui::Pos2, vis_rect: egui::Rect) -> ViewAction {
        let (left_rect, right_rect) = split_stereo_views(vis_rect);
        let left_zone = zone_from_rect(left_rect, pos, ZoneMode::NineZones);

        match left_zone {
            Some(Zone::West) => {
                self.toggle_view(ActiveViewId::Map);
                return ViewAction::None;
            }
            Some(Zone::SouthWest) => {
                self.toggle_view(ActiveViewId::Compass);
                return ViewAction::None;
            }
            _ => {}
        }

        match self.active_view {
            ActiveViewId::Scene => self.handle_scene_tap(pos, vis_rect),
            ActiveViewId::Map => {
                self.handle_map_tap(left_zone, right_rect, pos);
                ViewAction::None
            }
            ActiveViewId::Compass => {
                let waypoints_len = self.compass_waypoints().len();
                self.compass
                    .handle_tap(left_zone, right_rect, pos, waypoints_len);
                ViewAction::None
            }
        }
    }

    fn handle_drag(
        &mut self,
        is_left_view: bool,
        from: egui::Pos2,
        to: egui::Pos2,
        w_thickness: &mut f32,
    ) {
        match self.active_view {
            ActiveViewId::Scene => {
                self.scene_view
                    .handle_drag(&mut self.camera, from, to, w_thickness);
            }
            ActiveViewId::Map => {
                if is_left_view {
                    let delta = to - from;
                    *w_thickness = adjust_w_thickness(*w_thickness, delta.x);
                } else {
                    self.map.handle_drag(from, to);
                }
            }
            ActiveViewId::Compass => {
                self.compass.handle_drag(from, to);
            }
        }
    }

    fn handle_hold(&mut self, pos: egui::Pos2, vis_rect: egui::Rect) {
        match self.active_view {
            ActiveViewId::Scene => {
                self.handle_scene_hold(pos, vis_rect);
            }
            ActiveViewId::Map => {
                let (_, right_rect) = split_stereo_views(vis_rect);
                self.map.handle_hold(right_rect, pos);
            }
            ActiveViewId::Compass => {}
        }
    }

    fn handle_drag_start(&mut self, drag_view: DragView) {
        if self.active_view == ActiveViewId::Scene {
            self.scene_view.handle_drag_start(drag_view);
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::C) {
                self.toggle_view(ActiveViewId::Compass);
            }
            if i.key_pressed(egui::Key::G) {
                self.toggle_view(ActiveViewId::Map);
            }
        });

        match self.active_view {
            ActiveViewId::Scene => {
                self.scene_view.handle_keyboard(ctx, &mut self.camera);
            }
            ActiveViewId::Map => {
                self.map.handle_keyboard(ctx);
            }
            ActiveViewId::Compass => {
                let waypoints_len = self.compass_waypoints().len();
                ctx.input(|i| {
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        self.compass.cycle_waypoint(-1, waypoints_len);
                    }
                    if i.key_pressed(egui::Key::ArrowRight) {
                        self.compass.cycle_waypoint(1, waypoints_len);
                    }
                    if i.key_pressed(egui::Key::F) {
                        self.compass.frame_mode = self.compass.frame_mode.other();
                    }
                });
            }
        }
    }

    fn zone_mode_for_view(&self, _is_left_view: bool) -> ZoneMode {
        match self.active_view {
            ActiveViewId::Scene => self.scene_view.zone_mode(),
            _ => ZoneMode::NineZones,
        }
    }

    fn clear_interaction_state(&mut self) {
        if self.active_view == ActiveViewId::Scene {
            self.scene_view.clear_interaction_state();
        }
    }
}
