//! Main application

use eframe::egui;

use crate::input::{analyze_tap_in_stereo_view, zone_to_action, CameraAction};
use crate::state::AppState;
use crate::ui::{draw_controls, render_visualization};

pub struct FourDeersApp {
    state: AppState,
    sidebar_open: bool,
    last_tap_time: Option<f64>,
    mouse_down_pos: Option<egui::Pos2>,
    mouse_down_time: Option<f64>,
}

impl FourDeersApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(),
            sidebar_open: true,
            last_tap_time: None,
            mouse_down_pos: None,
            mouse_down_time: None,
        }
    }
}

impl eframe::App for FourDeersApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.process_pointer_events(ctx);
        self.process_keyboard_input(ctx);
        self.render_ui(ctx);
        ctx.request_repaint();
    }
}

impl FourDeersApp {
    fn process_pointer_events(&mut self, ctx: &egui::Context) {
        let mut tap_event = None;
        let mut tap_pixels_per_point = 1.0;

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
                            tap_pixels_per_point = i.pixels_per_point();
                        }
                    }
                    _ => {}
                }
            }
        });

        if let Some((pos, time)) = tap_event {
            self.handle_pointer_up(ctx, pos, time, tap_pixels_per_point);
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
                    if !self.state.is_drag_mode {
                        let drag_distance = (pos - mouse_down_pos).length();
                        let drag_threshold = 10.0;

                        if drag_distance > drag_threshold {
                            self.state.is_drag_mode = true;
                        }
                    }

                    if self.state.is_drag_mode {
                        self.process_drag(pos);
                    } else {
                        self.process_hold(pos);
                    }
                }
                self.state.last_mouse_pos = Some(pos);
            } else {
                self.clear_drag_state();
            }
        } else {
            self.clear_drag_state();
        }
    }

    fn process_drag(&mut self, pos: egui::Pos2) {
        if let Some(last_pos) = self.state.last_mouse_pos {
            let delta = pos - last_pos;
            self.state.camera.rotate(delta.x, delta.y);
        }
        self.state.held_action = None;
        self.state.is_dragging = true;
    }

    fn process_hold(&mut self, pos: egui::Pos2) {
        if let Some(visualization_rect) = self.state.visualization_rect {
            if visualization_rect.contains(pos) {
                if let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, pos) {
                    let action = zone_to_action(analysis.zone, analysis.is_left_view);

                    self.state.last_tap_pos = Some(pos);
                    self.state.last_tap_zone = Some(analysis.zone);
                    self.state.last_tap_view_left = analysis.is_left_view;

                    self.apply_camera_action(action, 0.08);
                    self.state.held_action = Some(action);
                }
            }
        }
        self.state.is_dragging = false;
    }

    fn clear_drag_state(&mut self) {
        self.state.is_dragging = false;
        self.state.last_mouse_pos = None;
        self.state.held_action = None;
        self.state.is_drag_mode = false;
    }

    fn process_keyboard_input(&mut self, ctx: &egui::Context) {
        let move_speed = 0.15;

        ctx.input(|i| {
            if i.key_down(egui::Key::ArrowUp) {
                self.apply_camera_action(CameraAction::MoveForward, move_speed);
            }
            if i.key_down(egui::Key::ArrowDown) {
                self.apply_camera_action(CameraAction::MoveBackward, move_speed);
            }
            if i.key_down(egui::Key::ArrowLeft) {
                self.apply_camera_action(CameraAction::StrafeLeft, move_speed);
            }
            if i.key_down(egui::Key::ArrowRight) {
                self.apply_camera_action(CameraAction::StrafeRight, move_speed);
            }
            if i.key_down(egui::Key::PageUp) {
                self.apply_camera_action(CameraAction::MoveUp, move_speed);
            }
            if i.key_down(egui::Key::PageDown) {
                self.apply_camera_action(CameraAction::MoveDown, move_speed);
            }
            if i.key_down(egui::Key::Period) {
                self.apply_camera_action(CameraAction::DecreaseW, move_speed);
            }
            if i.key_down(egui::Key::Comma) {
                self.apply_camera_action(CameraAction::IncreaseW, move_speed);
            }
        });
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        if self.sidebar_open {
            eframe::egui::SidePanel::left("controls")
                .default_width(280.0)
                .resizable(true)
                .show(ctx, |ui| {
                    draw_controls(ui, &mut self.state, || self.sidebar_open = false);
                });
        }

        if !self.sidebar_open {
            eframe::egui::Window::new("☰")
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
            let computed_rect = {
                let screen_rect = ctx.available_rect();
                let mut vis_rect = screen_rect;
                if self.sidebar_open {
                    vis_rect.min.x += 280.0;
                }
                vis_rect
            };
            if (rect.min.x - computed_rect.min.x).abs() > 1.0
                || (rect.min.y - computed_rect.min.y).abs() > 1.0
                || (rect.max.x - computed_rect.max.x).abs() > 1.0
                || (rect.max.y - computed_rect.max.y).abs() > 1.0
            {
                println!(
                    "RECT MISMATCH: computed=[({:.1},{:.1})-({:.1},{:.1})], actual=[({:.1},{:.1})-({:.1},{:.1})]",
                    computed_rect.min.x,
                    computed_rect.min.y,
                    computed_rect.max.x,
                    computed_rect.max.y,
                    rect.min.x,
                    rect.min.y,
                    rect.max.x,
                    rect.max.y
                );
            }
            render_visualization(ui, &mut self.state, rect);
        });
    }

    fn handle_pointer_up(
        &mut self,
        ctx: &egui::Context,
        pos: egui::Pos2,
        time: f64,
        _pixels_per_point: f32,
    ) {
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
            self.handle_tap_zone(ctx, pos);
        }
    }

    fn handle_tap_zone(&mut self, _ctx: &egui::Context, pos: egui::Pos2) {
        let Some(visualization_rect) = self.state.visualization_rect else {
            return;
        };

        if !visualization_rect.contains(pos) {
            return;
        }

        let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, pos) else {
            return;
        };

        println!(
            "TAP DETECTED: sidebar_open={}, pos=({:.1},{:.1}), vis_rect=[({:.1},{:.1})-({:.1},{:.1})], view={}, zone={:?}, norm=({:.2},{:.2}), view_rect=[({:.1},{:.1})-({:.1},{:.1})]",
            self.sidebar_open,
            pos.x,
            pos.y,
            visualization_rect.min.x,
            visualization_rect.min.y,
            visualization_rect.max.x,
            visualization_rect.max.y,
            if analysis.is_left_view { "left" } else { "right" },
            analysis.zone,
            analysis.norm_x,
            analysis.norm_y,
            analysis.view_rect.min.x,
            analysis.view_rect.min.y,
            analysis.view_rect.max.x,
            analysis.view_rect.max.y
        );

        self.state.last_tap_pos = Some(pos);
        self.state.last_tap_zone = Some(analysis.zone);
        self.state.last_tap_view_left = analysis.is_left_view;

        let action = zone_to_action(analysis.zone, analysis.is_left_view);
        self.apply_camera_action(action, 0.15);
    }

    fn apply_camera_action(&mut self, action: CameraAction, speed: f32) {
        let forward = self.state.camera.forward_vector();
        let right = self.state.camera.right_vector();
        let up = self.state.camera.up_vector();

        match action {
            CameraAction::MoveForward => self.state.camera.move_along(forward, speed),
            CameraAction::MoveBackward => self
                .state
                .camera
                .move_along((-forward.0, -forward.1, -forward.2), speed),
            CameraAction::StrafeLeft => self
                .state
                .camera
                .move_along((-right.0, -right.1, -right.2), speed),
            CameraAction::StrafeRight => self.state.camera.move_along(right, speed),
            CameraAction::MoveUp => self.state.camera.move_along(up, speed),
            CameraAction::MoveDown => self.state.camera.move_along((-up.0, -up.1, -up.2), speed),
            CameraAction::IncreaseW => self.state.camera.w += speed,
            CameraAction::DecreaseW => self.state.camera.w -= speed,
        }
    }
}
