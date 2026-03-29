//! Main application

use eframe::egui;

use crate::colors::panel_fill;
use crate::input::{analyze_tap_in_stereo_view, DragView, Zone};
use crate::render::{
    render_common_menu_half, split_stereo_views, FourDSettings, ProjectionMode, StereoSettings,
};
use crate::toy::ToyManager;

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
        }
    }
}

impl eframe::App for FourDeersApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
                        let drag_threshold = 10.0;

                        if drag_distance > drag_threshold {
                            self.is_drag_mode = true;
                            if let Some(vis_rect) =
                                self.toy_manager.active_toy().get_visualization_rect()
                            {
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
            let is_left_view = matches!(self.drag_view, Some(DragView::Left));
            self.toy_manager
                .active_toy_mut()
                .handle_drag(is_left_view, last_pos, pos);
        }
        self.last_drag_pos = Some(pos);
    }

    fn process_hold(&mut self, pos: egui::Pos2) {
        let vis_rect = self.toy_manager.active_toy().get_visualization_rect();
        let zone_mode = self.toy_manager.active_toy().get_zone_mode();

        if let Some(visualization_rect) = vis_rect {
            if visualization_rect.contains(pos) {
                if let Some(analysis) =
                    analyze_tap_in_stereo_view(visualization_rect, pos, zone_mode)
                {
                    self.toy_manager.active_toy_mut().handle_hold(&analysis);
                }
            }
        }
    }

    fn clear_drag_state(&mut self) {
        self.is_drag_mode = false;
        self.drag_view = None;
        self.last_drag_pos = None;

        if let Some(t) = self
            .toy_manager
            .active_toy_mut()
            .as_any_mut()
            .downcast_mut::<crate::toys::TesseractToy>()
        {
            t.drag_state.clear();
        }

        if let Some(t) = self
            .toy_manager
            .active_toy_mut()
            .as_any_mut()
            .downcast_mut::<crate::toys::tetrahedron_debug::TetrahedronDebugToy>()
        {
            t.drag_state.clear();
        }
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        let mut visualization_rect = None;

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            visualization_rect = Some(rect);
            self.toy_manager
                .active_toy_mut()
                .render_scene(ui, rect, self.settings.show_debug);

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
            self.toy_manager
                .active_toy()
                .render_toy_menu(&right_painter, right_menu_rect);

            self.toy_manager
                .active_toy_mut()
                .set_stereo_settings(&self.settings.stereo);
            self.toy_manager
                .active_toy_mut()
                .set_four_d_settings(&self.settings.four_d);

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

            ui.separator();
            ui.label("Projection Mode:");
            if ui
                .radio_value(
                    &mut self.settings.stereo.projection_mode,
                    ProjectionMode::Perspective,
                    "Perspective",
                )
                .clicked()
            {
                // Already handled by radio_value
            }
            if ui
                .radio_value(
                    &mut self.settings.stereo.projection_mode,
                    ProjectionMode::Orthographic,
                    "Orthographic",
                )
                .clicked()
            {
                // Already handled by radio_value
            }
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
        let vis_rect = self.toy_manager.active_toy().get_visualization_rect();
        let zone_mode = self.toy_manager.active_toy().get_zone_mode();

        let Some(visualization_rect) = vis_rect else {
            return;
        };

        if !visualization_rect.contains(pos) {
            return;
        }

        let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, pos, zone_mode) else {
            return;
        };

        if analysis.is_left_view && analysis.zone == Zone::NorthWest {
            self.menu_open = !self.menu_open;
            return;
        }

        self.toy_manager.active_toy_mut().handle_tap(&analysis);
    }
}
