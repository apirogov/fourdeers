//! Main application

use eframe::egui;

use crate::colors::PANEL_FILL;
use crate::gpu::GpuRenderer;
use crate::input::render_zone_debug_overlay;
use crate::input::{analyze_pointer_initial, DragView, PointerAnalysis, ZoneDebugOptions};
use crate::render::{
    render_common_menu_half, split_stereo_views, FourDSettings, StereoSettings, W_THICKNESS_MAX,
    W_THICKNESS_MIN,
};
use crate::toy::{ToyManager, ViewAction};

const DRAG_THRESHOLD: f32 = 10.0;
const TAP_MAX_DISTANCE: f32 = 10.0;
const TAP_MAX_TIME: f64 = 0.3;
const DOUBLE_TAP_SUPPRESSION_TIME: f64 = 0.15;
const MENU_BAR_HEIGHT: f32 = 30.0;
const MENU_INNER_MARGIN: i8 = 12;
const BUILD_INFO_FONT_SIZE: f32 = 12.0;
const BUILD_INFO_COLOR: egui::Color32 = egui::Color32::GRAY;

#[derive(Debug, Clone, Copy, Default)]
pub struct CommonSettings {
    pub show_debug: bool,
    pub four_d: FourDSettings,
    pub stereo: StereoSettings,
}

pub struct FourDeersApp {
    toy_manager: ToyManager,
    gpu_renderer: Option<GpuRenderer>,
    menu_open: bool,
    settings: CommonSettings,
    last_tap_time: Option<f64>,
    mouse_down_pos: Option<egui::Pos2>,
    mouse_down_time: Option<f64>,
    is_drag_mode: bool,
    drag_view: Option<DragView>,
    last_drag_pos: Option<egui::Pos2>,
    visualization_rect: Option<egui::Rect>,
}

impl FourDeersApp {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gpu_renderer = cc.wgpu_render_state.as_ref().map(GpuRenderer::new);
        Self {
            toy_manager: ToyManager::new(),
            gpu_renderer,
            menu_open: false,
            settings: CommonSettings::default(),
            last_tap_time: None,
            mouse_down_pos: None,
            mouse_down_time: None,
            is_drag_mode: false,
            drag_view: None,
            last_drag_pos: None,
            visualization_rect: None,
        }
    }
}

impl eframe::App for FourDeersApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::M) {
                self.menu_open = !self.menu_open;
            }
        });

        self.process_pointer_events(ctx);
        self.toy_manager.active_toy_mut().handle_keyboard(ctx);
        self.render_ui(ctx);
        ctx.request_repaint();
    }
}

impl FourDeersApp {
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

                        if drag_distance > DRAG_THRESHOLD {
                            self.is_drag_mode = true;
                            self.last_drag_pos = Some(pos); // Initialize last_pos!
                            if let Some(vis_rect) = self.visualization_rect {
                                let center_x = vis_rect.center().x;
                                let drag_view = if mouse_down_pos.x < center_x {
                                    DragView::Left
                                } else {
                                    DragView::Right
                                };
                                self.drag_view = Some(drag_view);
                                self.toy_manager
                                    .active_toy_mut()
                                    .handle_drag_start(drag_view);
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
            let delta = pos - last_pos;

            let analysis = PointerAnalysis {
                is_left_view: matches!(self.drag_view, Some(DragView::Left)),
                norm_pos: egui::vec2(0.0, 0.0),
                zone: None,
                drag_delta: delta,
                drag_view: self.drag_view,
                is_hold: false,
                is_drag: true,
            };

            self.toy_manager
                .active_toy_mut()
                .handle_drag(analysis, &mut self.settings.four_d.w_thickness);
        }
        self.last_drag_pos = Some(pos);
    }

    fn process_hold(&mut self, pos: egui::Pos2) {
        let Some(vis_rect) = self.visualization_rect else {
            return;
        };
        if !vis_rect.contains(pos) {
            return;
        }

        let left_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(true);
        let right_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(false);
        if let Some(mut analysis) =
            analyze_pointer_initial(vis_rect, pos, left_zone_mode, right_zone_mode)
        {
            analysis.is_hold = true;
            self.toy_manager.active_toy_mut().handle_pointer(analysis);
        }
    }

    fn clear_drag_state(&mut self) {
        self.is_drag_mode = false;
        self.drag_view = None;
        self.last_drag_pos = None;
        self.toy_manager.active_toy_mut().clear_interaction_state();
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

            self.toy_manager.active_toy_mut().render_scene(
                ui,
                rect,
                self.settings.show_debug,
                self.gpu_renderer.as_ref(),
            );

            let (left_rect, right_rect) = split_stereo_views(rect);
            let left_painter = ui.painter().with_clip_rect(left_rect);
            let right_painter = ui.painter().with_clip_rect(right_rect);

            let left_menu_rect = egui::Rect {
                min: left_rect.min,
                max: egui::pos2(left_rect.max.x, left_rect.min.y + MENU_BAR_HEIGHT),
            };

            render_common_menu_half(&left_painter, left_menu_rect);

            self.toy_manager.active_toy().render_view_overlays(
                &left_painter,
                left_rect,
                &right_painter,
                right_rect,
            );

            if self.settings.show_debug {
                let options = ZoneDebugOptions::default();
                let left_mode = self.toy_manager.active_toy().zone_mode_for_view(true);
                let right_mode = self.toy_manager.active_toy().zone_mode_for_view(false);
                render_zone_debug_overlay(&left_painter, left_rect, left_mode, &options);
                render_zone_debug_overlay(&right_painter, right_rect, right_mode, &options);
            }

            if self.menu_open {
                self.render_menu_overlay(ui, rect);
            }
        });
    }

    fn render_menu_overlay(&mut self, ui: &mut egui::Ui, vis_rect: egui::Rect) {
        let (left_rect, right_rect) = crate::render::split_stereo_views(vis_rect);

        let mut close_menu = false;

        let panel_frame = egui::Frame {
            fill: PANEL_FILL,
            corner_radius: egui::CornerRadius::ZERO,
            stroke: egui::Stroke::NONE,
            inner_margin: egui::Margin::same(MENU_INNER_MARGIN),
            ..Default::default()
        };

        egui::Area::new(egui::Id::new("left_menu"))
            .fixed_pos(left_rect.min)
            .show(ui.ctx(), |ui| {
                ui.set_width(left_rect.width());
                ui.set_height(left_rect.height());

                panel_frame.show(ui, |ui| {
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

                panel_frame.show(ui, |ui| {
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
        }

        if ui.button("Reset").clicked() {
            self.toy_manager.reset_active();
        }

        ui.collapsing("Debug Settings", |ui| {
            ui.checkbox(&mut self.settings.show_debug, "Show Debug Overlay");
        });

        ui.collapsing("4D Settings", |ui| {
            ui.add(
                egui::Slider::new(
                    &mut self.settings.four_d.w_thickness,
                    W_THICKNESS_MIN..=W_THICKNESS_MAX,
                )
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
            egui::RichText::new(format!("Commit: {short_hash}"))
                .size(BUILD_INFO_FONT_SIZE)
                .color(BUILD_INFO_COLOR),
        );
        ui.label(
            egui::RichText::new(format!("Built: {build_time}"))
                .size(BUILD_INFO_FONT_SIZE)
                .color(BUILD_INFO_COLOR),
        );
    }

    fn handle_pointer_up(&mut self, ctx: &egui::Context, pos: egui::Pos2, time: f64) {
        let is_tap = match (self.mouse_down_pos, self.mouse_down_time) {
            (Some(down_pos), Some(down_time)) => {
                let movement_distance = down_pos.distance(pos);
                let time_elapsed = time - down_time;
                movement_distance < TAP_MAX_DISTANCE && time_elapsed < TAP_MAX_TIME
            }
            _ => false,
        };

        self.mouse_down_pos = None;
        self.mouse_down_time = None;

        if is_tap && !ctx.wants_pointer_input() {
            if let Some(last_tap) = self.last_tap_time {
                if time - last_tap < DOUBLE_TAP_SUPPRESSION_TIME {
                    return;
                }
            }
            self.last_tap_time = Some(time);
            self.handle_tap_zone(pos);
        }
    }

    fn handle_tap_zone(&mut self, pos: egui::Pos2) {
        let Some(visualization_rect) = self.visualization_rect else {
            return;
        };

        if !visualization_rect.contains(pos) {
            return;
        }

        let left_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(true);
        let right_zone_mode = self.toy_manager.active_toy().zone_mode_for_view(false);
        let Some(analysis) =
            analyze_pointer_initial(visualization_rect, pos, left_zone_mode, right_zone_mode)
        else {
            return;
        };

        // Check for menu toggle
        if analysis.is_left_view {
            if let Some(zone) = analysis.zone {
                if zone == crate::input::Zone::NorthWest {
                    self.menu_open = !self.menu_open;
                    return;
                }
            }
        }

        let action = self.toy_manager.active_toy_mut().handle_pointer(analysis);
        if let ViewAction::ToggleMenu = action {
            self.menu_open = !self.menu_open;
        }
    }
}
