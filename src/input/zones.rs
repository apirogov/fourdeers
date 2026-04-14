//! Zone detection for stereoscopic view tap zones

use eframe::egui;

use crate::camera::Direction4D;

/// Maps a tap zone to a camera movement action.
///
/// Cardinal zones map to directional moves, diagonal zones to forward/backward/kata/ana.
/// Returns `None` for `Center` and other non-movement zones.
#[must_use]
pub const fn zone_to_movement_action(zone: Zone) -> Option<Direction4D> {
    match zone {
        Zone::North => Some(Direction4D::Up),
        Zone::South => Some(Direction4D::Down),
        Zone::West => Some(Direction4D::Left),
        Zone::East => Some(Direction4D::Right),
        Zone::NorthEast => Some(Direction4D::Forward),
        Zone::SouthWest => Some(Direction4D::Backward),
        Zone::NorthWest => Some(Direction4D::Kata),
        Zone::SouthEast => Some(Direction4D::Ana),
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

/// Result of analyzing a pointer event within a stereo view rect.
#[derive(Debug, Clone)]
pub struct PointerAnalysis {
    pub is_left_view: bool,
    pub norm_pos: egui::Vec2,
    pub zone: Option<Zone>,
    pub drag_delta: egui::Vec2,
    pub drag_view: Option<DragView>,
    pub is_hold: bool,
    pub is_drag: bool,
    pub tap_pos: egui::Pos2,
}

#[must_use]
pub fn analyze_pointer_initial(
    visualization_rect: egui::Rect,
    tap_pos: egui::Pos2,
    left_zone_mode: ZoneMode,
    right_zone_mode: ZoneMode,
) -> Option<PointerAnalysis> {
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

    let zone = zone_from_rect(view_rect, tap_pos, zone_mode)?;

    // Normalized position within the view rect (0.0 to 1.0)
    let norm_x = (tap_pos.x - view_rect.min.x) / view_rect.width();
    let norm_y = (tap_pos.y - view_rect.min.y) / view_rect.height();
    let norm_pos = egui::vec2(norm_x, norm_y);

    Some(PointerAnalysis {
        is_left_view,
        norm_pos,
        zone: Some(zone),
        drag_delta: egui::Vec2::ZERO,
        drag_view: None,
        is_hold: false,
        is_drag: false,
        tap_pos,
    })
}

/// Identifies a specific tetrahedron by its view half and zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TetraId {
    pub is_left_view: bool,
    pub zone: Zone,
}

#[must_use]
pub fn zone_from_rect(rect: egui::Rect, point: egui::Pos2, mode: ZoneMode) -> Option<Zone> {
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
        ZoneMode::FourZones => Some(zone_4way(norm_x, norm_y)),
        ZoneMode::NineZones => Some(zone_9way(norm_x, norm_y)),
    }
}

fn zone_4way(norm_x: f32, norm_y: f32) -> Zone {
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

fn zone_9way(norm_x: f32, norm_y: f32) -> Zone {
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
    fn test_analyze_pointer_initial_includes_geometry() {
        let visualization_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 100.0));
        let tap_pos = egui::pos2(50.0, 25.0);
        let analysis = analyze_pointer_initial(
            visualization_rect,
            tap_pos,
            ZoneMode::NineZones,
            ZoneMode::NineZones,
        )
        .unwrap();
        assert_eq!(analysis.is_left_view, true);
        assert_eq!(analysis.zone, Some(Zone::North)); // top-middle of left half
        assert_eq!(analysis.norm_pos.x, 0.5);
        assert_eq!(analysis.norm_pos.y, 0.25);
        assert_eq!(analysis.drag_delta, egui::Vec2::ZERO);
        assert_eq!(analysis.drag_view, None);
        assert_eq!(analysis.is_hold, false);
        assert_eq!(analysis.is_drag, false);
    }

    #[test]
    fn test_analyze_pointer_initial_outside() {
        let visualization_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 100.0));
        let tap_pos = egui::pos2(-10.0, 50.0);
        let analysis = analyze_pointer_initial(
            visualization_rect,
            tap_pos,
            ZoneMode::NineZones,
            ZoneMode::NineZones,
        );
        assert!(analysis.is_none());
    }

    #[test]
    fn test_zone_from_rect_4way() {
        let rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(100.0, 100.0),
        };

        assert_eq!(
            zone_from_rect(rect, egui::pos2(50.0, 25.0), ZoneMode::FourZones),
            Some(Zone::North)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(50.0, 75.0), ZoneMode::FourZones),
            Some(Zone::South)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(25.0, 50.0), ZoneMode::FourZones),
            Some(Zone::West)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(75.0, 50.0), ZoneMode::FourZones),
            Some(Zone::East)
        );
    }

    #[test]
    fn test_zone_from_rect_9way() {
        let rect = egui::Rect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(300.0, 300.0),
        };

        assert_eq!(
            zone_from_rect(rect, egui::pos2(50.0, 50.0), ZoneMode::NineZones),
            Some(Zone::NorthWest)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(150.0, 50.0), ZoneMode::NineZones),
            Some(Zone::North)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(250.0, 50.0), ZoneMode::NineZones),
            Some(Zone::NorthEast)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(50.0, 150.0), ZoneMode::NineZones),
            Some(Zone::West)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(150.0, 150.0), ZoneMode::NineZones),
            Some(Zone::Center)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(250.0, 150.0), ZoneMode::NineZones),
            Some(Zone::East)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(50.0, 250.0), ZoneMode::NineZones),
            Some(Zone::SouthWest)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(150.0, 250.0), ZoneMode::NineZones),
            Some(Zone::South)
        );
        assert_eq!(
            zone_from_rect(rect, egui::pos2(250.0, 250.0), ZoneMode::NineZones),
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
        use crate::camera::Direction4D;
        assert_eq!(zone_to_movement_action(Zone::North), Some(Direction4D::Up));
        assert_eq!(
            zone_to_movement_action(Zone::South),
            Some(Direction4D::Down)
        );
        assert_eq!(zone_to_movement_action(Zone::West), Some(Direction4D::Left));
        assert_eq!(
            zone_to_movement_action(Zone::East),
            Some(Direction4D::Right)
        );
    }

    #[test]
    fn test_zone_to_movement_action_diagonal() {
        use crate::camera::Direction4D;
        assert_eq!(
            zone_to_movement_action(Zone::NorthEast),
            Some(Direction4D::Forward)
        );
        assert_eq!(
            zone_to_movement_action(Zone::SouthWest),
            Some(Direction4D::Backward)
        );
        assert_eq!(
            zone_to_movement_action(Zone::NorthWest),
            Some(Direction4D::Kata)
        );
        assert_eq!(
            zone_to_movement_action(Zone::SouthEast),
            Some(Direction4D::Ana)
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
