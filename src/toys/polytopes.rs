//! Tesseract visualization toy

use eframe::egui;
use nalgebra::Vector4;

use crate::camera::Camera;
use crate::geometry::Bounds4D;
use crate::input::{DragView, TapAnalysis, ZoneMode};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{FourDSettings, StereoSettings};
use crate::toy::{CompassWaypoint, Toy, ViewAction};
use crate::toys::scene_view::{SceneRenderParams, SceneView};

const POSITION_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -10.0..=10.0;
const W_SLIDER_RANGE: std::ops::RangeInclusive<f32> = -3.0..=3.0;

pub struct PolytopesToy {
    camera: Camera,
    polytope_type: PolytopeType,
    cached_vertices: Vec<Vector4<f32>>,
    cached_indices: Vec<u16>,
    stereo: StereoSettings,
    four_d: FourDSettings,
    scene_view: SceneView,
}

impl Default for PolytopesToy {
    fn default() -> Self {
        Self::new()
    }
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
        }
    }

    fn ensure_polytope_cached(&mut self) {
        let (vertices, indices) = create_polytope(self.polytope_type);
        self.cached_vertices = vertices;
        self.cached_indices = indices;
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
            ui.add(
                egui::Slider::new(&mut self.camera.position.x, POSITION_SLIDER_RANGE.clone())
                    .text(""),
            );
            ui.label("Y:");
            ui.add(
                egui::Slider::new(&mut self.camera.position.y, POSITION_SLIDER_RANGE.clone())
                    .text(""),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Z:");
            ui.add(
                egui::Slider::new(&mut self.camera.position.z, POSITION_SLIDER_RANGE.clone())
                    .text(""),
            );
            ui.label("W:");
            ui.add(egui::Slider::new(&mut self.camera.position.w, W_SLIDER_RANGE.clone()).text(""));
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

    fn render_view_overlays(
        &self,
        left_painter: &egui::Painter,
        left_rect: egui::Rect,
        right_painter: &egui::Painter,
        right_rect: egui::Rect,
    ) {
        self.scene_view
            .render_overlays(left_painter, left_rect, right_painter, right_rect);
    }

    fn set_stereo_settings(&mut self, settings: &StereoSettings) {
        self.stereo = *settings;
    }

    fn set_four_d_settings(&mut self, settings: &FourDSettings) {
        self.four_d = *settings;
    }

    fn handle_tap(&mut self, analysis: &TapAnalysis) -> ViewAction {
        self.scene_view.handle_tap(analysis, &mut self.camera)
    }

    fn handle_drag(&mut self, _is_left_view: bool, from: egui::Pos2, to: egui::Pos2) {
        self.scene_view.handle_drag(&mut self.camera, from, to);
    }

    fn handle_hold(&mut self, analysis: &TapAnalysis) {
        self.scene_view.handle_hold(analysis, &mut self.camera);
    }

    fn handle_drag_start(&mut self, drag_view: DragView) {
        self.scene_view.handle_drag_start(drag_view);
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        self.scene_view.handle_keyboard(ctx, &mut self.camera);
    }

    fn visualization_rect(&self) -> Option<egui::Rect> {
        self.scene_view.visualization_rect
    }

    fn compass_vector(&self) -> Option<Vector4<f32>> {
        Some(-self.camera.position)
    }

    fn compass_reference_position(&self) -> Option<Vector4<f32>> {
        Some(self.camera.position)
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

    fn map_camera(&self) -> Option<&Camera> {
        Some(&self.camera)
    }

    fn compass_world_to_camera_frame(&self, world_vector: Vector4<f32>) -> Option<Vector4<f32>> {
        Some(self.camera.world_vector_to_camera_frame(world_vector))
    }

    fn zone_mode_for_view(&self, _is_left_view: bool) -> ZoneMode {
        self.scene_view.zone_mode()
    }

    fn clear_interaction_state(&mut self) {
        self.scene_view.clear_interaction_state();
    }
}
