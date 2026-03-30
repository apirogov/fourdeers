//! Zone detection for stereoscopic view tap zones

use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZoneMode {
    #[default]
    FourZones,
    NineZones,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    North,
    East,
    South,
    West,
    NorthWest,
    NorthEast,
    Center,
    SouthWest,
    SouthEast,
}

impl Zone {
    pub fn is_cardinal(self) -> bool {
        matches!(self, Zone::North | Zone::East | Zone::South | Zone::West)
    }

    pub fn all() -> [Zone; 9] {
        [
            Zone::NorthWest,
            Zone::North,
            Zone::NorthEast,
            Zone::West,
            Zone::Center,
            Zone::East,
            Zone::SouthWest,
            Zone::South,
            Zone::SouthEast,
        ]
    }

    pub fn cardinals() -> [Zone; 4] {
        [Zone::North, Zone::East, Zone::South, Zone::West]
    }
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
    pub zone_mode: ZoneMode,
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
    zone_mode: ZoneMode,
) -> Option<TapAnalysis> {
    analyze_tap_in_stereo_view_with_modes(visualization_rect, tap_pos, zone_mode, zone_mode)
}

pub fn analyze_tap_in_stereo_view_with_modes(
    visualization_rect: egui::Rect,
    tap_pos: egui::Pos2,
    left_zone_mode: ZoneMode,
    right_zone_mode: ZoneMode,
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

    let zone_mode = if is_left_view {
        left_zone_mode
    } else {
        right_zone_mode
    };

    let zone = get_zone_from_rect(view_rect, tap_pos, zone_mode)?;

    let norm_x = (tap_pos.x - view_rect.min.x) / view_rect.width();
    let norm_y = (tap_pos.y - view_rect.min.y) / view_rect.height();

    Some(TapAnalysis {
        is_left_view,
        view_rect,
        zone,
        zone_mode,
        norm_x,
        norm_y,
    })
}

pub fn get_zone_from_rect(rect: egui::Rect, point: egui::Pos2, mode: ZoneMode) -> Option<Zone> {
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

    match mode {
        ZoneMode::FourZones => get_zone_4way(norm_x, norm_y),
        ZoneMode::NineZones => get_zone_9way(norm_x, norm_y),
    }
}

fn get_zone_4way(norm_x: f32, norm_y: f32) -> Option<Zone> {
    let above_nw_se = norm_y < 1.0 - norm_x;
    let above_ne_sw = norm_y < norm_x;

    let zone = if above_nw_se && above_ne_sw {
        Zone::North
    } else if !above_nw_se && !above_ne_sw {
        Zone::South
    } else if above_nw_se && !above_ne_sw {
        Zone::West
    } else {
        Zone::East
    };

    Some(zone)
}

fn get_zone_9way(norm_x: f32, norm_y: f32) -> Option<Zone> {
    let third_x = 1.0 / 3.0;
    let third_y = 1.0 / 3.0;

    let col = if norm_x < third_x {
        0
    } else if norm_x < 2.0 * third_x {
        1
    } else {
        2
    };

    let row = if norm_y < third_y {
        0
    } else if norm_y < 2.0 * third_y {
        1
    } else {
        2
    };

    let zone = match (row, col) {
        (0, 0) => Zone::NorthWest,
        (0, 1) => Zone::North,
        (0, 2) => Zone::NorthEast,
        (1, 0) => Zone::West,
        (1, 1) => Zone::Center,
        (1, 2) => Zone::East,
        (2, 0) => Zone::SouthWest,
        (2, 1) => Zone::South,
        (2, 2) => Zone::SouthEast,
        _ => unreachable!(),
    };

    Some(zone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_tap_in_stereo_view() {
        let vis_rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(200.0, 100.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, egui::pos2(50.0, 10.0), ZoneMode::FourZones)
                .unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, egui::pos2(150.0, 10.0), ZoneMode::FourZones)
                .unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);

        assert!(
            analyze_tap_in_stereo_view(vis_rect, egui::pos2(-10.0, 50.0), ZoneMode::FourZones)
                .is_none()
        );
    }

    #[test]
    fn test_analyze_tap_in_stereo_view_with_modes() {
        let vis_rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(200.0, 100.0),
        };

        let left = analyze_tap_in_stereo_view_with_modes(
            vis_rect,
            egui::pos2(10.0, 10.0),
            ZoneMode::NineZones,
            ZoneMode::FourZones,
        )
        .expect("left analysis");
        assert!(left.is_left_view);
        assert_eq!(left.zone_mode, ZoneMode::NineZones);
        assert_eq!(left.zone, Zone::NorthWest);

        let right = analyze_tap_in_stereo_view_with_modes(
            vis_rect,
            egui::pos2(150.0, 10.0),
            ZoneMode::NineZones,
            ZoneMode::FourZones,
        )
        .expect("right analysis");
        assert!(!right.is_left_view);
        assert_eq!(right.zone_mode, ZoneMode::FourZones);
        assert_eq!(right.zone, Zone::North);
    }

    #[test]
    fn test_get_zone_from_rect_4way() {
        let rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(100.0, 100.0),
        };

        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 25.0), ZoneMode::FourZones),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 75.0), ZoneMode::FourZones),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(25.0, 50.0), ZoneMode::FourZones),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(75.0, 50.0), ZoneMode::FourZones),
            Some(Zone::East)
        );
    }

    #[test]
    fn test_get_zone_from_rect_9way() {
        let rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(300.0, 300.0),
        };

        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 50.0), ZoneMode::NineZones),
            Some(Zone::NorthWest)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(150.0, 50.0), ZoneMode::NineZones),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(250.0, 50.0), ZoneMode::NineZones),
            Some(Zone::NorthEast)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 150.0), ZoneMode::NineZones),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(150.0, 150.0), ZoneMode::NineZones),
            Some(Zone::Center)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(250.0, 150.0), ZoneMode::NineZones),
            Some(Zone::East)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(50.0, 250.0), ZoneMode::NineZones),
            Some(Zone::SouthWest)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(150.0, 250.0), ZoneMode::NineZones),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, egui::pos2(250.0, 250.0), ZoneMode::NineZones),
            Some(Zone::SouthEast)
        );
    }

    #[test]
    fn test_is_cardinal() {
        assert!(Zone::North.is_cardinal());
        assert!(Zone::East.is_cardinal());
        assert!(Zone::South.is_cardinal());
        assert!(Zone::West.is_cardinal());
        assert!(!Zone::NorthWest.is_cardinal());
        assert!(!Zone::NorthEast.is_cardinal());
        assert!(!Zone::Center.is_cardinal());
        assert!(!Zone::SouthWest.is_cardinal());
        assert!(!Zone::SouthEast.is_cardinal());
    }

    #[test]
    fn test_zone_mode_default() {
        assert_eq!(ZoneMode::default(), ZoneMode::FourZones);
    }
}
