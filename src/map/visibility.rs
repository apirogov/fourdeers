use eframe::egui;
use nalgebra::{Vector3, Vector4};

use crate::camera::Camera;
use crate::geometry::{convex_hull_2d, ConvexPolyhedron, VertexDedup};
use crate::render::{StereoProjector, StereoSettings};

use super::bounds::direction_to_tesseract;
use super::transform::MapViewTransform;

pub(crate) fn build_cross_section_polyhedron(
    cs_edges: &[[Vector4<f32>; 2]],
    map_transform: &MapViewTransform,
) -> ConvexPolyhedron {
    let mut dedup = VertexDedup::new();
    let mut edges: Vec<[usize; 2]> = Vec::new();
    for [p0_4d, p1_4d] in cs_edges {
        let v0 = map_transform.project_to_3d(*p0_4d);
        let v1 = map_transform.project_to_3d(*p1_4d);
        let i0 = dedup.find_or_add(v0);
        let i1 = dedup.find_or_add(v1);
        if i0 != i1 {
            edges.push([i0, i1]);
        }
    }
    ConvexPolyhedron {
        vertices: dedup.vertices,
        edges,
    }
}

pub(crate) fn compute_frustum_rays(
    scene_camera: &Camera,
    view_rect: egui::Rect,
    stereo: StereoSettings,
    bounds: &crate::geometry::Bounds4D,
    map_transform: &MapViewTransform,
) -> [Vector3<f32>; 4] {
    let scale =
        view_rect.height().min(view_rect.width() * 0.5) * crate::render::STEREO_SCALE_FACTOR;
    let cx = view_rect.center().x;
    let cy = view_rect.center().y;
    let pd = stereo.projection_distance;
    let corners = [
        Vector3::new(
            (view_rect.left() - cx) / scale,
            (cy - view_rect.top()) / scale,
            pd,
        ),
        Vector3::new(
            (view_rect.right() - cx) / scale,
            (cy - view_rect.top()) / scale,
            pd,
        ),
        Vector3::new(
            (view_rect.right() - cx) / scale,
            (cy - view_rect.bottom()) / scale,
            pd,
        ),
        Vector3::new(
            (view_rect.left() - cx) / scale,
            (cy - view_rect.bottom()) / scale,
            pd,
        ),
    ];
    let mut rays = [Vector3::zeros(); 4];
    let q_left = scene_camera.rotation_4d.q_left();
    for (i, dir_local) in corners.iter().enumerate() {
        let dir_3d = q_left.transform_vector(dir_local);
        let dir_4d = scene_camera.project_camera_3d_to_world_4d(dir_3d);
        let dir_tess = direction_to_tesseract(dir_4d, bounds);
        let dir_map_3d = map_transform.direction_to_3d(dir_tess);
        let len = dir_map_3d.norm();
        rays[i] = if len > 1e-10 {
            dir_map_3d / len
        } else {
            dir_map_3d
        };
    }
    rays
}

pub(crate) fn compute_frustum_planes(
    rays: &[Vector3<f32>; 4],
    cam_3d: Vector3<f32>,
) -> [(Vector3<f32>, Vector3<f32>); 4] {
    let forward = (rays[0] + rays[1] + rays[2] + rays[3]) * 0.25;
    let mut planes = [(Vector3::zeros(), Vector3::zeros()); 4];
    for i in 0..4 {
        let j = (i + 1) % 4;
        let mut normal = rays[i].cross(&rays[j]);
        let len = normal.norm();
        if len > 1e-10 {
            normal /= len;
        }
        if normal.dot(&forward) < 0.0 {
            normal = -normal;
        }
        planes[i] = (cam_3d, normal);
    }
    planes
}

pub(super) fn clip_segment_to_screen(
    map_transform: &MapViewTransform,
    projector: &StereoProjector,
    near_z: f32,
    p0: Vector4<f32>,
    p1: Vector4<f32>,
) -> Option<(egui::Pos2, egui::Pos2)> {
    let mut s0 = map_transform.project_to_3d(p0);
    let mut s1 = map_transform.project_to_3d(p1);
    let in0 = s0.z > near_z;
    let in1 = s1.z > near_z;
    if !in0 && !in1 {
        return None;
    }
    if !in0 || !in1 {
        let dz = s1.z - s0.z;
        if dz.abs() < 1e-10 {
            return None;
        }
        let t = (near_z - s0.z) / dz;
        let clipped = s0 + (s1 - s0) * t;
        if in0 {
            s1 = clipped;
        } else {
            s0 = clipped;
        }
    }
    let sp0 = projector.project_3d(s0.x, s0.y, s0.z)?;
    let sp1 = projector.project_3d(s1.x, s1.y, s1.z)?;
    Some((sp0.screen_pos, sp1.screen_pos))
}

pub(super) fn convex_hull_screen(
    pts_3d: &[Vector3<f32>],
    projector: &StereoProjector,
) -> Vec<egui::Pos2> {
    let pts_2d: Vec<egui::Pos2> = pts_3d
        .iter()
        .filter_map(|v3| projector.project_3d(v3.x, v3.y, v3.z))
        .map(|p| p.screen_pos)
        .collect();
    convex_hull_2d(&pts_2d)
}

#[cfg(test)]
mod tests {
    use eframe::egui;
    use nalgebra::{Vector3, Vector4};

    use crate::camera::Camera;
    use crate::geometry::{clip_polyhedron_by_plane, Bounds4D};
    use crate::polytopes::{create_polytope, PolytopeType};
    use crate::render::StereoSettings;
    use crate::test_utils::assert_approx_eq;

    use super::super::bounds::normalize_to_tesseract;
    use super::super::slice::compute_cross_section_edges;
    use super::super::{
        make_projector, NEAR_MARGIN, TESSERACT_CROSS_SECTION_EDGE_COUNT,
        TESSERACT_CROSS_SECTION_VERTEX_COUNT, TESSERACT_FACES,
    };

    use super::*;

    #[test]
    fn test_clip_segment_both_in_front() {
        let mt = MapViewTransform::new(&Camera::new());
        let proj = make_projector();
        let near_z = -3.0 + NEAR_MARGIN;
        let p0 = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let p1 = Vector4::new(1.0, 0.0, 0.0, 0.0);
        assert!(clip_segment_to_screen(&mt, &proj, near_z, p0, p1).is_some());
    }

    #[test]
    fn test_convex_hull_preserves_count() {
        let proj = make_projector();
        let pts = vec![
            Vector3::new(0.5, 0.5, 1.0),
            Vector3::new(-0.5, 0.5, 1.0),
            Vector3::new(-0.5, -0.5, 1.0),
            Vector3::new(0.5, -0.5, 1.0),
        ];
        assert_eq!(convex_hull_screen(&pts, &proj).len(), 4);
    }

    #[test]
    fn test_build_cross_section_polyhedron_cube() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let poly = build_cross_section_polyhedron(&cs_edges, &map_transform);
        assert_eq!(poly.vertices.len(), TESSERACT_CROSS_SECTION_VERTEX_COUNT);
        assert_eq!(poly.edges.len(), TESSERACT_CROSS_SECTION_EDGE_COUNT);
    }

    #[test]
    fn test_frustum_ray_directions_identity() {
        let scene_camera = Camera::new();
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let view_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 400.0));
        let stereo = StereoSettings::default();
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let rays = compute_frustum_rays(&scene_camera, view_rect, stereo, &bounds, &map_transform);
        let avg_z = (rays[0].z + rays[1].z + rays[2].z + rays[3].z) * 0.25;
        assert!(
            avg_z > 0.0,
            "average z should be positive (pointing forward), got {}",
            avg_z
        );
        for ray in &rays {
            assert_approx_eq(ray.norm(), 1.0, 1e-6);
        }
    }

    #[test]
    fn test_visibility_cone_3d_identity_cam() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let scene_camera = Camera::new();
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
        let cam_3d = map_transform.project_to_3d(norm_cam);
        let near_z = -3.0 + NEAR_MARGIN;
        if cam_3d.z <= near_z {
            return;
        }
        let poly = build_cross_section_polyhedron(&cs_edges, &map_transform);
        assert!(poly.vertices.len() >= 3);
        let view_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 400.0));
        let rays = compute_frustum_rays(
            &scene_camera,
            view_rect,
            StereoSettings::default(),
            &bounds,
            &map_transform,
        );
        let planes = compute_frustum_planes(&rays, cam_3d);
        let mut clipped = poly;
        for (pp, pn) in &planes {
            clipped = clip_polyhedron_by_plane(&clipped, *pp, *pn);
            if clipped.vertices.is_empty() {
                break;
            }
        }
        assert!(
            clipped.vertices.len() >= 3,
            "visibility cone should have >= 3 vertices with identity camera, got {}",
            clipped.vertices.len()
        );
    }

    #[test]
    fn test_visibility_cone_3d_rotated_cam() {
        let (vertices, _indices) = create_polytope(PolytopeType::EightCell);
        let slice_normal = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let slice_origin = Vector4::zeros();
        let cs_edges =
            compute_cross_section_edges(&vertices, &TESSERACT_FACES, slice_normal, slice_origin);
        let map_camera = Camera::new();
        let map_transform = MapViewTransform::new(&map_camera);
        let mut scene_camera = Camera::new();
        scene_camera.rotate(0.5, 0.3);
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
        let cam_3d = map_transform.project_to_3d(norm_cam);
        let near_z = -3.0 + NEAR_MARGIN;
        if cam_3d.z <= near_z {
            return;
        }
        let poly = build_cross_section_polyhedron(&cs_edges, &map_transform);
        assert!(poly.vertices.len() >= 3);
        let view_rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 400.0));
        let rays = compute_frustum_rays(
            &scene_camera,
            view_rect,
            StereoSettings::default(),
            &bounds,
            &map_transform,
        );
        let planes = compute_frustum_planes(&rays, cam_3d);
        let mut clipped = poly;
        for (pp, pn) in &planes {
            clipped = clip_polyhedron_by_plane(&clipped, *pp, *pn);
            if clipped.vertices.is_empty() {
                break;
            }
        }
        assert!(
            clipped.vertices.len() >= 3,
            "visibility cone should have >= 3 vertices with rotated camera, got {}",
            clipped.vertices.len()
        );
    }
}
