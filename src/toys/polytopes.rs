//! Multi-polytope scene with all 6 regular convex 4-polytopes

use eframe::egui;
use nalgebra::Vector4;

use crate::camera::Camera;
use crate::geometry::Bounds4D;
use crate::input::{DragView, PointerAnalysis, Zone, ZoneMode};
use crate::map::{MapRenderParams, MapView};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{render_tap_zone_label, CompassFrameMode, FourDSettings, StereoSettings};
use crate::toy::{CompassWaypoint, Toy, ViewAction};
use crate::toys::scene_view::{SceneRenderParams, SceneView};
use crate::view::CompassView;

const POSITION_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -10.0..=10.0;
const W_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -3.0..=3.0;
const JUMP_DISTANCE: f32 = 5.0;

const BOX_BOUND: f32 = 10.0;
const MIN_PAIR_DIST: f32 = 10.0;
const MIN_ORIGIN_DIST: f32 = 5.0;
const REPULSION_ITERS: usize = 50;

const ORIGIN_WAYPOINT_TITLE: &str = "Origin";
const INITIAL_WAYPOINT_INDEX: usize = 1;

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn from_seed() -> Self {
        let mut seed = [0u8; 8];
        getrandom::getrandom(&mut seed).expect("getrandom failed");
        let state = u64::from_ne_bytes(seed);
        assert!(state != 0, "PRNG seed must not be zero");
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self, min: f32, max: f32) -> f32 {
        let bits = self.next_u64();
        let t = (bits as f64) / (u64::MAX as f64);
        min + (max - min) * t as f32
    }

    fn shuffle<T>(&mut self, slice: &mut [T]) {
        let len = slice.len();
        if len < 2 {
            return;
        }
        for i in (1..len).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            slice.swap(i, j);
        }
    }
}

struct ScenePolytope {
    polytope_type: PolytopeType,
    position: Vector4<f32>,
    world_vertices: Vec<Vector4<f32>>,
    indices: Vec<u16>,
}

impl ScenePolytope {
    fn new(polytope_type: PolytopeType, position: Vector4<f32>) -> Self {
        let (base_vertices, indices) = create_polytope(polytope_type);
        let world_vertices: Vec<Vector4<f32>> =
            base_vertices.into_iter().map(|v| v + position).collect();
        Self {
            polytope_type,
            position,
            world_vertices,
            indices,
        }
    }
}

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
    polytopes: Vec<ScenePolytope>,
    merged_vertices: Vec<Vector4<f32>>,
    merged_indices: Vec<u16>,
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
        let polytopes = Self::build_scene();
        let (merged_vertices, merged_indices) = Self::merge_geometry(&polytopes);

        let mut camera = Camera::new();
        if let Some(first) = polytopes.first() {
            camera.position = first.position + Vector4::new(0.0, 0.0, -JUMP_DISTANCE, 0.0);
            camera.set_yaw_l(0.0);
            camera.set_pitch_l(0.0);
            camera.set_yaw_r(0.0);
            camera.set_pitch_r(0.0);
        }

        let mut compass = CompassView::new();
        compass.set_waypoint_index(INITIAL_WAYPOINT_INDEX);

        Self {
            camera,
            polytopes,
            merged_vertices,
            merged_indices,
            stereo: StereoSettings::new(),
            four_d: FourDSettings::default(),
            scene_view: SceneView::new(),
            map: MapView::new(),
            compass,
            active_view: ActiveViewId::default(),
        }
    }

    fn generate_positions(rng: &mut XorShift64) -> Vec<(PolytopeType, Vector4<f32>)> {
        let mut positions: Vec<Vector4<f32>> = (0..6)
            .map(|_| {
                Vector4::new(
                    rng.next_f32(-BOX_BOUND, BOX_BOUND),
                    rng.next_f32(-BOX_BOUND, BOX_BOUND),
                    rng.next_f32(-BOX_BOUND, BOX_BOUND),
                    rng.next_f32(-BOX_BOUND, BOX_BOUND),
                )
            })
            .collect();

        for _ in 0..REPULSION_ITERS {
            for i in 0..positions.len() {
                for j in (i + 1)..positions.len() {
                    let diff = positions[i] - positions[j];
                    let dist = diff.norm();
                    if dist < MIN_PAIR_DIST && dist > 1e-6 {
                        let push = diff * ((MIN_PAIR_DIST - dist) * 0.5 / dist);
                        positions[i] += push;
                        positions[j] -= push;
                    }
                }
                let dist_origin = positions[i].norm();
                if dist_origin < MIN_ORIGIN_DIST && dist_origin > 1e-6 {
                    let push = positions[i] * ((MIN_ORIGIN_DIST - dist_origin) / dist_origin);
                    positions[i] += push;
                }
                positions[i] = positions[i].map(|c| c.clamp(-BOX_BOUND, BOX_BOUND));
            }
        }

        let mut types: Vec<PolytopeType> = PolytopeType::all().to_vec();
        rng.shuffle(&mut types);

        types.into_iter().zip(positions).collect()
    }

    fn build_scene() -> Vec<ScenePolytope> {
        let mut rng = XorShift64::from_seed();
        Self::generate_positions(&mut rng)
            .into_iter()
            .map(|(pt, pos)| ScenePolytope::new(pt, pos))
            .collect()
    }

    fn merge_geometry(polytopes: &[ScenePolytope]) -> (Vec<Vector4<f32>>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_offset: u32 = 0;
        for p in polytopes {
            vertices.extend_from_slice(&p.world_vertices);
            indices.extend(p.indices.iter().map(|i| vertex_offset + *i as u32));
            vertex_offset += p.world_vertices.len() as u32;
        }
        let merged_indices: Vec<u16> = indices
            .into_iter()
            .map(|i| {
                let result = i as u16;
                assert_eq!(result as u32, i, "index overflow in merged geometry");
                result
            })
            .collect();
        (vertices, merged_indices)
    }

    fn toggle_view(&mut self, view: ActiveViewId) {
        self.active_view = if self.active_view == view {
            ActiveViewId::Scene
        } else {
            view
        };
    }

    fn jump_to_selected_waypoint(&mut self) {
        let waypoints = self.compass_waypoints();
        if let Some(wp) = self.compass.current_waypoint(&waypoints) {
            self.camera.position = wp.position + Vector4::new(0.0, 0.0, -JUMP_DISTANCE, 0.0);
            self.camera.set_yaw_l(0.0);
            self.camera.set_pitch_l(0.0);
            self.camera.set_yaw_r(0.0);
            self.camera.set_pitch_r(0.0);
            self.active_view = ActiveViewId::Scene;
        }
    }

    fn select_waypoint(&mut self, waypoint_index: usize) {
        let waypoints_len = self.compass_waypoints().len();
        if waypoint_index < waypoints_len {
            self.compass.set_waypoint_index(waypoint_index);
            self.active_view = ActiveViewId::Compass;
        }
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

    fn compass_waypoints(&self) -> Vec<CompassWaypoint> {
        let mut waypoints = vec![CompassWaypoint {
            title: ORIGIN_WAYPOINT_TITLE,
            position: Vector4::zeros(),
        }];
        for p in &self.polytopes {
            waypoints.push(CompassWaypoint {
                title: p.polytope_type.short_name(),
                position: p.position,
            });
        }
        waypoints
    }

    fn scene_geometry_bounds(&self) -> Option<Bounds4D> {
        let first_vertex = self.polytopes.first()?.world_vertices.first()?;
        let mut bounds = Bounds4D::from_point(*first_vertex);
        for p in &self.polytopes {
            for v in &p.world_vertices {
                bounds = bounds.expanded_to(*v);
            }
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
        ui.label("4D Polytope Scene");

        let total_vertices: usize = self.polytopes.iter().map(|p| p.world_vertices.len()).sum();
        let total_edges: usize = self.polytopes.iter().map(|p| p.indices.len() / 2).sum();
        ui.label(format!(
            "{} polytopes \u{2022} {} vertices \u{2022} {} edges",
            self.polytopes.len(),
            total_vertices,
            total_edges
        ));

        ui.separator();

        ui.collapsing("Controls", |ui| {
            ui.label("Movement (hold):");
            ui.label("  Arrows: Up/Down/Left/Right");
            ui.label("  PgUp/PgDn: Forward/Backward");
            ui.label("  Comma/Period: Ana/Kata");
            ui.label("");
            ui.label("Views:");
            ui.label("  M: Menu | C: Compass | G: Map");
            ui.label("  U: UI info (scene view)");
            ui.label("");
            ui.label("Waypoints:");
            ui.label("  Arrow Left/Right: cycle (compass)");
            ui.label("  J: Jump to waypoint");
            ui.label("  F: Toggle frame mode");
            ui.label("  L: Toggle labels (map)");
        });

        ui.add_space(8.0);

        ui.collapsing("Camera", |ui| {
            self.render_camera_controls(ui);
        });

        ui.separator();
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
                        vertices: &self.merged_vertices,
                        indices: &self.merged_indices,
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

    fn handle_pointer(&mut self, analysis: PointerAnalysis) -> ViewAction {
        if analysis.is_left_view && !analysis.is_hold {
            if let Some(zone) = analysis.zone {
                match zone {
                    Zone::West => {
                        self.toggle_view(ActiveViewId::Map);
                        return ViewAction::None;
                    }
                    Zone::SouthWest => {
                        self.toggle_view(ActiveViewId::Compass);
                        return ViewAction::None;
                    }
                    _ => {}
                }
            }
        }

        if matches!(self.active_view, ActiveViewId::Compass) && !analysis.is_left_view {
            if let Some(zone) = analysis.zone {
                if !analysis.is_hold && zone == Zone::North {
                    self.jump_to_selected_waypoint();
                    return ViewAction::None;
                }
            }
        }

        let action = match self.active_view {
            ActiveViewId::Scene => self.scene_view.handle_pointer(&analysis, &mut self.camera),
            ActiveViewId::Map => {
                let waypoints = self.compass_waypoints();
                let geometry_bounds = self.scene_geometry_bounds();
                self.map
                    .handle_pointer(&analysis, Some(&self.camera), &waypoints, geometry_bounds)
            }
            ActiveViewId::Compass => {
                let waypoints_len = self.compass_waypoints().len();
                self.compass.handle_pointer(&analysis, waypoints_len)
            }
        };

        if let ViewAction::SelectWaypoint(waypoint_idx) = action {
            self.select_waypoint(waypoint_idx);
            return ViewAction::None;
        }

        action
    }

    fn handle_drag(&mut self, analysis: PointerAnalysis, w_thickness: &mut f32) -> ViewAction {
        match self.active_view {
            ActiveViewId::Scene => {
                self.scene_view
                    .handle_drag(&analysis, &mut self.camera, w_thickness)
            }
            ActiveViewId::Map => self.map.handle_drag(&analysis, w_thickness),
            ActiveViewId::Compass => self.compass.handle_drag(&analysis),
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
            if i.key_pressed(egui::Key::J) {
                self.jump_to_selected_waypoint();
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
