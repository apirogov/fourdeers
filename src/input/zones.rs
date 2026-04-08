//! Zone detection for stereoscopic view tap zones

use eframe::egui;

use crate::camera::CameraAction;

/// Maps a tap zone to a camera movement action.
///
/// Cardinal zones map to directional moves, diagonal zones to forward/backward/kata/ana.
/// Returns `None` for `Center` and other non-movement zones.
#[must_use]
pub const fn zone_to_movement_action(zone: Zone) -> Option<CameraAction> {
    match zone {
        Zone::North => Some(CameraAction::MoveUp),
        Zone::South => Some(CameraAction::MoveDown),
        Zone::West => Some(CameraAction::MoveLeft),
        Zone::East => Some(CameraAction::MoveRight),
        Zone::NorthEast => Some(CameraAction::MoveForward),
        Zone::SouthWest => Some(CameraAction::MoveBackward),
        Zone::NorthWest => Some(CameraAction::MoveKata),
        Zone::SouthEast => Some(CameraAction::MoveAna),
        Zone::Center => None,
    }
}

/// How many zones to divide a view rect into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZoneMode {
    /// Divide into 4 quadrants (N/E/S/W)
    #[default]
    FourZones,
    /// Divide into 9 regions (3×3 grid)
    NineZones,
}

/// A named region within a divided view rect.
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

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::North => write!(f, "North"),
            Zone::East => write!(f, "East"),
            Zone::South => write!(f, "South"),
            Zone::West => write!(f, "West"),
            Zone::NorthWest => write!(f, "NorthWest"),
            Zone::NorthEast => write!(f, "NorthEast"),
            Zone::Center => write!(f, "Center"),
            Zone::SouthWest => write!(f, "SouthWest"),
            Zone::SouthEast => write!(f, "SouthEast"),
        }
    }
}

impl Zone {
    /// Returns true for N/E/S/W (cardinal directions), false for diagonals and center.
    #[must_use]
    pub const fn is_cardinal(self) -> bool {
        matches!(self, Zone::North | Zone::East | Zone::South | Zone::West)
    }

    /// All 9 zones in grid order (NW, N, NE, W, C, E, SW, S, SE).
    #[must_use]
    pub const fn all() -> [Zone; 9] {
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

    /// The 4 cardinal zones: N, E, S, W.
    #[must_use]
    pub const fn cardinals() -> [Zone; 4] {
        [Zone::North, Zone::East, Zone::South, Zone::West]
    }
}

/// Which stereo view half a drag or tap occurred in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragView {
    Left,
    Right,
}

/// Result of analyzing a tap within a stereo view rect.
#[derive(Debug, Clone)]
pub struct TapAnalysis {
    pub is_left_view: bool,
    pub view_rect: egui::Rect,
    pub zone: Zone,
    pub zone_mode: ZoneMode,
    pub norm_x: f32,
    pub norm_y: f32,
}

/// Identifies a specific tetrahedron by its view half and zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TetraId {
    pub is_left_view: bool,
    pub zone: Zone,
}

#[must_use]
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

#[must_use]
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
        ZoneMode::FourZones => Some(get_zone_4way(norm_x, norm_y)),
        ZoneMode::NineZones => Some(get_zone_9way(norm_x, norm_y)),
    }
}

fn get_zone_4way(norm_x: f32, norm_y: f32) -> Zone {
    let above_nw_se = norm_y < 1.0 - norm_x;
    let above_ne_sw = norm_y < norm_x;

    if above_nw_se && above_ne_sw {
        Zone::North
    } else if !above_nw_se && !above_ne_sw {
        Zone::South
    } else if above_nw_se && !above_ne_sw {
        Zone::West
    } else {
        Zone::East
    }
}

fn get_zone_9way(norm_x: f32, norm_y: f32) -> Zone {
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

    match (row, col) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let outside = analyze_tap_in_stereo_view_with_modes(
            vis_rect,
            egui::pos2(-10.0, 50.0),
            ZoneMode::NineZones,
            ZoneMode::FourZones,
        );
        assert!(outside.is_none());
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

    #[test]
    fn test_zone_to_movement_action_cardinal() {
        use crate::camera::CameraAction;
        assert_eq!(
            zone_to_movement_action(Zone::North),
            Some(CameraAction::MoveUp)
        );
        assert_eq!(
            zone_to_movement_action(Zone::South),
            Some(CameraAction::MoveDown)
        );
        assert_eq!(
            zone_to_movement_action(Zone::West),
            Some(CameraAction::MoveLeft)
        );
        assert_eq!(
            zone_to_movement_action(Zone::East),
            Some(CameraAction::MoveRight)
        );
    }

    #[test]
    fn test_zone_to_movement_action_diagonal() {
        use crate::camera::CameraAction;
        assert_eq!(
            zone_to_movement_action(Zone::NorthEast),
            Some(CameraAction::MoveForward)
        );
        assert_eq!(
            zone_to_movement_action(Zone::SouthWest),
            Some(CameraAction::MoveBackward)
        );
        assert_eq!(
            zone_to_movement_action(Zone::NorthWest),
            Some(CameraAction::MoveKata)
        );
        assert_eq!(
            zone_to_movement_action(Zone::SouthEast),
            Some(CameraAction::MoveAna)
        );
    }

    #[test]
    fn test_zone_to_movement_action_none() {
        assert_eq!(zone_to_movement_action(Zone::Center), None);
    }

    #[test]
    fn test_zone_display() {
        assert_eq!(Zone::North.to_string(), "North");
        assert_eq!(Zone::SouthEast.to_string(), "SouthEast");
        assert_eq!(Zone::Center.to_string(), "Center");
    }
}
