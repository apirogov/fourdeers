//! Main application

use eframe::egui;

use crate::input::{analyze_tap_in_stereo_view, zone_to_action, CameraAction};
use crate::state::{AppState, DragView, TetraId};
use crate::tetrahedron::get_tetrahedron_layout;
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
                            if let Some(vis_rect) = self.state.visualization_rect {
                                let center_x = vis_rect.center().x;
                                self.state.drag_view = if mouse_down_pos.x < center_x {
                                    Some(DragView::Left)
                                } else {
                                    Some(DragView::Right)
                                };
                            }
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

    fn get_tetrahedron_center(view_rect: egui::Rect, zone: crate::input::Zone) -> (f32, f32) {
        let layout = get_tetrahedron_layout(view_rect);
        match zone {
            crate::input::Zone::North => {
                (view_rect.center().x, view_rect.min.y + layout.edge_offset)
            }
            crate::input::Zone::South => {
                (view_rect.center().x, view_rect.max.y - layout.edge_offset)
            }
            crate::input::Zone::West => {
                (view_rect.min.x + layout.edge_offset, view_rect.center().y)
            }
            crate::input::Zone::East => {
                (view_rect.max.x - layout.edge_offset, view_rect.center().y)
            }
        }
    }

    fn is_mouse_over_tetrahedron(
        pos: egui::Pos2,
        view_rect: egui::Rect,
        zone: crate::input::Zone,
    ) -> bool {
        let (center_x, center_y) = Self::get_tetrahedron_center(view_rect, zone);
        let layout = get_tetrahedron_layout(view_rect);
        let hit_radius = layout.scale * 1.5;
        let dx = pos.x - center_x;
        let dy = pos.y - center_y;
        (dx * dx + dy * dy) <= hit_radius * hit_radius
    }

    fn process_drag(&mut self, pos: egui::Pos2) {
        use nalgebra::{UnitQuaternion, Vector3};

        if let Some(tetra_id) = self.state.dragging_tetrahedron {
            if let Some(last_pos) = self.state.last_tetra_drag_pos {
                let delta = pos - last_pos;
                let current_rot = self.state.get_tetrahedron_rotation(tetra_id);

                let yaw_rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -delta.x * 0.005);
                let pitch_rot =
                    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), delta.y * 0.005);

                let incremental = pitch_rot * yaw_rot;
                let new_rot = incremental * current_rot;

                self.state.set_tetrahedron_rotation(tetra_id, new_rot);
            }
            self.state.last_tetra_drag_pos = Some(pos);
            self.state.held_action = None;
            self.state.is_dragging = true;
            return;
        }

        // Regular camera drag
        if let Some(last_pos) = self.state.last_mouse_pos {
            let delta = pos - last_pos;

            // Check if we're near a tetrahedron zone and should rotate it instead
            if let Some(visualization_rect) = self.state.visualization_rect {
                if visualization_rect.contains(last_pos) {
                    if let Some(analysis) = analyze_tap_in_stereo_view(visualization_rect, last_pos)
                    {
                        if Self::is_mouse_over_tetrahedron(
                            last_pos,
                            analysis.view_rect,
                            analysis.zone,
                        ) {
                            let tetra_id = TetraId {
                                is_left_view: analysis.is_left_view,
                                zone: analysis.zone,
                            };
                            self.state.dragging_tetrahedron = Some(tetra_id);
                            self.state.last_tetra_drag_pos = Some(pos);
                            self.state.is_dragging = true;
                            return;
                        }
                    }
                }
            }

            // Camera rotation
            match self.state.drag_view {
                Some(DragView::Left) => {
                    self.state.camera.rotate(delta.x, delta.y);
                    self.state.reset_tetrahedron_rotations();
                }
                Some(DragView::Right) => {
                    self.state.camera.rotate_4d(delta.x, delta.y);
                    self.state.reset_tetrahedron_rotations();
                }
                None => {}
            }
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
        self.state.drag_view = None;
        self.state.last_mouse_pos = None;
        self.state.held_action = None;
        self.state.is_drag_mode = false;
        self.state.dragging_tetrahedron = None;
        self.state.last_tetra_drag_pos = None;
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
                self.apply_camera_action(CameraAction::MoveSliceOrthogonalPos, move_speed);
            }
            if i.key_down(egui::Key::Comma) {
                self.apply_camera_action(CameraAction::MoveSliceOrthogonalNeg, move_speed);
            }
        });
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        if self.sidebar_open {
            eframe::egui::SidePanel::left("controls")
                .default_width(280.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        draw_controls(ui, &mut self.state, || self.sidebar_open = false);
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
        // Reset tetrahedron rotations when camera moves
        self.state.reset_tetrahedron_rotations();

        let forward = self.state.camera.forward_vector();
        let right = self.state.camera.right_vector();
        let up = self.state.camera.up_vector();
        let basis_4d = self.state.camera.rotation_4d.basis_vectors();

        let project_3d_to_4d = |v3: (f32, f32, f32)| -> [f32; 4] {
            [
                v3.0 * basis_4d[0][0] + v3.1 * basis_4d[1][0] + v3.2 * basis_4d[2][0],
                v3.0 * basis_4d[0][1] + v3.1 * basis_4d[1][1] + v3.2 * basis_4d[2][1],
                v3.0 * basis_4d[0][2] + v3.1 * basis_4d[1][2] + v3.2 * basis_4d[2][2],
                v3.0 * basis_4d[0][3] + v3.1 * basis_4d[1][3] + v3.2 * basis_4d[2][3],
            ]
        };

        match action {
            CameraAction::MoveForward => {
                let v4 = project_3d_to_4d(forward);
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::MoveBackward => {
                let v4 = project_3d_to_4d(forward);
                self.state.camera.x -= v4[0] * speed;
                self.state.camera.y -= v4[1] * speed;
                self.state.camera.z -= v4[2] * speed;
                self.state.camera.w -= v4[3] * speed;
            }
            CameraAction::StrafeLeft => {
                let v4 = project_3d_to_4d((-right.0, -right.1, -right.2));
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::StrafeRight => {
                let v4 = project_3d_to_4d(right);
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::MoveUp => {
                let v4 = project_3d_to_4d(up);
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::MoveDown => {
                let v4 = project_3d_to_4d((-up.0, -up.1, -up.2));
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::IncreaseW => self.state.camera.w += speed,
            CameraAction::DecreaseW => self.state.camera.w -= speed,
            CameraAction::MoveSliceForward => {
                let v4 = project_3d_to_4d(forward);
                self.state.camera.x += v4[0] * speed;
                self.state.camera.y += v4[1] * speed;
                self.state.camera.z += v4[2] * speed;
                self.state.camera.w += v4[3] * speed;
            }
            CameraAction::MoveSliceBackward => {
                let v4 = project_3d_to_4d(forward);
                self.state.camera.x -= v4[0] * speed;
                self.state.camera.y -= v4[1] * speed;
                self.state.camera.z -= v4[2] * speed;
                self.state.camera.w -= v4[3] * speed;
            }
            CameraAction::MoveSliceOrthogonalPos => {
                let w_dir = basis_4d[3];
                self.state.camera.x += w_dir[0] * speed;
                self.state.camera.y += w_dir[1] * speed;
                self.state.camera.z += w_dir[2] * speed;
                self.state.camera.w += w_dir[3] * speed;
            }
            CameraAction::MoveSliceOrthogonalNeg => {
                let w_dir = basis_4d[3];
                self.state.camera.x -= w_dir[0] * speed;
                self.state.camera.y -= w_dir[1] * speed;
                self.state.camera.z -= w_dir[2] * speed;
                self.state.camera.w -= w_dir[3] * speed;
            }
        }
    }
}
