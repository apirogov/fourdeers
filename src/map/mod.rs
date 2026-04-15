//! 4D Map view: a tesseract representing the 4D scene volume
//!
//! The map renders a tesseract wireframe (the 8-cell / hypercube) whose vertices
//! correspond to the signed axis extremes of the scene bounds. Inside, it shows:
//! - The scene camera's current 3D slice as a green filled truncated cube
//! - Waypoints as full labeled tetrahedra
//! - The scene camera position as a labeled tetrahedron
//!
//! The map has its own Camera allowing independent 3D/4D navigation.

#![allow(clippy::excessive_precision)]

use eframe::egui;

#[cfg(test)]
use crate::camera::Camera;
#[cfg(test)]
use crate::render::{ProjectionMode, StereoProjector};
#[cfg(test)]
use crate::rotation4d::Rotation4D;

pub mod bounds;
pub mod helpers;
pub mod renderer;
pub mod slice;
pub mod view;
pub mod visibility;

pub(crate) use crate::camera::CameraProjection;

pub use bounds::{compute_bounds, normalize_to_tesseract};
pub use renderer::{MapRenderParams, MapRenderer};
pub use view::MapView;

pub(super) const BOUNDS_PADDING_FACTOR: f32 = 0.2;
pub(super) use crate::colors::{DIM_GRAY, SLICE_GREEN, VISIBILITY_DARK_GREEN};
pub(super) const MAP_CAMERA_BACK_OFFSET: f32 = 4.0;
pub(super) const NEAR_MARGIN: f32 = 0.5;
pub(super) const TETRA_SCALE_WAYPOINT: f32 = 0.15;
pub(super) const TETRA_SCALE_CAMERA: f32 = 0.2;
pub(super) const FORWARD_ARROW_LENGTH: f32 = 0.4;
pub(super) const MAP_TETRA_ARROW_STROKE_WIDTH: f32 = 2.0;
pub(super) const MAP_TETRA_ORIGIN_DOT_RADIUS: f32 = 2.0;
pub(super) const MAP_TETRA_TIP_DOT_RADIUS: f32 = 0.0;
pub(super) const MAP_TETRA_LABEL_NORMAL_OFFSET: f32 = 0.12;
pub(super) use crate::render::TESSERACT_EDGE_STROKE_WIDTH as EDGE_STROKE_WIDTH;
pub(super) const TAP_RADIUS_MULTIPLIER: f32 = 6.0;
pub(super) const TAP_RADIUS_MIN: f32 = 25.0;
pub(super) const TAP_RADIUS_MAX: f32 = 60.0;

pub(super) const MAP_ARROW_HEAD_SCALE: f32 = 15.0;
pub(super) const MAP_WAYPOINT_DOT_RADIUS: f32 = 3.0;
pub(super) const MAP_CAMERA_DOT_RADIUS: f32 = 3.0;
pub(super) const MAP_AXIS_FONT_SIZE: f32 = 8.0;
pub(super) const MAP_AXIS_LABEL_OFFSET_Y: f32 = 8.0;
pub(super) const MAP_VERTEX_FONT_SIZE: f32 = 10.0;
pub(super) const MAP_EDGE_LABEL_OFFSET: egui::Vec2 = egui::Vec2::new(4.0, -6.0);
pub(super) const MAP_TIP_FONT_SIZE: f32 = 9.0;
pub(super) const MAP_TIP_LABEL_OFFSET_Y: f32 = 12.0;
pub(super) const MAP_DISTANCE_FONT_SIZE: f32 = 8.0;
pub(super) const MAP_DISTANCE_LABEL_OFFSET_Y: f32 = 12.0;

#[cfg(test)]
pub(super) const TESSERACT_EDGE_COUNT: usize = 32;

#[cfg(test)]
pub(super) const TESSERACT_CROSS_SECTION_VERTEX_COUNT: usize = 8;

#[cfg(test)]
pub(super) const TESSERACT_CROSS_SECTION_EDGE_COUNT: usize = 12;

pub(super) const TESSERACT_FACES: [[u16; 4]; 24] = [
    [0, 2, 6, 4],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 1, 3, 2],
    [4, 5, 7, 6],
    [8, 10, 14, 12],
    [9, 11, 15, 13],
    [8, 9, 13, 12],
    [10, 11, 15, 14],
    [8, 9, 11, 10],
    [12, 13, 15, 14],
    [0, 2, 10, 8],
    [1, 3, 11, 9],
    [0, 1, 9, 8],
    [2, 3, 11, 10],
    [4, 6, 14, 12],
    [5, 7, 15, 13],
    [4, 5, 13, 12],
    [6, 7, 15, 14],
    [0, 4, 12, 8],
    [1, 5, 13, 9],
    [2, 6, 14, 10],
    [3, 7, 15, 11],
];

pub(super) const AXIS_CHARS: [char; 4] = ['X', 'Y', 'Z', 'W'];

#[cfg(test)]
pub(super) fn make_projector() -> StereoProjector {
    StereoProjector::new(
        egui::Pos2::new(200.0, 200.0),
        100.0,
        crate::render::DEFAULT_PROJECTION_DISTANCE,
        ProjectionMode::Perspective,
    )
}

#[cfg(test)]
pub(super) fn make_4d_rotated_camera() -> Camera {
    let mut cam = Camera::new();
    let rot = Rotation4D::from_6_plane_angles(0.37, -0.21, 0.44, 0.29, -0.18, 0.53);
    cam.set_rotation_4d(rot);
    cam
}
