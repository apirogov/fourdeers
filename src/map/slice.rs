use eframe::egui;
use nalgebra::Vector4;

use crate::camera::Camera;

use super::helpers::lerp_color;
use super::{DIM_GRAY, SLICE_GREEN};

pub(crate) struct SliceInfo {
    slice_normal: Vector4<f32>,
    cam_position: Vector4<f32>,
    w_half: f32,
}

impl SliceInfo {
    pub(crate) fn new(scene_camera: &Camera, w_thickness: f32) -> Self {
        let w = scene_camera.slice_rotation().basis_w();
        let slice_normal = Vector4::new(w[0], w[1], w[2], w[3]);
        Self {
            slice_normal,
            cam_position: scene_camera.position,
            w_half: w_thickness * 0.5,
        }
    }

    pub(crate) fn style_for_point(&self, world_pos: Vector4<f32>) -> (egui::Color32, f32) {
        let d = (world_pos - self.cam_position).dot(&self.slice_normal);
        let abs_d = d.abs();
        if abs_d <= self.w_half {
            (SLICE_GREEN, 1.0)
        } else if abs_d < 2.0 * self.w_half {
            let t = ((abs_d - self.w_half) / self.w_half).clamp(0.0, 1.0);
            let alpha = 1.0 - t * 0.7;
            let edge_color = lerp_color(SLICE_GREEN, DIM_GRAY, t);
            (edge_color, alpha)
        } else {
            (DIM_GRAY, 0.3)
        }
    }
}

pub(super) fn compute_slice_cross_section(
    vertices: &[Vector4<f32>],
    indices: &[u16],
    slice_normal: Vector4<f32>,
    slice_origin: Vector4<f32>,
) -> Vec<Vector4<f32>> {
    let mut points = Vec::new();
    for chunk in indices.chunks(2) {
        if chunk.len() != 2 {
            continue;
        }
        let p0 = vertices[chunk[0] as usize];
        let p1 = vertices[chunk[1] as usize];
        let d0 = (p0 - slice_origin).dot(&slice_normal);
        let d1 = (p1 - slice_origin).dot(&slice_normal);
        let denom = d1 - d0;
        if denom.abs() > 1e-10 {
            let t = -d0 / denom;
            if t > 0.0 && t < 1.0 {
                points.push(p0 + (p1 - p0) * t);
            }
        }
    }
    points
}

pub(super) fn compute_cross_section_edges(
    vertices: &[Vector4<f32>],
    faces: &[[u16; 4]],
    slice_normal: Vector4<f32>,
    slice_origin: Vector4<f32>,
) -> Vec<[Vector4<f32>; 2]> {
    let mut edges = Vec::new();
    for face in faces {
        let face_verts: [Vector4<f32>; 4] = [
            vertices[face[0] as usize],
            vertices[face[1] as usize],
            vertices[face[2] as usize],
            vertices[face[3] as usize],
        ];
        let distances: [f32; 4] = [
            (face_verts[0] - slice_origin).dot(&slice_normal),
            (face_verts[1] - slice_origin).dot(&slice_normal),
            (face_verts[2] - slice_origin).dot(&slice_normal),
            (face_verts[3] - slice_origin).dot(&slice_normal),
        ];
        let mut crossings: [Vector4<f32>; 4] = [Vector4::zeros(); 4];
        let mut crossing_count = 0usize;
        for i in 0..4 {
            let j = (i + 1) % 4;
            let di = distances[i];
            let dj = distances[j];
            if di.signum() != dj.signum() && (di - dj).abs() > 1e-10 {
                let t = di / (di - dj);
                let t = t.clamp(0.0, 1.0);
                crossings[crossing_count] = face_verts[i] + (face_verts[j] - face_verts[i]) * t;
                crossing_count += 1;
            }
        }
        if crossing_count == 2 {
            edges.push([crossings[0], crossings[1]]);
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use eframe::egui;
    use nalgebra::Vector4;

    use crate::camera::Camera;
    use crate::polytopes::{create_polytope, PolytopeType};
    use crate::render::{ProjectionMode, StereoProjector};
    use crate::rotation4d::Rotation4D;
    use crate::test_utils::assert_approx_eq;

    use super::super::{
        make_4d_rotated_camera, NEAR_MARGIN, TESSERACT_CROSS_SECTION_EDGE_COUNT,
        TESSERACT_CROSS_SECTION_VERTEX_COUNT, TESSERACT_EDGE_COUNT, TESSERACT_FACES,
    };
    use crate::camera::CameraProjection;

    use super::*;

    struct SliceSegment {
        p0: Vector4<f32>,
        p1: Vector4<f32>,
        fully_in: bool,
    }

    #[allow(clippy::cast_precision_loss)]
    fn compute_in_band_segments(
        vertices: &[Vector4<f32>],
        indices: &[u16],
        slice_normal: Vector4<f32>,
        slice_origin: Vector4<f32>,
        w_half: f32,
    ) -> Vec<SliceSegment> {
        let mut segments = Vec::new();
        for chunk in indices.chunks(2) {
            if chunk.len() != 2 {
                continue;
            }
            let p0 = vertices[chunk[0] as usize];
            let p1 = vertices[chunk[1] as usize];
            let d0 = (p0 - slice_origin).dot(&slice_normal);
            let d1 = (p1 - slice_origin).dot(&slice_normal);
            let denom = d1 - d0;
            let in0 = d0.abs() <= w_half;
            let in1 = d1.abs() <= w_half;
            if !in0 && !in1 {
                if d0.signum() != d1.signum() && denom.abs() > 1e-10 {
                    let t_enter = (w_half * d0.signum() - d0) / denom;
                    let t_exit = (w_half * d1.signum() - d0) / denom;
                    let t_min = t_enter.min(t_exit).clamp(0.0, 1.0);
                    let t_max = t_enter.max(t_exit).clamp(0.0, 1.0);
                    segments.push(SliceSegment {
                        p0: p0 + (p1 - p0) * t_min,
                        p1: p0 + (p1 - p0) * t_max,
                        fully_in: false,
                    });
                }
                continue;
            }
            let (tp0, tp1, fully_in) = if in0 && in1 {
                (p0, p1, true)
            } else {
                let outside_sign = if !in0 { d0.signum() } else { d1.signum() };
                let t = (w_half * outside_sign - d0) / denom;
                let t = t.clamp(0.0, 1.0);
                let clipped = p0 + (p1 - p0) * t;
                if !in0 {
                    (clipped, p1, false)
                } else {
                    (p0, clipped, false)
                }
            };
            segments.push(SliceSegment {
                p0: tp0,
                p1: tp1,
                fully_in,
            });
        }
        segments
    }

    fn snap_point(p: Vector4<f32>, resolution: f32) -> [i64; 4] {
        [
            (p[0] * resolution).round() as i64,
            (p[1] * resolution).round() as i64,
            (p[2] * resolution).round() as i64,
            (p[3] * resolution).round() as i64,
        ]
    }

    #[test]
    fn test_cross_section_default_w_slice_produces_cube() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        assert_eq!(
            cross.len(),
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            "w=0 tesseract cross-section should have {} vertices, got {}",
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            cross.len()
        );
        for pt in &cross {
            assert_approx_eq(pt[3], 0.0, 1e-6);
            for i in 0..3 {
                assert!(
                    pt[i].abs() <= 1.0 + 1e-6,
                    "xyz component should be in [-1,1]"
                );
            }
        }
    }

    #[test]
    fn test_in_band_segments_default_slice() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let w_half = 1.25;
        let segments =
            compute_in_band_segments(&vertices, &indices, slice_normal, slice_origin, w_half);
        assert!(
            segments.len() >= TESSERACT_CROSS_SECTION_EDGE_COUNT,
            "w=0 slice at w_half=1.25 should have >= {} edges, got {}",
            TESSERACT_CROSS_SECTION_EDGE_COUNT,
            segments.len()
        );
        let fully_in_count = segments.iter().filter(|s| s.fully_in).count();
        assert!(
            fully_in_count >= 4,
            "at least 4 edges should be fully in the slice band, got {}",
            fully_in_count
        );
    }

    #[test]
    fn test_in_band_segments_edges_lie_within_band() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let w_half = 1.25;
        let segments =
            compute_in_band_segments(&vertices, &indices, slice_normal, slice_origin, w_half);
        for seg in &segments {
            for p in &[seg.p0, seg.p1] {
                let d = (p - slice_origin).dot(&slice_normal);
                assert!(
                    d.abs() <= w_half + 1e-6,
                    "in-band segment endpoint should be within band: d={}, w_half={}",
                    d,
                    w_half
                );
            }
        }
    }

    #[test]
    fn test_near_margin_value() {
        const { assert!(NEAR_MARGIN > 0.3) };
    }

    #[test]
    fn test_tesseract_edge_and_cross_section_counts() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        assert_eq!(indices.len() / 2, TESSERACT_EDGE_COUNT);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        assert_eq!(cross.len(), TESSERACT_CROSS_SECTION_VERTEX_COUNT);
        let mut edge_count = 0usize;
        for i in 0..cross.len() {
            for j in (i + 1)..cross.len() {
                let diff_count = (0..3)
                    .filter(|&k| (cross[i][k] - cross[j][k]).abs() > 0.5)
                    .count();
                if diff_count == 1 {
                    edge_count += 1;
                }
            }
        }
        assert_eq!(edge_count, TESSERACT_CROSS_SECTION_EDGE_COUNT);
    }

    #[test]
    fn test_zero_w_slice_vertices_form_cube() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let mut distances = std::collections::HashSet::new();
        for i in 0..cross.len() {
            for j in (i + 1)..cross.len() {
                let d = (cross[i] - cross[j]).norm();
                let rounded = (d * 1000.0).round() as i64;
                distances.insert(rounded);
            }
        }
        assert!(
            distances.len() <= 3,
            "cube should have at most 3 distinct edge lengths (side, face diagonal, space diagonal), got {}",
            distances.len()
        );
        let mut side_count = 0usize;
        for i in 0..cross.len() {
            for v in cross.iter().skip(i + 1) {
                let d = (cross[i] - v).norm();
                let rounded = (d * 1000.0).round();
                if rounded == 2000.0 {
                    side_count += 1;
                }
            }
        }
        let edges_per_vertex =
            2.0 * side_count as f32 / TESSERACT_CROSS_SECTION_VERTEX_COUNT as f32;
        assert!(
            (edges_per_vertex - 3.0).abs() < 0.1,
            "each cube vertex should have degree 3, got {}",
            edges_per_vertex
        );
    }

    #[test]
    fn test_8_cell_structure_invariants() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        assert_eq!(vertices.len(), 16);
        assert_eq!(indices.len() / 2, TESSERACT_EDGE_COUNT);
        let mut degrees = vec![0u16; vertices.len()];
        for chunk in indices.chunks(2) {
            if chunk.len() == 2 {
                degrees[chunk[0] as usize] += 1;
                degrees[chunk[1] as usize] += 1;
            }
        }
        for (i, &d) in degrees.iter().enumerate() {
            assert_eq!(d, 4, "tesseract vertex {} should have degree 4", i);
        }
    }

    #[test]
    fn test_cross_section_edges_from_faces_w0_is_cube() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        assert_eq!(
            cs_edges.len(),
            TESSERACT_CROSS_SECTION_EDGE_COUNT,
            "w=0 cross-section should have 12 edges"
        );
        let mut vertex_counts: std::collections::HashMap<[i64; 4], u32> =
            std::collections::HashMap::new();
        let resolution = 1000.0;
        for [p0, p1] in &cs_edges {
            *vertex_counts
                .entry(snap_point(*p0, resolution))
                .or_insert(0) += 1;
            *vertex_counts
                .entry(snap_point(*p1, resolution))
                .or_insert(0) += 1;
        }
        assert_eq!(
            vertex_counts.len(),
            TESSERACT_CROSS_SECTION_VERTEX_COUNT,
            "w=0 cross-section should have 8 unique vertices"
        );
        for (key, &deg) in &vertex_counts {
            assert_eq!(
                deg, 3,
                "cube vertex {:?} should have degree 3, got {}",
                key, deg
            );
        }
    }

    #[test]
    fn test_cross_section_edges_match_hull_under_4d_map_rotation() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let map_camera = make_4d_rotated_camera();
        let map_transform = CameraProjection::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_screen: Vec<egui::Pos2> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project(*p).0;
                if p3.z > near_z {
                    proj.project_3d(p3.x, p3.y, p3.z)
                } else {
                    None
                }
            })
            .map(|p| p.screen_pos)
            .collect();
        for [p0, p1] in &cs_edges {
            let s0 = map_transform.project(*p0).0;
            let s1 = map_transform.project(*p1).0;
            if s0.z <= near_z || s1.z <= near_z {
                continue;
            }
            let Some(sp0) = proj.project_3d(s0.x, s0.y, s0.z) else {
                continue;
            };
            let Some(sp1) = proj.project_3d(s1.x, s1.y, s1.z) else {
                continue;
            };
            let mut found0 = false;
            let mut found1 = false;
            for &cp in &cross_screen {
                if (cp - sp0.screen_pos).length() < 1.0 {
                    found0 = true;
                }
                if (cp - sp1.screen_pos).length() < 1.0 {
                    found1 = true;
                }
            }
            assert!(
                found0,
                "edge endpoint {:?} should match a cross-section screen point (4D rotated map)",
                sp0.screen_pos
            );
            assert!(
                found1,
                "edge endpoint {:?} should match a cross-section screen point (4D rotated map)",
                sp1.screen_pos
            );
        }
    }

    #[test]
    fn test_cross_section_edges_with_tilted_slice() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(1.0, 0.5, -0.3, 1.0).normalize();
        let slice_origin = Vector4::new(0.15, -0.1, 0.05, 0.2);
        let cross = compute_slice_cross_section(&vertices, &_indices, slice_normal, slice_origin);
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        assert!(
            cross.len() >= 4,
            "tilted slice should produce >= 4 vertices, got {}",
            cross.len()
        );
        assert!(
            cs_edges.len() >= 6,
            "tilted slice should produce >= 6 edges, got {}",
            cs_edges.len()
        );
        let resolution = 1000.0;
        let mut vertex_degrees: std::collections::HashMap<[i64; 4], u32> =
            std::collections::HashMap::new();
        for [p0, p1] in &cs_edges {
            *vertex_degrees
                .entry(snap_point(*p0, resolution))
                .or_insert(0) += 1;
            *vertex_degrees
                .entry(snap_point(*p1, resolution))
                .or_insert(0) += 1;
        }
        for (key, &deg) in &vertex_degrees {
            assert!(
                deg >= 3,
                "tilted slice vertex {:?} should have degree >= 3, got {}",
                key,
                deg
            );
        }
    }

    #[test]
    fn test_style_for_point_at_camera_is_in_slab() {
        let cam = Camera::new();
        let info = SliceInfo::new(&cam, 2.5);
        let (color, alpha) = info.style_for_point(cam.position);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_in_slab_small_w_offset() {
        let cam = Camera::new();
        let info = SliceInfo::new(&cam, 2.5);
        let pos_nearby = cam.position + Vector4::new(0.0, 0.0, 0.0, 0.5);
        let (color, alpha) = info.style_for_point(pos_nearby);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_near_slab_lerps() {
        let cam = Camera::new();
        let info = SliceInfo::new(&cam, 2.5);
        let w_half = 1.25;
        let pos_near = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half + 0.5 * w_half);
        let (color, alpha) = info.style_for_point(pos_near);
        assert_ne!(color, SLICE_GREEN);
        assert_ne!(color, DIM_GRAY);
        assert!(
            alpha > 0.3 && alpha < 1.0,
            "alpha should be between 0.3 and 1.0 for near-slab, got {}",
            alpha
        );
    }

    #[test]
    fn test_style_for_point_far_from_slab() {
        let cam = Camera::new();
        let info = SliceInfo::new(&cam, 2.5);
        let pos_far = cam.position + Vector4::new(0.0, 0.0, 0.0, 20.0);
        let (color, alpha) = info.style_for_point(pos_far);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_far_negative_w() {
        let cam = Camera::new();
        let info = SliceInfo::new(&cam, 2.5);
        let pos_far_neg = cam.position + Vector4::new(0.0, 0.0, 0.0, -20.0);
        let (color, alpha) = info.style_for_point(pos_far_neg);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_boundary_at_w_half() {
        let cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let pos_boundary = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half);
        let (color, alpha) = info.style_for_point(pos_boundary);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
    }

    #[test]
    fn test_style_for_point_boundary_at_2w_half() {
        let cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let pos_boundary = cam.position + Vector4::new(0.0, 0.0, 0.0, 2.0 * w_half);
        let (color, alpha) = info.style_for_point(pos_boundary);
        assert_eq!(color, DIM_GRAY);
        assert_approx_eq(alpha, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_with_tilted_slice() {
        let mut cam = Camera::new();
        cam.set_rotation_4d(Rotation4D::from_6_plane_angles(
            0.0, 0.0, 0.0, 0.5, 0.0, 0.0,
        ));
        let info = SliceInfo::new(&cam, 2.5);
        let (color, alpha) = info.style_for_point(cam.position);
        assert_eq!(color, SLICE_GREEN);
        assert_approx_eq(alpha, 1.0, 1e-6);
        let pos_far = cam.position + Vector4::new(0.0, 0.0, 0.0, 20.0);
        let (color_far, alpha_far) = info.style_for_point(pos_far);
        assert_eq!(color_far, DIM_GRAY);
        assert_approx_eq(alpha_far, 0.3, 1e-6);
    }

    #[test]
    fn test_style_for_point_lerp_is_continuous() {
        let cam = Camera::new();
        let w_thickness = 2.5;
        let info = SliceInfo::new(&cam, w_thickness);
        let w_half = w_thickness * 0.5;
        let epsilon = 0.01;
        let pos_just_inside = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half - epsilon);
        let pos_just_outside = cam.position + Vector4::new(0.0, 0.0, 0.0, w_half + epsilon);
        let (color_in, alpha_in) = info.style_for_point(pos_just_inside);
        let (color_out, alpha_out) = info.style_for_point(pos_just_outside);
        let color_dist = ((color_in.r() as i32 - color_out.r() as i32).abs()
            + (color_in.g() as i32 - color_out.g() as i32).abs()
            + (color_in.b() as i32 - color_out.b() as i32).abs()) as f32;
        assert!(
            color_dist < 15.0,
            "color should be nearly continuous at w_half boundary, distance={}",
            color_dist
        );
        assert!(
            (alpha_in - alpha_out).abs() < 0.1,
            "alpha should be nearly continuous at w_half boundary: in={}, out={}",
            alpha_in,
            alpha_out
        );
    }

    #[test]
    fn test_cross_section_edges_project_consistently_with_map_transform() {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.7, -0.3, 0.5, 1.0).normalize();
        let slice_origin = Vector4::new(0.1, -0.05, 0.2, 0.15);
        let cross = compute_slice_cross_section(&vertices, &indices, slice_normal, slice_origin);
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let map_camera = make_4d_rotated_camera();
        let map_transform = CameraProjection::new(&map_camera);
        let proj = StereoProjector::new(
            egui::Pos2::new(200.0, 200.0),
            100.0,
            3.0,
            ProjectionMode::Perspective,
        );
        let near_z = -3.0 + NEAR_MARGIN;
        let cross_screen: Vec<egui::Pos2> = cross
            .iter()
            .filter_map(|p| {
                let p3 = map_transform.project(*p).0;
                if p3.z > near_z {
                    proj.project_3d(p3.x, p3.y, p3.z)
                } else {
                    None
                }
            })
            .map(|p| p.screen_pos)
            .collect();
        for [p0, p1] in &cs_edges {
            let s0 = map_transform.project(*p0).0;
            let s1 = map_transform.project(*p1).0;
            if s0.z <= near_z || s1.z <= near_z {
                continue;
            }
            let Some(sp0) = proj.project_3d(s0.x, s0.y, s0.z) else {
                continue;
            };
            let Some(sp1) = proj.project_3d(s1.x, s1.y, s1.z) else {
                continue;
            };
            let mut found0 = false;
            let mut found1 = false;
            for &cp in &cross_screen {
                if (cp - sp0.screen_pos).length() < 2.0 {
                    found0 = true;
                }
                if (cp - sp1.screen_pos).length() < 2.0 {
                    found1 = true;
                }
            }
            assert!(
                found0,
                "edge endpoint screen {:?} should match a cross-section screen point (tilted slice + 4D map)",
                sp0.screen_pos
            );
            assert!(
                found1,
                "edge endpoint screen {:?} should match a cross-section screen point (tilted slice + 4D map)",
                sp1.screen_pos
            );
        }
    }
}
