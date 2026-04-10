use eframe::egui;
use nalgebra::{Vector3, Vector4};

use crate::camera::Camera;
use crate::geometry::{convex_hull_2d, ConvexPolyhedron, VertexDedup};
use crate::render::{StereoProjector, StereoSettings};

use super::bounds::direction_to_tesseract;
use super::CameraProjection;

pub(crate) fn build_cross_section_polyhedron(
    cs_edges: &[[Vector4<f32>; 2]],
    map_transform: &CameraProjection,
) -> ConvexPolyhedron {
    let mut dedup = VertexDedup::new();
    let mut edges: Vec<[usize; 2]> = Vec::new();
    for [p0_4d, p1_4d] in cs_edges {
        let v0 = map_transform.project(*p0_4d).0;
        let v1 = map_transform.project(*p1_4d).0;
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
    map_transform: &CameraProjection,
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
        let dir_map_3d = map_transform.project_direction(dir_tess);
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
    map_transform: &CameraProjection,
    projector: &StereoProjector,
    near_z: f32,
    p0: Vector4<f32>,
    p1: Vector4<f32>,
) -> Option<(egui::Pos2, egui::Pos2)> {
    let mut s0 = map_transform.project(p0).0;
    let mut s1 = map_transform.project(p1).0;
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

    use crate::camera::{Camera, CameraProjection};
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
        let mt = CameraProjection::new(&Camera::new());
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
        let map_transform = CameraProjection::new(&map_camera);
        let poly = build_cross_section_polyhedron(&cs_edges, &map_transform);
        assert_eq!(poly.vertices.len(), TESSERACT_CROSS_SECTION_VERTEX_COUNT);
        assert_eq!(poly.edges.len(), TESSERACT_CROSS_SECTION_EDGE_COUNT);
    }

    #[test]
    fn test_frustum_ray_directions_identity() {
        let scene_camera = Camera::new();
        let map_camera = Camera::new();
        let map_transform = CameraProjection::new(&map_camera);
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
        let map_transform = CameraProjection::new(&map_camera);
        let scene_camera = Camera::new();
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
        let cam_3d = map_transform.project(norm_cam).0;
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
        let map_transform = CameraProjection::new(&map_camera);
        let mut scene_camera = Camera::new();
        scene_camera.rotate(0.5, 0.3);
        let bounds = Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );
        let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
        let cam_3d = map_transform.project(norm_cam).0;
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

    #[test]
    fn test_project_direction_no_offset_at_origin() {
        let mut cam = Camera::new();
        cam.position = Vector4::zeros();
        let proj = CameraProjection::new(&cam);
        let dir = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let pos_result = proj.project(dir).0;
        let dir_result = proj.project_direction(dir);
        assert_approx_eq(pos_result.x, dir_result.x, 1e-6);
        assert_approx_eq(pos_result.y, dir_result.y, 1e-6);
        assert_approx_eq(pos_result.z, dir_result.z, 1e-6);
    }

    #[test]
    fn test_forward_direction_points_at_origin_while_orbiting() {
        use crate::geometry::Bounds4D;
        use crate::map::bounds::{compute_bounds, direction_to_tesseract, normalize_to_tesseract};
        use crate::toy::CompassWaypoint;

        let geometry_bounds = Some(Bounds4D::from_corners(
            Vector4::new(-1.0, -1.0, -1.0, -1.0),
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        ));
        let map_camera = Camera::new();
        let map_transform = CameraProjection::new(&map_camera);

        let orbit_radius = 5.0f32;
        let steps = 12;
        for step in 0..steps {
            let angle = 2.0 * std::f32::consts::PI * step as f32 / steps as f32;
            let cam_x = orbit_radius * angle.sin();
            let cam_z = -orbit_radius * angle.cos();

            let mut scene_camera = Camera::new();
            scene_camera.position = Vector4::new(cam_x, 0.0, cam_z, 0.0);
            let yaw_to_origin = (-cam_x).atan2(-cam_z);
            scene_camera.set_yaw_pitch_l(yaw_to_origin, 0.0);

            let waypoints: Vec<CompassWaypoint> = vec![];
            let bounds = compute_bounds(&scene_camera, &waypoints, geometry_bounds);

            let norm_cam = normalize_to_tesseract(scene_camera.position, &bounds);
            let cam_3d = map_transform.project(norm_cam).0;

            let norm_origin = normalize_to_tesseract(Vector4::zeros(), &bounds);
            let origin_3d = map_transform.project(norm_origin).0;

            let to_origin = origin_3d - cam_3d;
            let to_origin_len = to_origin.norm();
            if to_origin_len < 1e-6 {
                continue;
            }
            let to_origin_dir = to_origin / to_origin_len;

            let forward_4d =
                scene_camera.project_camera_3d_to_world_4d(scene_camera.forward_vector());
            let forward_tess = direction_to_tesseract(forward_4d, &bounds);
            let forward_3d = map_transform.project_direction(forward_tess);
            let forward_len = forward_3d.norm();
            assert!(
                forward_len > 1e-10,
                "forward direction should be non-zero at step {}",
                step
            );
            let forward_dir = forward_3d / forward_len;

            let dot = to_origin_dir.dot(&forward_dir);
            assert!(
                dot > 0.99,
                "forward arrow should point at origin at angle {:.1}° (step {}): \
                 dot={:.6}, to_origin_dir={:?}, forward_dir={:?}, cam=({:.2},{:.2}), \
                 cam_3d={:?}, origin_3d={:?}, bounds_z=[{:.2},{:.2}]",
                angle.to_degrees(),
                step,
                dot,
                to_origin_dir,
                forward_dir,
                cam_x,
                cam_z,
                cam_3d,
                origin_3d,
                bounds.min[2],
                bounds.max[2],
            );
        }
    }
}
