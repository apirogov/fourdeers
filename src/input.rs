//! Input handling for stereoscopic view tap zones

/// Directional zone enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    North,
    East,
    South,
    West,
}

/// Camera movement action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    MoveUp,
    MoveDown,
    IncreaseW,
    DecreaseW,
    MoveSliceForward,
    MoveSliceBackward,
    MoveSliceOrthogonalPos,
    MoveSliceOrthogonalNeg,
}

/// Result of analyzing a tap within a stereoscopic view
#[derive(Debug, Clone)]
pub struct TapAnalysis {
    pub is_left_view: bool,
    pub view_rect: eframe::egui::Rect,
    pub zone: Zone,
    pub norm_x: f32,
    pub norm_y: f32,
}

/// Map a zone and view to a camera action
pub fn zone_to_action(zone: Zone, is_left_view: bool) -> CameraAction {
    if is_left_view {
        match zone {
            Zone::North => CameraAction::MoveUp,
            Zone::South => CameraAction::MoveDown,
            Zone::West => CameraAction::StrafeLeft,
            Zone::East => CameraAction::StrafeRight,
        }
    } else {
        match zone {
            Zone::North => CameraAction::MoveSliceForward,
            Zone::South => CameraAction::MoveSliceBackward,
            Zone::West => CameraAction::MoveSliceOrthogonalNeg,
            Zone::East => CameraAction::MoveSliceOrthogonalPos,
        }
    }
}

/// Analyze a tap position within a stereoscopic visualization rectangle
pub fn analyze_tap_in_stereo_view(
    visualization_rect: eframe::egui::Rect,
    tap_pos: eframe::egui::Pos2,
) -> Option<TapAnalysis> {
    if !visualization_rect.contains(tap_pos) {
        return None;
    }

    let center_x = visualization_rect.center().x;
    let is_left_view = tap_pos.x < center_x;

    let view_rect = if is_left_view {
        eframe::egui::Rect {
            min: visualization_rect.min,
            max: eframe::egui::pos2(center_x, visualization_rect.max.y),
        }
    } else {
        eframe::egui::Rect {
            min: eframe::egui::pos2(center_x, visualization_rect.min.y),
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

/// Determine which zone a point falls into within a rectangle.
///
/// The rectangle is divided by two diagonal lines creating an X pattern:
/// - One line from NW to SE
/// - One line from NE to SW
///
/// This creates 4 edge-aligned triangular zones:
/// - North: point above both diagonals
/// - South: point below both diagonals
/// - West: point above NW-SE diagonal but below NE-SW diagonal
/// - East: point below NW-SE diagonal but above NE-SW diagonal
pub fn get_zone_from_rect(rect: eframe::egui::Rect, point: eframe::egui::Pos2) -> Option<Zone> {
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

    #[cfg(debug_assertions)]
    println!(
        "  ZONE DEBUG: rect=[({:.1},{:.1})-({:.1},{:.1})], point=({:.1},{:.1}), norm=({:.3},{:.3}), above_nw_se={}, above_ne_sw={}",
        rect.min.x, rect.min.y, rect.max.x, rect.max.y,
        point.x, point.y,
        norm_x, norm_y, above_nw_se, above_ne_sw
    );

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

    fn create_centered_rect(width: f32, height: f32) -> eframe::egui::Rect {
        eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(width, height),
        }
    }

    #[test]
    fn test_analyze_tap_in_stereo_view() {
        let vis_rect = eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(200.0, 100.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(50.0, 10.0)).unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);
        assert!(analysis.norm_x > 0.4 && analysis.norm_x < 0.6);
        assert!(analysis.norm_y < 0.2);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(50.0, 90.0)).unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::South);
        assert!(analysis.norm_y > 0.8);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(10.0, 50.0)).unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::West);
        assert!(analysis.norm_x < 0.2);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(90.0, 50.0)).unwrap();
        assert!(analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::East);
        assert!(analysis.norm_x > 0.8);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(150.0, 10.0)).unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::North);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(150.0, 90.0)).unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::South);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(110.0, 50.0)).unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::West);

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(190.0, 50.0)).unwrap();
        assert!(!analysis.is_left_view);
        assert_eq!(analysis.zone, Zone::East);

        assert!(analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(-10.0, 50.0)).is_none());
        assert!(analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(210.0, 50.0)).is_none());
        assert!(analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(100.0, -10.0)).is_none());
        assert!(analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(100.0, 110.0)).is_none());
    }

    #[test]
    fn test_specific_issue_right_view_west_zone() {
        let vis_rect = eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(200.0, 100.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(125.0, 50.0)).unwrap();
        assert!(!analysis.is_left_view, "Should be right view");
        assert_eq!(
            analysis.zone,
            Zone::West,
            "Point at normalized (0.25, 0.5) in right view should be West zone"
        );
        assert!((analysis.norm_x - 0.25).abs() < 0.01);
        assert!((analysis.norm_y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_specific_issue_right_view_east_zone() {
        let vis_rect = eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(200.0, 100.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(vis_rect, eframe::egui::pos2(175.0, 50.0)).unwrap();
        assert!(!analysis.is_left_view, "Should be right view");
        assert_eq!(
            analysis.zone,
            Zone::East,
            "Point at normalized (0.75, 0.5) in right view should be East zone"
        );
        assert!((analysis.norm_x - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_with_sidebar_scenario() {
        let visualization_rect = eframe::egui::Rect {
            min: eframe::egui::pos2(280.0, 0.0),
            max: eframe::egui::pos2(800.0, 600.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(visualization_rect, eframe::egui::pos2(605.0, 300.0))
                .unwrap();
        assert!(!analysis.is_left_view, "Should be right view");
        assert_eq!(
            analysis.zone,
            Zone::West,
            "Left side of right view should be West"
        );
        assert!((analysis.norm_x - 0.25).abs() < 0.01);

        let analysis =
            analyze_tap_in_stereo_view(visualization_rect, eframe::egui::pos2(735.0, 300.0))
                .unwrap();
        assert!(!analysis.is_left_view, "Should be right view");
        assert_eq!(
            analysis.zone,
            Zone::East,
            "Right side of right view should be East"
        );
        assert!((analysis.norm_x - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_negative_normalized_x_bug() {
        let visualization_rect = eframe::egui::Rect {
            min: eframe::egui::pos2(280.0, 0.0),
            max: eframe::egui::pos2(800.0, 600.0),
        };

        let analysis =
            analyze_tap_in_stereo_view(visualization_rect, eframe::egui::pos2(350.0, 300.0))
                .unwrap();
        assert!(
            analysis.is_left_view,
            "x=350 should be in left view (center at 540)"
        );
        let expected_norm_x = (350.0 - 280.0) / 260.0;
        assert!(
            (analysis.norm_x - expected_norm_x).abs() < 0.01,
            "Normalized x should be ~{:.3}, got {:.3}",
            expected_norm_x,
            analysis.norm_x
        );
        assert!(
            analysis.norm_x >= 0.0 && analysis.norm_x <= 1.0,
            "Normalized x should be in 0-1 range, got {}",
            analysis.norm_x
        );

        let analysis =
            analyze_tap_in_stereo_view(visualization_rect, eframe::egui::pos2(500.0, 300.0))
                .unwrap();
        assert!(
            analysis.is_left_view,
            "x=500 should be in left view (center at 540)"
        );
        let expected_norm_x = (500.0 - 280.0) / 260.0;
        assert!(
            (analysis.norm_x - expected_norm_x).abs() < 0.01,
            "Normalized x should be ~{:.3}, got {:.3}",
            expected_norm_x,
            analysis.norm_x
        );
        assert!(
            analysis.norm_x >= 0.0 && analysis.norm_x <= 1.0,
            "Normalized x should be in 0-1 range, got {}",
            analysis.norm_x
        );

        let analysis =
            analyze_tap_in_stereo_view(visualization_rect, eframe::egui::pos2(600.0, 300.0))
                .unwrap();
        assert!(
            !analysis.is_left_view,
            "x=600 should be in right view (center at 540)"
        );
        let expected_norm_x = (600.0 - 540.0) / 260.0;
        assert!(
            (analysis.norm_x - expected_norm_x).abs() < 0.01,
            "Normalized x should be ~{:.3}, got {:.3}",
            expected_norm_x,
            analysis.norm_x
        );
        assert!(
            analysis.norm_x >= 0.0 && analysis.norm_x <= 1.0,
            "Normalized x should be in 0-1 range, got {}",
            analysis.norm_x
        );
    }

    #[test]
    fn test_get_zone_from_rect_specific_points() {
        let rect = eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(100.0, 100.0),
        };

        let zone = get_zone_from_rect(rect, eframe::egui::pos2(25.0, 50.0)).unwrap();
        assert_eq!(zone, Zone::West, "Point at (0.25, 0.5) should be West");

        let zone = get_zone_from_rect(rect, eframe::egui::pos2(75.0, 50.0)).unwrap();
        assert_eq!(zone, Zone::East, "Point at (0.75, 0.5) should be East");

        let zone = get_zone_from_rect(rect, eframe::egui::pos2(50.0, 25.0)).unwrap();
        assert_eq!(zone, Zone::North, "Point at (0.5, 0.25) should be North");

        let zone = get_zone_from_rect(rect, eframe::egui::pos2(50.0, 75.0)).unwrap();
        assert_eq!(zone, Zone::South, "Point at (0.5, 0.75) should be South");
    }

    #[test]
    fn test_zone_north() {
        let rect = create_centered_rect(100.0, 100.0);
        let center = rect.center();

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 10.0)),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 20.0)),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(30.0, 15.0)),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(70.0, 15.0)),
            Some(Zone::North)
        );
    }

    #[test]
    fn test_zone_south() {
        let rect = create_centered_rect(100.0, 100.0);
        let center = rect.center();

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 90.0)),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 85.0)),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(30.0, 85.0)),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(70.0, 85.0)),
            Some(Zone::South)
        );
    }

    #[test]
    fn test_zone_west() {
        let rect = create_centered_rect(100.0, 100.0);
        let center = rect.center();

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(10.0, center.y)),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(15.0, center.y)),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(15.0, 30.0)),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(15.0, 70.0)),
            Some(Zone::West)
        );
    }

    #[test]
    fn test_zone_east() {
        let rect = create_centered_rect(100.0, 100.0);
        let center = rect.center();

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(90.0, center.y)),
            Some(Zone::East)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(85.0, center.y)),
            Some(Zone::East)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(85.0, 30.0)),
            Some(Zone::East)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(85.0, 70.0)),
            Some(Zone::East)
        );
    }

    #[test]
    fn test_point_outside_rect() {
        let rect = create_centered_rect(100.0, 100.0);

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(-10.0, 50.0)),
            None
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(110.0, 50.0)),
            None
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(50.0, -10.0)),
            None
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(50.0, 110.0)),
            None
        );
    }

    #[test]
    fn test_center_point() {
        let rect = create_centered_rect(100.0, 100.0);
        let center = rect.center();

        assert!(get_zone_from_rect(rect, center).is_some());
    }

    #[test]
    fn test_diagonal_boundaries() {
        let rect = create_centered_rect(100.0, 100.0);

        assert!(get_zone_from_rect(rect, eframe::egui::pos2(0.0, 100.0)).is_some());
        assert!(get_zone_from_rect(rect, eframe::egui::pos2(100.0, 0.0)).is_some());
        assert!(get_zone_from_rect(rect, eframe::egui::pos2(0.0, 0.0)).is_some());
        assert!(get_zone_from_rect(rect, eframe::egui::pos2(100.0, 100.0)).is_some());
    }

    #[test]
    fn test_asymmetric_rect() {
        let rect = eframe::egui::Rect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(200.0, 100.0),
        };
        let center = rect.center();

        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 10.0)),
            Some(Zone::North)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(center.x, 90.0)),
            Some(Zone::South)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(10.0, center.y)),
            Some(Zone::West)
        );
        assert_eq!(
            get_zone_from_rect(rect, eframe::egui::pos2(190.0, center.y)),
            Some(Zone::East)
        );
    }

    #[test]
    fn test_zone_to_action_left_view() {
        assert_eq!(zone_to_action(Zone::North, true), CameraAction::MoveUp);
        assert_eq!(zone_to_action(Zone::South, true), CameraAction::MoveDown);
        assert_eq!(zone_to_action(Zone::West, true), CameraAction::StrafeLeft);
        assert_eq!(zone_to_action(Zone::East, true), CameraAction::StrafeRight);
    }

    #[test]
    fn test_zone_to_action_right_view() {
        assert_eq!(
            zone_to_action(Zone::North, false),
            CameraAction::MoveSliceForward
        );
        assert_eq!(
            zone_to_action(Zone::South, false),
            CameraAction::MoveSliceBackward
        );
        assert_eq!(
            zone_to_action(Zone::West, false),
            CameraAction::MoveSliceOrthogonalNeg
        );
        assert_eq!(
            zone_to_action(Zone::East, false),
            CameraAction::MoveSliceOrthogonalPos
        );
    }
}
