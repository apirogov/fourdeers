//! Main application

use eframe::egui;

use crate::input::{analyze_tap_in_stereo_view, DragView};
use crate::toy::ToyManager;

pub struct FourDeersApp {
    toy_manager: ToyManager,
    sidebar_open: bool,
    last_tap_time: Option<f64>,
    mouse_down_pos: Option<egui::Pos2>,
    mouse_down_time: Option<f64>,
    is_drag_mode: bool,
    drag_view: Option<DragView>,
}

impl FourDeersApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            toy_manager: ToyManager::new(),
            sidebar_open: true,
            last_tap_time: None,
            mouse_down_pos: None,
            mouse_down_time: None,
            is_drag_mode: false,
            drag_view: None,
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
                                let toy = self.toy_manager.active_toy_mut();
                                toy.handle_drag_start(drag_view);
                                if let Some(t) =
                                    toy.as_any_mut().downcast_mut::<crate::toys::TesseractToy>()
                                {
                                    t.drag_state.last_mouse_pos = Some(mouse_down_pos);
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
        let last_pos = self
            .toy_manager
            .active_toy()
            .as_any()
            .downcast_ref::<crate::toys::TesseractToy>()
            .and_then(|t| t.drag_state.last_mouse_pos);

        if let Some(last_pos) = last_pos {
            let is_left_view = matches!(self.drag_view, Some(DragView::Left));
            self.toy_manager
                .active_toy_mut()
                .handle_drag(is_left_view, last_pos, pos);
        }

        if let Some(t) = self
            .toy_manager
            .active_toy_mut()
            .as_any_mut()
            .downcast_mut::<crate::toys::TesseractToy>()
        {
            t.drag_state.last_mouse_pos = Some(pos);
            t.drag_state.is_dragging = true;
        }
    }

    fn process_hold(&mut self, pos: egui::Pos2) {
        let vis_rect = self.toy_manager.active_toy().get_visualization_rect();

        if let Some(visualization_rect) = vis_rect {
            if visualization_rect.contains(pos) {
                if let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, pos) {
                    self.toy_manager.active_toy_mut().handle_hold(&analysis);
                }
            }
        }
    }

    fn clear_drag_state(&mut self) {
        self.is_drag_mode = false;
        self.drag_view = None;

        if let Some(t) = self
            .toy_manager
            .active_toy_mut()
            .as_any_mut()
            .downcast_mut::<crate::toys::TesseractToy>()
        {
            t.drag_state.clear();
        }
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        if self.sidebar_open {
            eframe::egui::SidePanel::left("controls")
                .default_width(280.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.draw_common_controls(ui);
                        ui.separator();
                        self.toy_manager.active_toy_mut().render_sidebar(ui);
                    });
                });
        }

        if !self.sidebar_open {
            eframe::egui::Window::new("Menu")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .fixed_pos([10.0, 10.0])
                .default_size([120.0, 40.0])
                .show(ctx, |ui| {
                    if ui.button("Controls").clicked() {
                        self.sidebar_open = true;
                    }
                });
        }

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            self.toy_manager
                .active_toy_mut()
                .render_scene(ui, rect, false);
        });
    }

    fn draw_common_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("FourDeers");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("X").on_hover_text("Close sidebar").clicked() {
                    self.sidebar_open = false;
                }
            });
        });
        ui.separator();

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

        let Some(visualization_rect) = vis_rect else {
            return;
        };

        if !visualization_rect.contains(pos) {
            return;
        }

        let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, pos) else {
            return;
        };

        self.toy_manager.active_toy_mut().handle_tap(&analysis);
    }
}
