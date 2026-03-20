//! UI components and rendering

use eframe::egui;
use nalgebra::Vector3;

use crate::geometry::{apply_so4_rotation, create_tesseract, Vertex4D};
use crate::input::Zone;
use crate::state::AppState;

/// Draw the control panel
pub fn draw_controls(ui: &mut egui::Ui, state: &mut AppState, on_close: impl FnOnce()) {
    ui.horizontal(|ui| {
        ui.heading("🦌 FourDeers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").on_hover_text("Close sidebar").clicked() {
                on_close();
            }
        });
    });
    ui.separator();
    ui.label("3D Slice through 4D Tesseract");
    ui.label("Arrows: Move | PgUp/Dn: Up/Down | ,/. : W-slice");
    ui.label("Mouse: Look");
    ui.separator();

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

    ui.checkbox(&mut state.show_debug, "Show Debug Overlay");

    ui.add_space(8.0);

    ui.heading("Position & Orientation");

    let is_small_screen = ui.available_width() < 250.0;

    if is_small_screen {
        draw_position_controls_vertical(ui, state);
    } else {
        draw_position_controls_horizontal(ui, state);
    }

    draw_orientation_controls(ui, state);

    ui.separator();
    ui.add_space(4.0);

    ui.collapsing("Keyboard Controls", |ui| {
        ui.label("Arrow keys: Move forward/back/strafe");
        ui.label("PageUp/Down: Move up/down");
        ui.label(",/.: W-slice movement");
        ui.label("");
        ui.label("Tap & hold zones for movement");
        ui.label("Drag to rotate camera");
    });

    ui.separator();
    ui.add_space(4.0);

    ui.collapsing("4D Object Rotation", |ui| {
        draw_rotation_controls(ui, state);
    });

    ui.add_space(4.0);

    ui.collapsing("Slice Settings", |ui| {
        ui.add(egui::Slider::new(&mut state.w_thickness, 0.1..=2.0).text("W Thickness"));
    });

    ui.add_space(4.0);

    ui.collapsing("Stereoscopic", |ui| {
        ui.add(egui::Slider::new(&mut state.eye_separation, 0.0..=1.0).text("Eye Separation"));
        ui.add(
            egui::Slider::new(&mut state.projection_distance, 1.0..=10.0)
                .text("Projection Distance"),
        );
    });

    ui.add_space(4.0);

    ui.collapsing("W Coloring", |ui| {
        ui.horizontal(|ui| {
            ui.label("Range:");
            ui.add(egui::DragValue::new(&mut state.w_min).speed(0.1));
            ui.label("to");
            ui.add(egui::DragValue::new(&mut state.w_max).speed(0.1));
        });
    });

    ui.separator();
    ui.label("Geometry: 4D Tesseract");
    ui.label("16 vertices, 32 edges");

    if ui.button("Reset").clicked() {
        state.reset();
    }
}

fn draw_position_controls_vertical(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label("X Position");
        ui.add(egui::Slider::new(&mut state.camera.x, -10.0..=10.0).text(""));
        ui.label("Y Position");
        ui.add(egui::Slider::new(&mut state.camera.y, -10.0..=10.0).text(""));
        ui.label("Z Position");
        ui.add(egui::Slider::new(&mut state.camera.z, -10.0..=10.0).text(""));
        ui.label("W-slice:");
        ui.add(egui::Slider::new(&mut state.camera.w, -3.0..=3.0).text(""));
    });
}

fn draw_position_controls_horizontal(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("X Position");
            ui.add(egui::Slider::new(&mut state.camera.x, -10.0..=10.0).text(""));
        });
        ui.vertical(|ui| {
            ui.label("Y Position");
            ui.add(egui::Slider::new(&mut state.camera.y, -10.0..=10.0).text(""));
        });
    });

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("Z Position");
            ui.add(egui::Slider::new(&mut state.camera.z, -10.0..=10.0).text(""));
        });
        ui.vertical(|ui| {
            ui.label("W-slice:");
            ui.add(egui::Slider::new(&mut state.camera.w, -3.0..=3.0).text(""));
        });
    });
}

fn draw_orientation_controls(ui: &mut egui::Ui, state: &mut AppState) {
    let mut yaw = state.camera.yaw();
    let mut pitch = state.camera.pitch();

    ui.label("Camera Orientation:");
    ui.horizontal(|ui| {
        ui.label("Yaw");
        if ui
            .add(egui::Slider::new(&mut yaw, -std::f32::consts::PI..=std::f32::consts::PI).text(""))
            .changed()
        {
            state.camera.set_yaw_pitch(yaw, pitch);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Pitch");
        if ui
            .add(
                egui::Slider::new(&mut pitch, -std::f32::consts::PI..=std::f32::consts::PI)
                    .text(""),
            )
            .changed()
        {
            state.camera.set_yaw_pitch(yaw, pitch);
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "Position: ({:.2}, {:.2}, {:.2})",
            state.camera.x, state.camera.y, state.camera.z
        ));
        ui.label(format!("W: {:.2}", state.camera.w));
    });
}

fn draw_rotation_controls(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(
                &mut state.rot_xy,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("XY"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.rot_xz,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("XZ"),
        );
    });

    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(
                &mut state.rot_yz,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("YZ"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.rot_xw,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("XW"),
        );
    });

    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(
                &mut state.rot_yw,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("YW"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.rot_zw,
                -std::f32::consts::PI..=std::f32::consts::PI,
            )
            .text("ZW"),
        );
    });
}

/// Render the 3D visualization
pub fn render_visualization(ui: &mut egui::Ui, state: &mut AppState, rect: egui::Rect) {
    draw_background(ui, rect);

    let (left_rect, right_rect) = split_stereo_views(rect);
    state.visualization_rect = Some(rect);

    if state.show_debug {
        draw_debug_rects(ui, rect, left_rect, right_rect);
    }

    draw_center_divider(ui, rect);

    let ctx = prepare_render_context(state);

    render_eye_view(ui, &ctx, left_rect, -1.0);
    render_eye_view(ui, &ctx, right_rect, 1.0);

    if state.show_debug {
        draw_debug_overlay(ui, state, left_rect, right_rect);
    }
}

struct RenderContext<'a> {
    state: &'a AppState,
    vertices: Vec<Vertex4D>,
    indices: Vec<u16>,
    sin_xy: f32,
    cos_xy: f32,
    sin_xz: f32,
    cos_xz: f32,
    sin_yz: f32,
    cos_yz: f32,
    sin_xw: f32,
    cos_xw: f32,
    sin_yw: f32,
    cos_yw: f32,
    sin_zw: f32,
    cos_zw: f32,
    inv_orientation: nalgebra::UnitQuaternion<f32>,
    w_slice_center: f32,
    w_half: f32,
}

fn prepare_render_context(state: &AppState) -> RenderContext<'_> {
    let (vertices, indices) = create_tesseract();

    let (sin_xy, cos_xy) = state.rot_xy.sin_cos();
    let (sin_xz, cos_xz) = state.rot_xz.sin_cos();
    let (sin_yz, cos_yz) = state.rot_yz.sin_cos();
    let (sin_xw, cos_xw) = state.rot_xw.sin_cos();
    let (sin_yw, cos_yw) = state.rot_yw.sin_cos();
    let (sin_zw, cos_zw) = state.rot_zw.sin_cos();

    let inv_orientation = state.camera.orientation.inverse();
    let w_slice_center = state.camera.w;
    let w_half = state.w_thickness * 0.5;

    RenderContext {
        state,
        vertices,
        indices,
        sin_xy,
        cos_xy,
        sin_xz,
        cos_xz,
        sin_yz,
        cos_yz,
        sin_xw,
        cos_xw,
        sin_yw,
        cos_yw,
        sin_zw,
        cos_zw,
        inv_orientation,
        w_slice_center,
        w_half,
    }
}

fn split_stereo_views(rect: egui::Rect) -> (egui::Rect, egui::Rect) {
    let left_rect = egui::Rect {
        min: rect.min,
        max: egui::pos2(rect.center().x, rect.max.y),
    };
    let right_rect = egui::Rect {
        min: egui::pos2(rect.center().x, rect.min.y),
        max: rect.max,
    };
    (left_rect, right_rect)
}

fn draw_background(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 40));
}

fn draw_debug_rects(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    left_rect: egui::Rect,
    right_rect: egui::Rect,
) {
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
        egui::StrokeKind::Inside,
    );

    ui.painter().rect_stroke(
        left_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 255)),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_stroke(
        right_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 255)),
        egui::StrokeKind::Inside,
    );

    let text_color = egui::Color32::from_rgb(200, 200, 100);
    let font = egui::FontId::proportional(12.0);

    ui.painter().text(
        rect.left_top() + egui::Vec2::new(5.0, 15.0),
        egui::Align2::LEFT_TOP,
        format!(
            "vis: ({:.0},{:.0})-({:.0},{:.0})",
            rect.min.x, rect.min.y, rect.max.x, rect.max.y
        ),
        font.clone(),
        text_color,
    );

    ui.painter().text(
        left_rect.left_top() + egui::Vec2::new(5.0, 30.0),
        egui::Align2::LEFT_TOP,
        format!(
            "left: ({:.0},{:.0})-({:.0},{:.0})",
            left_rect.min.x, left_rect.min.y, left_rect.max.x, left_rect.max.y
        ),
        font.clone(),
        text_color,
    );

    ui.painter().text(
        right_rect.left_top() + egui::Vec2::new(5.0, 45.0),
        egui::Align2::LEFT_TOP,
        format!(
            "right: ({:.0},{:.0})-({:.0},{:.0})",
            right_rect.min.x, right_rect.min.y, right_rect.max.x, right_rect.max.y
        ),
        font,
        text_color,
    );
}

fn draw_center_divider(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
    );
}

fn render_eye_view(
    ui: &mut egui::Ui,
    ctx: &RenderContext<'_>,
    view_rect: egui::Rect,
    eye_sign: f32,
) {
    let center = view_rect.center();
    let scale = view_rect.height().min(view_rect.width()) * 0.35;
    let eye_offset = eye_sign * ctx.state.eye_separation * 0.5;

    let painter = ui.painter().with_clip_rect(view_rect);

    render_tesseract_edges(&painter, ctx, center, scale, eye_offset);
    render_axes_widget(&painter, ctx, view_rect);
    render_position_label(&painter, ctx.state, view_rect);
}

fn render_tesseract_edges(
    painter: &egui::Painter,
    ctx: &RenderContext<'_>,
    center: egui::Pos2,
    scale: f32,
    eye_offset: f32,
) {
    for chunk in ctx.indices.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }

        let v0 = &ctx.vertices[chunk[0] as usize];
        let v1 = &ctx.vertices[chunk[1] as usize];

        let p0_4d = apply_so4_rotation(
            v0.position,
            ctx.sin_xy,
            ctx.cos_xy,
            ctx.sin_xz,
            ctx.cos_xz,
            ctx.sin_yz,
            ctx.cos_yz,
            ctx.sin_xw,
            ctx.cos_xw,
            ctx.sin_yw,
            ctx.cos_yw,
            ctx.sin_zw,
            ctx.cos_zw,
        );
        let p1_4d = apply_so4_rotation(
            v1.position,
            ctx.sin_xy,
            ctx.cos_xy,
            ctx.sin_xz,
            ctx.cos_xz,
            ctx.sin_yz,
            ctx.cos_yz,
            ctx.sin_xw,
            ctx.cos_xw,
            ctx.sin_yw,
            ctx.cos_yw,
            ctx.sin_zw,
            ctx.cos_zw,
        );

        let w0_in_slice = p0_4d.w >= ctx.w_slice_center - ctx.w_half
            && p0_4d.w <= ctx.w_slice_center + ctx.w_half;
        let w1_in_slice = p1_4d.w >= ctx.w_slice_center - ctx.w_half
            && p1_4d.w <= ctx.w_slice_center + ctx.w_half;

        if !w0_in_slice && !w1_in_slice {
            continue;
        }

        let (screen_p0, screen_p1) =
            project_edge_points(p0_4d, p1_4d, ctx, center, scale, eye_offset);

        let Some((s0, s1)) = screen_p0.zip(screen_p1) else {
            continue;
        };

        let w_avg = (p0_4d.w + p1_4d.w) / 2.0;
        let t = ((w_avg - ctx.state.w_min) / (ctx.state.w_max - ctx.state.w_min)).clamp(0.0, 1.0);
        let alpha = if w0_in_slice && w1_in_slice { 255 } else { 100 };
        let r = (255.0 * t) as u8;
        let g = (200.0 * (1.0 - t.abs())) as u8;
        let b = (150.0 + 105.0 * t) as u8;
        let color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);

        painter.line_segment([s0, s1], egui::Stroke::new(2.5, color));
    }
}

fn project_edge_points(
    p0_4d: nalgebra::Vector4<f32>,
    p1_4d: nalgebra::Vector4<f32>,
    ctx: &RenderContext<'_>,
    center: egui::Pos2,
    scale: f32,
    eye_offset: f32,
) -> (Option<egui::Pos2>, Option<egui::Pos2>) {
    let p0_rel = Vector3::new(
        p0_4d.x - ctx.state.camera.x,
        p0_4d.y - ctx.state.camera.y,
        p0_4d.z - ctx.state.camera.z,
    );
    let p1_rel = Vector3::new(
        p1_4d.x - ctx.state.camera.x,
        p1_4d.y - ctx.state.camera.y,
        p1_4d.z - ctx.state.camera.z,
    );

    let p0_cam = ctx.inv_orientation.transform_vector(&p0_rel);
    let p1_cam = ctx.inv_orientation.transform_vector(&p1_rel);

    let x0_final = p0_cam.x + eye_offset;
    let x1_final = p1_cam.x + eye_offset;

    let dist = ctx.state.projection_distance;

    let s0 = if p0_cam.z > -dist + 0.1 {
        let scale0 = scale / (p0_cam.z + dist);
        Some(egui::Pos2::new(
            center.x + x0_final * scale0,
            center.y - p0_cam.y * scale0,
        ))
    } else {
        None
    };

    let s1 = if p1_cam.z > -dist + 0.1 {
        let scale1 = scale / (p1_cam.z + dist);
        Some(egui::Pos2::new(
            center.x + x1_final * scale1,
            center.y - p1_cam.y * scale1,
        ))
    } else {
        None
    };

    (s0, s1)
}

fn render_axes_widget(painter: &egui::Painter, ctx: &RenderContext<'_>, view_rect: egui::Rect) {
    let gadget_pos = egui::Pos2::new(view_rect.center().x, view_rect.max.y - 60.0);
    let gadget_scale = 30.0;

    let axes = [
        (
            [1.0f32, 0.0, 0.0],
            egui::Color32::from_rgb(255, 80, 80),
            "X",
        ),
        (
            [0.0f32, 1.0, 0.0],
            egui::Color32::from_rgb(80, 255, 80),
            "Y",
        ),
        (
            [0.0f32, 0.0, 1.0],
            egui::Color32::from_rgb(80, 150, 255),
            "Z",
        ),
    ];

    for (axis, color, label) in axes {
        let world_axis = Vector3::new(axis[0], axis[1], axis[2]);
        let rotated_axis = ctx.inv_orientation.transform_vector(&world_axis);

        let p_end = egui::Pos2::new(
            gadget_pos.x + rotated_axis.x * gadget_scale,
            gadget_pos.y - rotated_axis.y * gadget_scale,
        );

        painter.line_segment([gadget_pos, p_end], egui::Stroke::new(4.0, color));
        painter.text(
            p_end,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            color,
        );
    }
}

fn render_position_label(painter: &egui::Painter, state: &AppState, view_rect: egui::Rect) {
    let pos_label_y = view_rect.max.y - 60.0 + 30.0 + 20.0;
    let pos_label = egui::Pos2::new(view_rect.center().x, pos_label_y);
    painter.text(
        pos_label,
        egui::Align2::CENTER_CENTER,
        format!(
            "X:{:.1} Y:{:.1} Z:{:.1} W:{:.1}",
            state.camera.x, state.camera.y, state.camera.z, state.camera.w
        ),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(255, 200, 100),
    );
}

fn draw_debug_overlay(
    ui: &mut egui::Ui,
    state: &AppState,
    left_rect: egui::Rect,
    right_rect: egui::Rect,
) {
    if let (Some(tap_pos), Some(zone), view_left) = (
        state.last_tap_pos,
        state.last_tap_zone,
        state.last_tap_view_left,
    ) {
        let view_rect = if view_left { left_rect } else { right_rect };

        draw_zone_highlight(ui, view_rect, zone, tap_pos);
    }

    if let Some(viz_rect) = state.visualization_rect {
        ui.painter().rect_stroke(
            viz_rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
            egui::StrokeKind::Outside,
        );
    }
}

fn draw_zone_highlight(ui: &mut egui::Ui, view_rect: egui::Rect, zone: Zone, tap_pos: egui::Pos2) {
    let lt = view_rect.left_top();
    let rt = view_rect.right_top();
    let rb = view_rect.right_bottom();
    let lb = view_rect.left_bottom();
    let center = view_rect.center();

    ui.painter().line_segment(
        [lt, rb],
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_premultiplied(100, 100, 100, 100),
        ),
    );
    ui.painter().line_segment(
        [rt, lb],
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_premultiplied(100, 100, 100, 100),
        ),
    );

    let vertices = match zone {
        Zone::North => vec![lt, rt, center],
        Zone::South => vec![lb, rb, center],
        Zone::West => vec![lt, lb, center],
        Zone::East => vec![rt, rb, center],
    };

    ui.painter().add(egui::Shape::convex_polygon(
        vertices.clone(),
        egui::Color32::from_rgba_unmultiplied(255, 255, 0, 100),
        egui::Stroke::NONE,
    ));
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        ui.painter().line_segment(
            [vertices[i], vertices[j]],
            egui::Stroke::new(4.0, egui::Color32::from_rgb(255, 50, 50)),
        );
    }

    ui.painter()
        .circle_filled(tap_pos, 8.0, egui::Color32::from_rgb(255, 0, 0));

    let norm_x = (tap_pos.x - view_rect.min.x) / view_rect.width();
    let norm_y = (tap_pos.y - view_rect.min.y) / view_rect.height();
    let zone_text = format!(
        "{:?}\nabs: ({:.0},{:.0})\nnorm: ({:.2},{:.2})",
        zone, tap_pos.x, tap_pos.y, norm_x, norm_y
    );
    ui.painter().text(
        tap_pos + egui::Vec2::new(0.0, 25.0),
        egui::Align2::CENTER_CENTER,
        zone_text,
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
}
