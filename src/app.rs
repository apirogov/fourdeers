//! Main application

use eframe::egui;
use nalgebra::{UnitQuaternion, Vector3, Vector4};

use crate::colors::panel_fill;
use crate::input::render_zone_debug_overlay;
use crate::input::{
    analyze_tap_in_stereo_view_with_modes, get_zone_from_rect, DragView, Zone, ZoneDebugOptions,
    ZoneMode,
};
use crate::render::{
    draw_background, draw_center_divider, render_common_menu_half, render_stereo_views,
    render_tap_zone_label, split_stereo_views, CompassFrameMode, FourDSettings, ProjectionMode,
    StereoSettings,
};
use crate::toy::{CompassWaypoint, ToyManager};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ActiveView {
    #[default]
    Main,
    Compass,
    Map,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CommonSettings {
    pub show_debug: bool,
    pub four_d: FourDSettings,
    pub stereo: StereoSettings,
}

pub struct FourDeersApp {
    toy_manager: ToyManager,
    menu_open: bool,
    settings: CommonSettings,
    last_tap_time: Option<f64>,
    mouse_down_pos: Option<egui::Pos2>,
    mouse_down_time: Option<f64>,
    is_drag_mode: bool,
    drag_view: Option<DragView>,
    last_drag_pos: Option<egui::Pos2>,
    visualization_rect: Option<egui::Rect>,
    active_view: ActiveView,
    compass_rotation: UnitQuaternion<f32>,
    compass_waypoint_index: usize,
    compass_frame_mode: CompassFrameMode,
}

impl FourDeersApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            toy_manager: ToyManager::new(),
            menu_open: false,
            settings: CommonSettings::default(),
            last_tap_time: None,
            mouse_down_pos: None,
            mouse_down_time: None,
            is_drag_mode: false,
            drag_view: None,
            last_drag_pos: None,
            visualization_rect: None,
            active_view: ActiveView::default(),
            compass_rotation: UnitQuaternion::identity(),
            compass_waypoint_index: 0,
            compass_frame_mode: CompassFrameMode::World,
        }
    }
}

impl eframe::App for FourDeersApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::M) {
                self.menu_open = !self.menu_open;
            }
            if i.key_pressed(egui::Key::C) {
                self.active_view = if self.active_view == ActiveView::Compass {
                    ActiveView::Main
                } else {
                    ActiveView::Compass
                };
            }
            if i.key_pressed(egui::Key::G) {
                self.active_view = if self.active_view == ActiveView::Map {
                    ActiveView::Main
                } else {
                    ActiveView::Map
                };
            }
            if self.active_view == ActiveView::Compass {
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.cycle_compass_waypoint(-1);
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.cycle_compass_waypoint(1);
                }
                if i.key_pressed(egui::Key::F) {
                    self.toggle_compass_frame_mode();
                }
            }
        });

        self.process_pointer_events(ctx);
        if self.active_view == ActiveView::Main {
            self.toy_manager.active_toy_mut().handle_keyboard(ctx);
        }
        self.render_ui(ctx);
        ctx.request_repaint();
    }
}

impl FourDeersApp {
    fn current_compass_waypoint(&mut self) -> Option<CompassWaypoint> {
        let waypoints = self.toy_manager.active_toy().compass_waypoints();
        if waypoints.is_empty() {
            return None;
        }
        if self.compass_waypoint_index >= waypoints.len() {
            self.compass_waypoint_index = 0;
        }
        Some(waypoints[self.compass_waypoint_index].clone())
    }

    fn cycle_compass_waypoint(&mut self, direction: i32) {
        let waypoints = self.toy_manager.active_toy().compass_waypoints();
        if waypoints.is_empty() {
            return;
        }

        let len = waypoints.len() as i32;
        let idx = self.compass_waypoint_index as i32;
        self.compass_waypoint_index = (idx + direction).rem_euclid(len) as usize;
    }

    fn toggle_compass_frame_mode(&mut self) {
        self.compass_frame_mode = match self.compass_frame_mode {
            CompassFrameMode::World => CompassFrameMode::Camera,
            CompassFrameMode::Camera => CompassFrameMode::World,
        };
    }

    fn render_compass_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        draw_background(ui, rect);
        draw_center_divider(ui, rect);

        let (mut vector_4d, waypoint_title) =
            if let Some(waypoint) = self.current_compass_waypoint() {
                let reference = self
                    .toy_manager
                    .active_toy()
                    .compass_reference_position()
                    .unwrap_or_else(Vector4::zeros);
                (waypoint.position - reference, waypoint.title)
            } else {
                (
                    self.toy_manager
                        .active_toy()
                        .compass_vector()
                        .unwrap_or_else(Vector4::zeros),
                    "Compass",
                )
            };

        if matches!(self.compass_frame_mode, CompassFrameMode::Camera) {
            if let Some(camera_frame) = self
                .toy_manager
                .active_toy()
                .compass_world_to_camera_frame(vector_4d)
            {
                vector_4d = camera_frame;
            }
        }

        use crate::tetrahedron::{format_magnitude, magnitude_4d, TetrahedronGadget};
        let magnitude_label = format_magnitude(magnitude_4d(vector_4d));
        let gadget = TetrahedronGadget::from_4d_vector_with_quaternion(
            vector_4d,
            self.compass_rotation,
            1.0,
        )
        .with_base_label(magnitude_label)
        .with_tip_label(waypoint_title);

        let eye_sep = self.settings.stereo.eye_separation;
        let proj_dist = self.settings.stereo.projection_distance;
        render_stereo_views(
            ui,
            rect,
            eye_sep,
            proj_dist,
            ProjectionMode::Orthographic,
            |painter, projector, _view_rect| {
                use crate::render::render_tetrahedron_with_projector;
                render_tetrahedron_with_projector(
                    painter,
                    &gadget,
                    projector,
                    self.compass_frame_mode,
                );
            },
        );
    }

    fn render_map_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        draw_background(ui, rect);
        draw_center_divider(ui, rect);

        let eye_sep = self.settings.stereo.eye_separation;
        let proj_dist = self.settings.stereo.projection_distance;
        render_stereo_views(
            ui,
            rect,
            eye_sep,
            proj_dist,
            ProjectionMode::Orthographic,
            |painter, _projector, view_rect| {
                let center = view_rect.center();
                let font_id = egui::FontId::proportional(16.0);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    "4D Map",
                    font_id,
                    egui::Color32::from_rgb(120, 120, 140),
                );
            },
        );
    }

    fn process_pointer_events(&mut self, ctx: &egui::Context) {
        let mut tap_event = None;

        ctx.input(|i| {
            for event in &i.raw.events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: true,
                        modifiers,
                    } => {
                        if *button == egui::PointerButton::Primary && !modifiers.any() {
                            self.mouse_down_pos = Some(*pos);
                            self.mouse_down_time = Some(i.time);
                        }
                    }
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: false,
                        modifiers,
                    } => {
                        if *button == egui::PointerButton::Primary && !modifiers.any() {
                            tap_event = Some((*pos, i.time));
                        }
                    }
                    _ => {}
                }
            }
        });

        if let Some((pos, time)) = tap_event {
            self.handle_pointer_up(ctx, pos, time);
        }

        self.process_drag_or_hold(ctx);
    }

    fn process_drag_or_hold(&mut self, ctx: &egui::Context) {
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
        let is_primary_down = ctx.input(|i| i.pointer.primary_down());
        let wants_pointer = ctx.wants_pointer_input();

        if is_primary_down && !wants_pointer {
            if let Some(pos) = mouse_pos {
                if let Some(mouse_down_pos) = self.mouse_down_pos {
                    if !self.is_drag_mode {
                        let drag_distance = (pos - mouse_down_pos).length();
                        let drag_threshold = 10.0;

                        if drag_distance > drag_threshold {
                            self.is_drag_mode = true;
                            if let Some(vis_rect) = self.visualization_rect {
                                let center_x = vis_rect.center().x;
                                let drag_view = if mouse_down_pos.x < center_x {
                                    DragView::Left
                                } else {
                                    DragView::Right
                                };
                                self.drag_view = Some(drag_view);
                                if self.active_view == ActiveView::Main {
                                    self.toy_manager
                                        .active_toy_mut()
                                        .handle_drag_start(drag_view);
                                }
                            }
                        }
                    }

                    if self.is_drag_mode {
                        self.process_drag(pos);
                    } else {
                        self.process_hold(pos);
                    }
                }
            }
        } else {
            self.clear_drag_state();
        }
    }

    fn process_drag(&mut self, pos: egui::Pos2) {
        if let Some(last_pos) = self.last_drag_pos {
            match self.active_view {
                ActiveView::Compass => {
                    let delta = pos - last_pos;
                    let yaw_rot =
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), delta.x * 0.005);
                    let pitch_rot =
                        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * 0.005);
                    let incremental = pitch_rot * yaw_rot;
                    self.compass_rotation = incremental * self.compass_rotation;
                }
                ActiveView::Map => {}
                ActiveView::Main => {
                    let is_left_view = matches!(self.drag_view, Some(DragView::Left));
                    self.toy_manager
                        .active_toy_mut()
                        .handle_drag(is_left_view, last_pos, pos);
                }
            }
        }
        self.last_drag_pos = Some(pos);
    }

    fn process_hold(&mut self, pos: egui::Pos2) {
        if self.active_view != ActiveView::Main {
            return;
        }

        let vis_rect = self.visualization_rect;

        if let Some(visualization_rect) = vis_rect {
            if visualization_rect.contains(pos) {
                let left_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(true);
                let right_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(false);

                if let Some(analysis) = analyze_tap_in_stereo_view_with_modes(
                    visualization_rect,
                    pos,
                    left_zone_mode,
                    right_zone_mode,
                ) {
                    self.toy_manager.active_toy_mut().handle_hold(&analysis);
                }
            }
        }
    }

    fn clear_drag_state(&mut self) {
        self.is_drag_mode = false;
        self.drag_view = None;
        self.last_drag_pos = None;
        if self.active_view == ActiveView::Main {
            self.toy_manager.active_toy_mut().clear_interaction_state();
        }
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            self.visualization_rect = Some(rect);
            self.toy_manager
                .active_toy_mut()
                .set_stereo_settings(&self.settings.stereo);
            self.toy_manager
                .active_toy_mut()
                .set_four_d_settings(&self.settings.four_d);

            match self.active_view {
                ActiveView::Main => {
                    self.toy_manager.active_toy_mut().render_scene(
                        ui,
                        rect,
                        self.settings.show_debug,
                    );
                }
                ActiveView::Compass => {
                    self.render_compass_scene(ui, rect);
                }
                ActiveView::Map => {
                    self.render_map_scene(ui, rect);
                }
            }

            let (left_rect, right_rect) = split_stereo_views(rect);
            let left_painter = ui.painter().with_clip_rect(left_rect);
            let right_painter = ui.painter().with_clip_rect(right_rect);

            let left_menu_rect = egui::Rect {
                min: left_rect.min,
                max: egui::pos2(left_rect.max.x, left_rect.min.y + 30.0),
            };
            let right_menu_rect = egui::Rect {
                min: right_rect.min,
                max: egui::pos2(right_rect.max.x, right_rect.min.y + 30.0),
            };

            render_common_menu_half(&left_painter, left_menu_rect);
            let compass_label = if self.active_view == ActiveView::Compass {
                "Close"
            } else {
                "Compass"
            };
            render_tap_zone_label(&left_painter, left_rect, Zone::West, compass_label, None);

            let map_label = if self.active_view == ActiveView::Map {
                "Close"
            } else {
                "Map"
            };
            render_tap_zone_label(&left_painter, left_rect, Zone::SouthWest, map_label, None);

            if self.active_view == ActiveView::Compass {
                let frame_label = match self.compass_frame_mode {
                    CompassFrameMode::World => "Frame: World",
                    CompassFrameMode::Camera => "Frame: Camera",
                };
                render_tap_zone_label(&left_painter, left_rect, Zone::South, frame_label, None);
                render_tap_zone_label(&right_painter, right_rect, Zone::South, "Prev", None);
                render_tap_zone_label(&right_painter, right_rect, Zone::SouthEast, "Next", None);
            }

            if self.active_view == ActiveView::Main {
                self.toy_manager
                    .active_toy()
                    .render_toy_menu(&right_painter, right_menu_rect);
            }

            if self.settings.show_debug {
                let options = ZoneDebugOptions::default();
                let left_mode = if self.active_view != ActiveView::Main {
                    ZoneMode::NineZones
                } else {
                    self.toy_manager.active_toy().zone_mode_for_view(true)
                };
                let right_mode = if self.active_view != ActiveView::Main {
                    ZoneMode::NineZones
                } else {
                    self.toy_manager.active_toy().zone_mode_for_view(false)
                };
                render_zone_debug_overlay(&left_painter, left_rect, left_mode, &options);
                render_zone_debug_overlay(&right_painter, right_rect, right_mode, &options);
            }

            if self.menu_open {
                self.render_menu_overlay(ui, rect);
            }
        });
    }

    fn render_menu_overlay(&mut self, ui: &mut egui::Ui, vis_rect: egui::Rect) {
        let left_rect = egui::Rect {
            min: vis_rect.min,
            max: egui::pos2(vis_rect.center().x, vis_rect.max.y),
        };
        let right_rect = egui::Rect {
            min: egui::pos2(vis_rect.center().x, vis_rect.min.y),
            max: vis_rect.max,
        };

        let mut close_menu = false;

        egui::Area::new(egui::Id::new("left_menu"))
            .fixed_pos(left_rect.min)
            .show(ui.ctx(), |ui| {
                ui.set_width(left_rect.width());
                ui.set_height(left_rect.height());

                egui::Frame {
                    fill: panel_fill(),
                    corner_radius: egui::CornerRadius::ZERO,
                    stroke: egui::Stroke::NONE,
                    inner_margin: egui::Margin::same(12),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("X").on_hover_text("Close menu").clicked() {
                            close_menu = true;
                        }
                        ui.heading("FourDeers");
                    });
                    ui.separator();

                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.draw_common_controls(ui);
                        });
                });
            });

        egui::Area::new(egui::Id::new("right_menu"))
            .fixed_pos(right_rect.min)
            .show(ui.ctx(), |ui| {
                ui.set_width(right_rect.width());
                ui.set_height(right_rect.height());

                egui::Frame {
                    fill: panel_fill(),
                    corner_radius: egui::CornerRadius::ZERO,
                    stroke: egui::Stroke::NONE,
                    inner_margin: egui::Margin::same(12),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.toy_manager.active_toy_mut().render_sidebar(ui);
                        });
                });
            });

        if close_menu {
            self.menu_open = false;
        }
    }

    fn draw_common_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Select Toy:");
        let toy_list: Vec<_> = self.toy_manager.toy_list();
        let active_id = self.toy_manager.active_toy_id().to_string();
        let mut switch_to_id: Option<String> = None;
        egui::ComboBox::from_label("")
            .selected_text(self.toy_manager.active_toy_name())
            .show_ui(ui, |ui| {
                for (id, name) in &toy_list {
                    let is_selected = *id == active_id;
                    if ui.selectable_label(is_selected, *name).clicked() {
                        switch_to_id = Some(id.to_string());
                    }
                }
            });

        if let Some(id) = switch_to_id {
            self.toy_manager.switch_to(&id);
            self.compass_waypoint_index = 0;
        }

        if ui.button("Reset").clicked() {
            self.toy_manager.reset_active();
        }

        ui.collapsing("Debug Settings", |ui| {
            ui.checkbox(&mut self.settings.show_debug, "Show Debug Overlay");
        });

        ui.collapsing("4D Settings", |ui| {
            ui.add(
                egui::Slider::new(&mut self.settings.four_d.w_thickness, 0.1..=5.0)
                    .text("W Thickness"),
            );
            ui.label("Controls the range of W dimension visible in the slice");

            ui.add(
                egui::Slider::new(&mut self.settings.four_d.w_color_intensity, 0.0..=1.0)
                    .text("W Color Intensity"),
            );
            ui.label("Controls how strongly the W dimension affects edge coloring");
        });

        ui.collapsing("Stereoscopic Settings", |ui| {
            ui.add(
                egui::Slider::new(&mut self.settings.stereo.eye_separation, 0.0..=1.0)
                    .text("Eye Separation"),
            );
            ui.add(
                egui::Slider::new(&mut self.settings.stereo.projection_distance, 1.0..=10.0)
                    .text("Projection Distance"),
            );
        });

        let commit_hash = env!("GIT_COMMIT_HASH");
        let build_time = env!("BUILD_TIME");
        let short_hash = &commit_hash[..commit_hash.len().min(8)];
        ui.label(
            egui::RichText::new(format!("Commit: {}", short_hash))
                .size(12.0)
                .color(egui::Color32::GRAY),
        );
        ui.label(
            egui::RichText::new(format!("Built: {}", build_time))
                .size(12.0)
                .color(egui::Color32::GRAY),
        );
    }

    fn handle_pointer_up(&mut self, ctx: &egui::Context, pos: egui::Pos2, time: f64) {
        let is_tap = match (self.mouse_down_pos, self.mouse_down_time) {
            (Some(down_pos), Some(down_time)) => {
                let movement_distance = down_pos.distance(pos);
                let time_elapsed = time - down_time;
                movement_distance < 10.0 && time_elapsed < 0.3
            }
            _ => false,
        };

        self.mouse_down_pos = None;
        self.mouse_down_time = None;

        if is_tap && !ctx.wants_pointer_input() {
            if let Some(last_tap) = self.last_tap_time {
                if time - last_tap < 0.15 {
                    return;
                }
            }
            self.last_tap_time = Some(time);
            self.handle_tap_zone(pos);
        }
    }

    fn handle_tap_zone(&mut self, pos: egui::Pos2) {
        let vis_rect = self.visualization_rect;

        let Some(visualization_rect) = vis_rect else {
            return;
        };

        if !visualization_rect.contains(pos) {
            return;
        }

        let (left_rect, _) = split_stereo_views(visualization_rect);
        if left_rect.contains(pos)
            && get_zone_from_rect(left_rect, pos, ZoneMode::NineZones) == Some(Zone::NorthWest)
        {
            self.menu_open = !self.menu_open;
            return;
        }

        if left_rect.contains(pos)
            && get_zone_from_rect(left_rect, pos, ZoneMode::NineZones) == Some(Zone::West)
        {
            self.active_view = if self.active_view == ActiveView::Compass {
                ActiveView::Main
            } else {
                ActiveView::Compass
            };
            return;
        }

        if left_rect.contains(pos)
            && get_zone_from_rect(left_rect, pos, ZoneMode::NineZones) == Some(Zone::SouthWest)
        {
            self.active_view = if self.active_view == ActiveView::Map {
                ActiveView::Main
            } else {
                ActiveView::Map
            };
            return;
        }

        if self.active_view == ActiveView::Compass {
            let (_, right_rect) = split_stereo_views(visualization_rect);
            if right_rect.contains(pos) {
                let zone = get_zone_from_rect(right_rect, pos, ZoneMode::NineZones);
                if zone == Some(Zone::South) {
                    self.cycle_compass_waypoint(-1);
                    return;
                }
                if zone == Some(Zone::SouthEast) {
                    self.cycle_compass_waypoint(1);
                    return;
                }
            }
            return;
        }

        if self.active_view != ActiveView::Main {
            return;
        }

        let left_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(true);
        let right_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(false);

        let Some(analysis) = analyze_tap_in_stereo_view_with_modes(
            visualization_rect,
            pos,
            left_zone_mode,
            right_zone_mode,
        ) else {
            return;
        };

        self.toy_manager.active_toy_mut().handle_tap(&analysis);
    }
}
