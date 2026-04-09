//! UI drawing primitives for stereo visualization

use eframe::egui;

use crate::colors::{LABEL_DEFAULT, OUTLINE_DEFAULT, VIEWPORT_BG};
use crate::input::Zone;

const TAP_LABEL_FONT_SIZE: f32 = 11.0;
const ARROW_HEAD_HALF_WIDTH: f32 = 0.5;
const OUTLINE_OFFSET: f32 = 0.5;

pub fn draw_background(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, VIEWPORT_BG);
}

pub fn draw_center_divider(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(2.0, egui::Color32::DARK_GRAY),
    );
}

pub fn render_tap_zone_label(
    painter: &egui::Painter,
    view_rect: egui::Rect,
    zone: Zone,
    label: &str,
    text_color: Option<egui::Color32>,
) {
    let (label_pos, align) = match zone {
        Zone::NorthWest => (view_rect.min, egui::Align2::LEFT_TOP),
        Zone::NorthEast => (
            egui::Pos2::new(view_rect.max.x, view_rect.min.y),
            egui::Align2::RIGHT_TOP,
        ),
        Zone::SouthWest => (
            egui::Pos2::new(view_rect.min.x, view_rect.max.y),
            egui::Align2::LEFT_BOTTOM,
        ),
        Zone::SouthEast => (view_rect.max, egui::Align2::RIGHT_BOTTOM),
        Zone::North => (
            egui::Pos2::new(view_rect.center().x, view_rect.min.y),
            egui::Align2::CENTER_TOP,
        ),
        Zone::South => (
            egui::Pos2::new(view_rect.center().x, view_rect.max.y),
            egui::Align2::CENTER_BOTTOM,
        ),
        Zone::West => (
            egui::Pos2::new(view_rect.min.x, view_rect.center().y),
            egui::Align2::LEFT_CENTER,
        ),
        Zone::East => (
            egui::Pos2::new(view_rect.max.x, view_rect.center().y),
            egui::Align2::RIGHT_CENTER,
        ),
        Zone::Center => (view_rect.center(), egui::Align2::CENTER_CENTER),
    };

    let font_id = egui::FontId::proportional(TAP_LABEL_FONT_SIZE);
    let outline_color = OUTLINE_DEFAULT;
    let text_color = text_color.unwrap_or(LABEL_DEFAULT);

    painter.text(label_pos, align, label, font_id.clone(), outline_color);
    painter.text(label_pos, align, label, font_id, text_color);
}

pub fn render_common_menu_half(painter: &egui::Painter, rect: egui::Rect) {
    render_tap_zone_label(painter, rect, Zone::NorthWest, "Menu", None);
}

pub fn draw_arrow_head(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: egui::Vec2,
    head_size: f32,
    color: egui::Color32,
) {
    let dir = direction.normalized();
    let perp = egui::Vec2::new(-dir.y, dir.x);
    let arrow_base = tip - dir * head_size;
    let arrow_left = arrow_base + perp * (head_size * ARROW_HEAD_HALF_WIDTH);
    let arrow_right = arrow_base - perp * (head_size * ARROW_HEAD_HALF_WIDTH);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, arrow_left, arrow_right],
        color,
        egui::Stroke::NONE,
    ));
}

pub fn render_outlined_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    text_color: egui::Color32,
    outline_color: egui::Color32,
) {
    painter.text(pos, align, text, font_id.clone(), outline_color);
    painter.text(pos, align, text, font_id, text_color);
}

pub fn render_dual_outlined_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    text_color: egui::Color32,
    outline_color: egui::Color32,
) {
    let offset = egui::Vec2::new(OUTLINE_OFFSET, OUTLINE_OFFSET);
    painter.text(pos + offset, align, text, font_id.clone(), outline_color);
    painter.text(pos - offset, align, text, font_id.clone(), outline_color);
    painter.text(pos, align, text, font_id, text_color);
}
