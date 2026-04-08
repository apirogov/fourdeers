//! Debug rendering helpers for tap zones

use eframe::egui;

use super::zones::{Zone, ZoneMode};
use crate::colors::{DEBUG_BOUNDARY, DEBUG_LABEL};

#[derive(Debug, Clone)]
pub struct ZoneDebugOptions {
    pub show_boundaries: bool,
    pub show_labels: bool,
    pub boundary_color: egui::Color32,
    pub label_color: egui::Color32,
    pub label_font_size: f32,
}

impl Default for ZoneDebugOptions {
    fn default() -> Self {
        Self {
            show_boundaries: true,
            show_labels: true,
            boundary_color: DEBUG_BOUNDARY,
            label_color: DEBUG_LABEL,
            label_font_size: 10.0,
        }
    }
}

impl ZoneDebugOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn show_boundaries(mut self, show: bool) -> Self {
        self.show_boundaries = show;
        self
    }

    #[must_use]
    pub const fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    #[must_use]
    pub const fn boundary_color(mut self, color: egui::Color32) -> Self {
        self.boundary_color = color;
        self
    }

    #[must_use]
    pub const fn label_color(mut self, color: egui::Color32) -> Self {
        self.label_color = color;
        self
    }

    #[must_use]
    pub const fn label_font_size(mut self, size: f32) -> Self {
        self.label_font_size = size;
        self
    }
}

pub fn render_zone_debug_overlay(
    painter: &egui::Painter,
    view_rect: egui::Rect,
    zone_mode: ZoneMode,
    options: &ZoneDebugOptions,
) {
    if options.show_boundaries {
        render_zone_boundaries(painter, view_rect, zone_mode, options.boundary_color);
    }

    if options.show_labels {
        render_zone_labels(painter, view_rect, zone_mode, options);
    }
}

fn render_zone_boundaries(
    painter: &egui::Painter,
    view_rect: egui::Rect,
    zone_mode: ZoneMode,
    color: egui::Color32,
) {
    match zone_mode {
        ZoneMode::FourZones => render_4zone_boundaries(painter, view_rect, color),
        ZoneMode::NineZones => render_9zone_boundaries(painter, view_rect, color),
    }
}

fn render_4zone_boundaries(painter: &egui::Painter, view_rect: egui::Rect, color: egui::Color32) {
    let min = view_rect.min;
    let max = view_rect.max;

    painter.line_segment(
        [egui::pos2(min.x, max.y), egui::pos2(max.x, min.y)],
        egui::Stroke::new(1.0, color),
    );
    painter.line_segment(
        [min, egui::pos2(max.x, max.y)],
        egui::Stroke::new(1.0, color),
    );
}

fn render_9zone_boundaries(painter: &egui::Painter, view_rect: egui::Rect, color: egui::Color32) {
    let min = view_rect.min;
    let max = view_rect.max;
    let width = view_rect.width();
    let height = view_rect.height();

    let third_w = width / 3.0;
    let third_h = height / 3.0;

    painter.line_segment(
        [
            egui::pos2(min.x + third_w, min.y),
            egui::pos2(min.x + third_w, max.y),
        ],
        egui::Stroke::new(1.0, color),
    );
    painter.line_segment(
        [
            egui::pos2(min.x + 2.0 * third_w, min.y),
            egui::pos2(min.x + 2.0 * third_w, max.y),
        ],
        egui::Stroke::new(1.0, color),
    );
    painter.line_segment(
        [
            egui::pos2(min.x, min.y + third_h),
            egui::pos2(max.x, min.y + third_h),
        ],
        egui::Stroke::new(1.0, color),
    );
    painter.line_segment(
        [
            egui::pos2(min.x, min.y + 2.0 * third_h),
            egui::pos2(max.x, min.y + 2.0 * third_h),
        ],
        egui::Stroke::new(1.0, color),
    );
}

fn render_zone_labels(
    painter: &egui::Painter,
    view_rect: egui::Rect,
    zone_mode: ZoneMode,
    options: &ZoneDebugOptions,
) {
    match zone_mode {
        ZoneMode::FourZones => render_4zone_labels(painter, view_rect, options),
        ZoneMode::NineZones => render_9zone_labels(painter, view_rect, options),
    }
}

fn render_4zone_labels(painter: &egui::Painter, view_rect: egui::Rect, options: &ZoneDebugOptions) {
    let center = view_rect.center();
    let offset = (view_rect.width().min(view_rect.height()) * 0.15).min(30.0);

    let labels = [
        (
            Zone::North,
            "N",
            egui::pos2(center.x, view_rect.min.y + offset),
        ),
        (
            Zone::South,
            "S",
            egui::pos2(center.x, view_rect.max.y - offset),
        ),
        (
            Zone::West,
            "W",
            egui::pos2(view_rect.min.x + offset, center.y),
        ),
        (
            Zone::East,
            "E",
            egui::pos2(view_rect.max.x - offset, center.y),
        ),
    ];

    for (_zone, label, pos) in labels {
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(options.label_font_size),
            options.label_color,
        );
    }
}

fn render_9zone_labels(painter: &egui::Painter, view_rect: egui::Rect, options: &ZoneDebugOptions) {
    let min = view_rect.min;
    let width = view_rect.width();
    let height = view_rect.height();

    let third_w = width / 3.0;
    let third_h = height / 3.0;

    let labels = [
        (
            Zone::NorthWest,
            "NW",
            egui::pos2(min.x + third_w / 2.0, min.y + third_h / 2.0),
        ),
        (
            Zone::North,
            "N",
            egui::pos2(min.x + third_w * 1.5, min.y + third_h / 2.0),
        ),
        (
            Zone::NorthEast,
            "NE",
            egui::pos2(min.x + third_w * 2.5, min.y + third_h / 2.0),
        ),
        (
            Zone::West,
            "W",
            egui::pos2(min.x + third_w / 2.0, min.y + third_h * 1.5),
        ),
        (
            Zone::Center,
            "C",
            egui::pos2(min.x + third_w * 1.5, min.y + third_h * 1.5),
        ),
        (
            Zone::East,
            "E",
            egui::pos2(min.x + third_w * 2.5, min.y + third_h * 1.5),
        ),
        (
            Zone::SouthWest,
            "SW",
            egui::pos2(min.x + third_w / 2.0, min.y + third_h * 2.5),
        ),
        (
            Zone::South,
            "S",
            egui::pos2(min.x + third_w * 1.5, min.y + third_h * 2.5),
        ),
        (
            Zone::SouthEast,
            "SE",
            egui::pos2(min.x + third_w * 2.5, min.y + third_h * 2.5),
        ),
    ];

    for (_zone, label, pos) in labels {
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(options.label_font_size),
            options.label_color,
        );
    }
}
