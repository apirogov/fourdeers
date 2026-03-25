//! Zone detection for stereoscopic view tap zones

use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragView {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct TapAnalysis {
    pub is_left_view: bool,
    pub view_rect: egui::Rect,
    pub zone: Zone,
    pub norm_x: f32,
    pub norm_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TetraId {
    pub is_left_view: bool,
    pub zone: Zone,
}

pub fn analyze_tap_in_stereo_view(
    visualization_rect: egui::Rect,
    tap_pos: egui::Pos2,
) -> Option<TapAnalysis> {
    if !visualization_rect.contains(tap_pos) {
        return None;
    }

    let center_x = visualization_rect.center().x;
    let is_left_view = tap_pos.x < center_x;

    let view_rect = if is_left_view {
        egui::Rect {
            min: visualization_rect.min,
            max: egui::pos2(center_x, visualization_rect.max.y),
        }
    } else {
        egui::Rect {
            min: egui::pos2(center_x, visualization_rect.min.y),
            max: visualization_rect.max,
        }
    };

    let zone = get_zone_from_rect(view_rect, tap_pos)?;

    let norm_x = (tap_pos.x - view_rect.min.x) / view_rect.width();
    let norm_y = (tap_pos.y - view_rect.min.y) / view_rect.height();

    Some(TapAnalysis {
        is_left_view,
        view_rect,
        zone,
        norm_x,
        norm_y,
    })
}

pub fn get_zone_from_rect(rect: egui::Rect, point: egui::Pos2) -> Option<Zone> {
    if !rect.contains(point) {
        return None;
    }

    let width = rect.width();
    let height = rect.height();

    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let norm_x = (point.x - rect.min.x) / width;
    let norm_y = (point.y - rect.min.y) / height;

    let above_nw_se = norm_y < 1.0 - norm_x;
    let above_ne_sw = norm_y < norm_x;

    if above_nw_se && above_ne_sw {
        Some(Zone::North)
    } else if !above_nw_se && !above_ne_sw {
        Some(Zone::South)
    } else if above_nw_se && !above_ne_sw {
        Some(Zone::West)
    } else {
        Some(Zone::East)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_centered_rect(width: f32, height: f32) -> egui::Rect {
        egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(width, height),
        }
    }

    #[test]
    fn test_analyze_tap_in_stereo_view() {
        let vis_rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(200.0, 100.0),
        };

        let analysis = analyze_tap_in_stereo_view(vis_rect, egui::pos2(50.0, 10.0)).unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);

        let analysis = analyze_tap_in_stereo_view(vis_rect, egui::pos2(150.0, 10.0)).unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);

        assert!(analyze_tap_in_stereo_view(vis_rect, egui::pos2(-10.0, 50.0)).is_none());
    }

    #[test]
    fn test_get_zone_from_rect() {
        let rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(100.0, 100.0),
        };

        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 25.0)),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 75.0)),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(25.0, 50.0)),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(75.0, 50.0)),
            Some(Zone::East)
        );
    }
}
