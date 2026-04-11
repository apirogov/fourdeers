use eframe::egui;
use nalgebra::{Vector3, Vector4};

use crate::camera::{Camera, CameraProjection};
use crate::colors::ARROW_FORWARD;
use crate::geometry::{clip_polyhedron_by_plane, Bounds4D, ConvexPolyhedron};
use crate::polytopes::{create_polytope, PolytopeType};
use crate::render::{
    batch::LineBatch, create_stereo_projectors, draw_arrow_head, draw_background,
    draw_center_divider, CompassFrameMode, FourDSettings, ProjectionMode, StereoProjector,
    StereoSettings, TesseractRenderConfig, TesseractRenderContext, TransformedVertex,
};
use crate::tetrahedron::{compute_component_color, format_magnitude, TetrahedronGadget};
use crate::toy::CompassWaypoint;

use super::bounds::{compute_bounds, direction_to_tesseract, normalize_to_tesseract};
use super::helpers::{edge_axis, render_tetrahedron_in_map};
use super::slice::{compute_cross_section_edges, compute_slice_cross_section, SliceInfo};
use super::visibility::{
    build_cross_section_polyhedron, clip_segment_to_screen, compute_frustum_planes,
    compute_frustum_rays, convex_hull_screen,
};
use super::{
    AXIS_CHARS, FORWARD_ARROW_LENGTH, MAP_AXIS_FONT_SIZE, MAP_AXIS_LABEL_OFFSET_Y,
    MAP_CAMERA_BACK_OFFSET, MAP_CAMERA_DOT_RADIUS, MAP_DISTANCE_FONT_SIZE,
    MAP_DISTANCE_LABEL_OFFSET_Y, MAP_EDGE_LABEL_OFFSET, MAP_WAYPOINT_DOT_RADIUS, NEAR_MARGIN,
    SLICE_GREEN, TAP_RADIUS_MAX, TAP_RADIUS_MIN, TAP_RADIUS_MULTIPLIER, TESSERACT_FACES,
    TETRA_SCALE_CAMERA, TETRA_SCALE_WAYPOINT, VISIBILITY_DARK_GREEN,
};

const SLICE_FILL_STROKE_WIDTH: f32 = 1.5;
const SLICE_EDGE_STROKE_WIDTH: f32 = 2.0;
const FORWARD_ARROW_STROKE_WIDTH: f32 = 2.0;
const FORWARD_ARROW_HEAD_SIZE: f32 = 10.0;

pub struct MapRenderParams<'a> {
    pub scene_camera: &'a Camera,
    pub waypoints: &'a [CompassWaypoint],
    pub stereo: StereoSettings,
    pub frame_mode: CompassFrameMode,
    pub geometry_bounds: Option<Bounds4D>,
}

struct PreparedWaypoint {
    s3d: Vector3<f32>,
    edge_color: egui::Color32,
    alpha: f32,
    gadget: TetrahedronGadget,
    dist_label: String,
}

struct PreparedCamera {
    s3d: Vector3<f32>,
    edge_color: egui::Color32,
    alpha: f32,
    gadget: TetrahedronGadget,
    tip_3d: Vector3<f32>,
}

struct SliceVolumeData {
    cross_section_3d: Vec<Vector3<f32>>,
    cs_edges: Vec<[Vector4<f32>; 2]>,
    near_z: f32,
    poly: Option<ConvexPolyhedron>,
    cam_3d: Vector3<f32>,
}

struct MapFrameData<'a> {
    tesseract_ctx: TesseractRenderContext<'a>,
    transformed: Vec<TransformedVertex>,
    slice: SliceVolumeData,
    waypoints: Vec<PreparedWaypoint>,
    camera: Option<PreparedCamera>,
}

pub struct MapRenderer {
    camera: Camera,
    tesseract_vertices: Vec<Vector4<f32>>,
    tesseract_indices: Vec<u16>,
    w_thickness: f32,
    w_color_intensity: f32,
    projection_distance: f32,
    labels_visible: bool,
    waypoint_tap_zones: Vec<(egui::Pos2, egui::Pos2, f32, usize)>,
}

impl Default for MapRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MapRenderer {
    #[must_use]
    pub fn new() -> Self {
        let (vertices, indices) = create_polytope(PolytopeType::EightCell);
        Self {
            camera: Camera::new(),
            tesseract_vertices: vertices,
            tesseract_indices: indices,
            w_thickness: crate::render::DEFAULT_W_THICKNESS,
            w_color_intensity: crate::render::DEFAULT_W_COLOR_INTENSITY,
            projection_distance: crate::render::DEFAULT_PROJECTION_DISTANCE,
            labels_visible: false,
            waypoint_tap_zones: Vec::new(),
        }
    }

    pub fn sync_settings(&mut self, four_d: &FourDSettings, projection_distance: f32) {
        self.w_thickness = four_d.w_thickness;
        self.w_color_intensity = four_d.w_color_intensity;
        self.projection_distance = projection_distance;
    }

    pub const fn toggle_labels(&mut self) {
        self.labels_visible = !self.labels_visible;
    }

    #[must_use]
    pub const fn labels_visible(&self) -> bool {
        self.labels_visible
    }

    pub fn apply_action(&mut self, action: crate::camera::Direction4D, speed: f32) {
        self.camera.apply_action(action, speed);
    }

    pub fn rotate_3d(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate(delta_x, delta_y);
    }

    pub fn rotate_4d(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate_4d(delta_x, delta_y);
    }

    pub fn reset_to_fit(&mut self, scene_camera: &Camera, bounds: &Bounds4D) {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let q_left = *scene_camera.rotation_4d().q_left();
        let offset_local = Vector3::new(0.0, 0.0, -MAP_CAMERA_BACK_OFFSET);
        let rotated_offset = q_left.transform_vector(&offset_local);
        self.camera.position =
            norm_cam + Vector4::new(rotated_offset[0], rotated_offset[1], rotated_offset[2], 0.0);
        self.camera
            .set_yaw_pitch_l(scene_camera.yaw_l(), scene_camera.pitch_l());
        self.camera.set_yaw_r(scene_camera.yaw_r());
        self.camera.set_pitch_r(scene_camera.pitch_r());
    }

    pub fn render(&mut self, ui: &mut egui::Ui, rect: egui::Rect, params: &MapRenderParams<'_>) {
        draw_background(ui, rect);
        draw_center_divider(ui, rect);
        let bounds = compute_bounds(
            params.scene_camera,
            params.waypoints,
            params.geometry_bounds,
        );
        let map_transform = CameraProjection::new(&self.camera);
        let frame_data =
            self.prepare_frame_data(&map_transform, params.scene_camera, &bounds, params);

        let views = create_stereo_projectors(
            rect,
            params.stereo.eye_separation,
            params.stereo.projection_distance,
            ProjectionMode::Perspective,
        );
        let left_painter = ui.painter().with_clip_rect(views.left_rect);
        let right_painter = ui.painter().with_clip_rect(views.right_rect);
        for (painter, projector, view_rect) in [
            (&left_painter, &views.left_projector, views.left_rect),
            (&right_painter, &views.right_projector, views.right_rect),
        ] {
            frame_data.tesseract_ctx.render_edges(
                painter,
                projector,
                &frame_data.transformed,
                painter.clip_rect(),
            );
            if self.labels_visible {
                self.render_vertex_labels(painter, projector, &frame_data.transformed);
                self.render_edge_labels(painter, projector, &frame_data.transformed);
            }
            let mut batch = LineBatch::new(SLICE_EDGE_STROKE_WIDTH);
            self.draw_slice_volume(
                &mut batch,
                projector,
                &map_transform,
                &frame_data.slice,
                view_rect,
                params.stereo,
                params.scene_camera,
                &bounds,
            );
            self.draw_waypoints(&mut batch, painter, projector, &frame_data.waypoints);
            if let Some(cam) = frame_data.camera.as_ref() {
                self.draw_camera_position(&mut batch, painter, projector, cam);
            }
            batch.submit(painter);
        }
        self.compute_waypoint_tap_zones(
            &views.left_projector,
            &views.right_projector,
            &frame_data.waypoints,
        );
    }

    fn prepare_frame_data<'a>(
        &'a self,
        map_transform: &CameraProjection,
        scene_camera: &Camera,
        bounds: &Bounds4D,
        params: &MapRenderParams<'_>,
    ) -> MapFrameData<'a> {
        let tesseract_ctx = self.build_tesseract_context(map_transform);
        let transformed = tesseract_ctx.transform_vertices();
        let slice = self.prepare_slice_volume(map_transform, scene_camera, bounds);
        let waypoints = self.prepare_waypoints(map_transform, scene_camera, bounds, params);
        let camera = self.prepare_camera(map_transform, scene_camera, bounds, params.frame_mode);
        MapFrameData {
            tesseract_ctx,
            transformed,
            slice,
            waypoints,
            camera,
        }
    }

    fn build_tesseract_context<'a>(
        &'a self,
        map_transform: &CameraProjection,
    ) -> TesseractRenderContext<'a> {
        let config = TesseractRenderConfig {
            four_d: FourDSettings {
                w_thickness: self.w_thickness,
                w_color_intensity: self.w_color_intensity,
            },
            stereo: StereoSettings::new().with_projection_distance(self.projection_distance),
        };
        TesseractRenderContext::from_config(
            &self.tesseract_vertices,
            &self.tesseract_indices,
            &self.camera,
            map_transform.clone(),
            config,
        )
    }

    fn prepare_slice_volume(
        &self,
        map_transform: &CameraProjection,
        scene_camera: &Camera,
        bounds: &Bounds4D,
    ) -> SliceVolumeData {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let w = scene_camera.slice_rotation().basis_w();
        let slice_normal = Vector4::new(w[0], w[1], w[2], w[3]);
        let near_z = -self.projection_distance + NEAR_MARGIN;
        let cross_section_4d = compute_slice_cross_section(
            &self.tesseract_vertices,
            &self.tesseract_indices,
            slice_normal,
            norm_cam,
        );
        let cross_section_3d: Vec<Vector3<f32>> = cross_section_4d
            .iter()
            .filter_map(|p4d| {
                let pt3d = map_transform.project(*p4d).0;
                if pt3d.z > near_z {
                    Some(pt3d)
                } else {
                    None
                }
            })
            .collect();
        let cs_edges = compute_cross_section_edges(
            &self.tesseract_vertices,
            &TESSERACT_FACES,
            slice_normal,
            norm_cam,
        );
        let cam_3d = map_transform.project(norm_cam).0;
        let poly = if cross_section_3d.len() >= 3 && cam_3d.z > near_z {
            let poly = build_cross_section_polyhedron(&cs_edges, map_transform);
            if poly.vertices.len() >= 3 {
                Some(poly)
            } else {
                None
            }
        } else {
            None
        };
        SliceVolumeData {
            cross_section_3d,
            cs_edges,
            near_z,
            poly,
            cam_3d,
        }
    }

    fn prepare_waypoints(
        &self,
        map_transform: &CameraProjection,
        scene_camera: &Camera,
        bounds: &Bounds4D,
        params: &MapRenderParams<'_>,
    ) -> Vec<PreparedWaypoint> {
        let slice_info = SliceInfo::new(scene_camera, self.w_thickness);
        let mut result = Vec::new();
        for wp in params.waypoints {
            let norm_pos = normalize_to_tesseract(wp.position, bounds);
            let vector_4d = match params.frame_mode {
                CompassFrameMode::Camera => scene_camera.world_vector_to_camera_frame(norm_pos),
                CompassFrameMode::World => norm_pos,
            };
            let s3d = map_transform.project(norm_pos).0;
            if s3d.z <= -self.projection_distance {
                continue;
            }
            let (edge_color, alpha) = slice_info.style_for_point(wp.position);
            let gadget =
                TetrahedronGadget::from_4d_vector_with_scale(vector_4d, TETRA_SCALE_WAYPOINT)
                    .with_tip_label(wp.title);
            let dist = (wp.position - scene_camera.position).norm();
            let dist_label = format!("({})", format_magnitude(dist));
            result.push(PreparedWaypoint {
                s3d,
                edge_color,
                alpha,
                gadget,
                dist_label,
            });
        }
        result
    }

    fn prepare_camera(
        &self,
        map_transform: &CameraProjection,
        scene_camera: &Camera,
        bounds: &Bounds4D,
        frame_mode: CompassFrameMode,
    ) -> Option<PreparedCamera> {
        let norm_cam = normalize_to_tesseract(scene_camera.position, bounds);
        let s3d = map_transform.project(norm_cam).0;
        if s3d.z <= -self.projection_distance {
            return None;
        }
        let slice_info = SliceInfo::new(scene_camera, self.w_thickness);
        let vector_4d = match frame_mode {
            CompassFrameMode::Camera => scene_camera.world_vector_to_camera_frame(norm_cam),
            CompassFrameMode::World => norm_cam,
        };
        let (edge_color, alpha) = slice_info.style_for_point(scene_camera.position);
        let gadget = TetrahedronGadget::from_4d_vector_with_scale(vector_4d, TETRA_SCALE_CAMERA)
            .with_tip_label("Cam");
        let forward_4d = scene_camera.project_camera_3d_to_world_4d(scene_camera.forward_vector());
        let forward_tess = direction_to_tesseract(forward_4d, bounds);
        let forward_3d = map_transform.project_direction(forward_tess);
        let forward_len = forward_3d.norm();
        let forward_dir = if forward_len > 1e-10 {
            forward_3d / forward_len
        } else {
            Vector3::zeros()
        };
        let tip_3d = s3d + forward_dir * FORWARD_ARROW_LENGTH;
        Some(PreparedCamera {
            s3d,
            edge_color,
            alpha,
            gadget,
            tip_3d,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_slice_volume(
        &self,
        batch: &mut LineBatch,
        projector: &StereoProjector,
        map_transform: &CameraProjection,
        data: &SliceVolumeData,
        view_rect: egui::Rect,
        stereo: StereoSettings,
        scene_camera: &Camera,
        bounds: &Bounds4D,
    ) {
        let screen_pts = if data.cross_section_3d.len() >= 3 {
            convex_hull_screen(&data.cross_section_3d, projector)
        } else {
            Vec::new()
        };
        if screen_pts.len() >= 3 {
            batch.add_convex_polygon(
                &screen_pts,
                crate::colors::SLICE_GREEN_FILL,
                SLICE_FILL_STROKE_WIDTH,
                SLICE_GREEN,
            );
            if let Some(poly) = &data.poly {
                let rays =
                    compute_frustum_rays(scene_camera, view_rect, stereo, bounds, map_transform);
                let planes = compute_frustum_planes(&rays, data.cam_3d);
                let mut clipped = poly.clone();
                for (pp, pn) in &planes {
                    clipped = clip_polyhedron_by_plane(&clipped, *pp, *pn);
                    if clipped.vertices.is_empty() {
                        break;
                    }
                }
                if clipped.vertices.len() >= 3 {
                    let vis_screen = convex_hull_screen(&clipped.vertices, projector);
                    if vis_screen.len() >= 3 {
                        batch.add_convex_polygon(
                            &vis_screen,
                            crate::colors::VISIBILITY_DARK_GREEN_FILL,
                            1.0,
                            VISIBILITY_DARK_GREEN,
                        );
                    }
                }
            }
        }
        for [p0, p1] in &data.cs_edges {
            if let Some(screen_seg) =
                clip_segment_to_screen(map_transform, projector, data.near_z, *p0, *p1)
            {
                batch.add_segment(screen_seg.0, screen_seg.1, SLICE_GREEN);
            }
        }
    }

    fn draw_waypoints(
        &self,
        batch: &mut LineBatch,
        painter: &egui::Painter,
        projector: &StereoProjector,
        waypoints: &[PreparedWaypoint],
    ) {
        for wp in waypoints {
            let Some(center_screen) = projector.project_3d(wp.s3d.x, wp.s3d.y, wp.s3d.z) else {
                continue;
            };
            render_tetrahedron_in_map(
                batch,
                painter,
                &wp.gadget,
                projector,
                CompassFrameMode::Camera,
                wp.edge_color,
                wp.alpha,
                wp.s3d,
                self.labels_visible,
            );
            if let Some(base_p) = projector.project_3d(wp.s3d.x, wp.s3d.y, wp.s3d.z) {
                let a = crate::colors::to_u8(wp.alpha * 200.0);
                painter.text(
                    base_p.screen_pos + egui::Vec2::new(0.0, MAP_DISTANCE_LABEL_OFFSET_Y),
                    egui::Align2::CENTER_TOP,
                    &wp.dist_label,
                    egui::FontId::proportional(MAP_DISTANCE_FONT_SIZE),
                    egui::Color32::from_rgba_unmultiplied(200, 200, 220, a),
                );
            }
            let dot_color = egui::Color32::from_rgba_unmultiplied(
                wp.edge_color.r(),
                wp.edge_color.g(),
                wp.edge_color.b(),
                crate::colors::to_u8(wp.alpha * 200.0),
            );
            batch.add_circle_filled(center_screen.screen_pos, MAP_CAMERA_DOT_RADIUS, dot_color);
        }
    }

    fn draw_camera_position(
        &self,
        batch: &mut LineBatch,
        painter: &egui::Painter,
        projector: &StereoProjector,
        cam: &PreparedCamera,
    ) {
        let Some(center_screen) = projector.project_3d(cam.s3d.x, cam.s3d.y, cam.s3d.z) else {
            return;
        };
        render_tetrahedron_in_map(
            batch,
            painter,
            &cam.gadget,
            projector,
            CompassFrameMode::Camera,
            cam.edge_color,
            cam.alpha,
            cam.s3d,
            self.labels_visible,
        );
        let dot_alpha = crate::colors::to_u8(cam.alpha * 255.0);
        batch.add_circle_filled(
            center_screen.screen_pos,
            MAP_CAMERA_DOT_RADIUS,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, dot_alpha),
        );
        let a = crate::colors::to_u8(cam.alpha * 255.0);
        let arrow_color = egui::Color32::from_rgba_unmultiplied(
            ARROW_FORWARD.r(),
            ARROW_FORWARD.g(),
            ARROW_FORWARD.b(),
            a,
        );
        if let (Some(arrow_p), Some(origin_p)) = (
            projector.project_3d(cam.tip_3d.x, cam.tip_3d.y, cam.tip_3d.z),
            projector.project_3d(cam.s3d.x, cam.s3d.y, cam.s3d.z),
        ) {
            let arrow_vec = arrow_p.screen_pos - origin_p.screen_pos;
            if arrow_vec.length() > 2.0 {
                batch.add_segment_with_width(
                    origin_p.screen_pos,
                    arrow_p.screen_pos,
                    FORWARD_ARROW_STROKE_WIDTH,
                    arrow_color,
                );
                if arrow_vec.length() > FORWARD_ARROW_HEAD_SIZE {
                    draw_arrow_head(
                        batch,
                        arrow_p.screen_pos,
                        arrow_vec,
                        FORWARD_ARROW_HEAD_SIZE,
                        arrow_color,
                    );
                }
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn render_vertex_labels(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[TransformedVertex],
    ) {
        let w_half = self.w_thickness * 0.5;
        for (i, tv) in transformed.iter().enumerate() {
            if !tv.in_slice {
                continue;
            }
            if tv.z <= -self.projection_distance {
                continue;
            }
            let Some(p) = projector.project_3d(tv.x, tv.y, tv.z) else {
                continue;
            };
            let vertex = &self.tesseract_vertices[i];
            let font_id = egui::FontId::monospace(MAP_AXIS_FONT_SIZE);
            for (ax, &ch) in AXIS_CHARS.iter().enumerate() {
                let component = vertex[ax];
                let color = compute_component_color(component, 1.0);
                let egui_color = color.to_egui_color();
                let offset_x = (ax as f32 - 1.5) * 7.0;
                painter.text(
                    p.screen_pos + egui::Vec2::new(offset_x, MAP_AXIS_LABEL_OFFSET_Y),
                    egui::Align2::CENTER_CENTER,
                    ch.to_string(),
                    font_id.clone(),
                    egui_color,
                );
            }
            let normalized_w = (tv.w / w_half).clamp(-1.0, 1.0);
            let dot_color = crate::render::w_to_color(normalized_w, 180, self.w_color_intensity);
            painter.circle_filled(p.screen_pos, MAP_WAYPOINT_DOT_RADIUS, dot_color);
        }
    }

    fn render_edge_labels(
        &self,
        painter: &egui::Painter,
        projector: &StereoProjector,
        transformed: &[TransformedVertex],
    ) {
        let font_id = egui::FontId::monospace(MAP_AXIS_FONT_SIZE);
        let near_plane = self.projection_distance;
        for chunk in self.tesseract_indices.chunks(2) {
            if chunk.len() != 2 {
                continue;
            }
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let t0 = &transformed[i0];
            let t1 = &transformed[i1];
            if !t0.in_slice && !t1.in_slice {
                continue;
            }
            if t0.z <= -near_plane && t1.z <= -near_plane {
                continue;
            }
            let Some(s0) = projector.project_3d(t0.x, t0.y, t0.z) else {
                continue;
            };
            let Some(s1) = projector.project_3d(t1.x, t1.y, t1.z) else {
                continue;
            };
            let Some(ax) = edge_axis(&self.tesseract_vertices, i0, i1) else {
                continue;
            };
            let mid = (s0.screen_pos + s1.screen_pos.to_vec2()) * 0.5 + MAP_EDGE_LABEL_OFFSET;
            let ch = AXIS_CHARS[ax];
            painter.text(
                mid,
                egui::Align2::CENTER_CENTER,
                ch.to_string(),
                font_id.clone(),
                crate::colors::AXIS_LABEL_YELLOW,
            );
        }
    }

    fn compute_waypoint_tap_zones(
        &mut self,
        left_projector: &StereoProjector,
        right_projector: &StereoProjector,
        waypoints: &[PreparedWaypoint],
    ) {
        self.waypoint_tap_zones.clear();
        for (idx, wp) in waypoints.iter().enumerate() {
            let Some(left_p) = left_projector.project_3d(wp.s3d.x, wp.s3d.y, wp.s3d.z) else {
                continue;
            };
            let Some(right_p) = right_projector.project_3d(wp.s3d.x, wp.s3d.y, wp.s3d.z) else {
                continue;
            };
            let z_offset = self.projection_distance + wp.s3d.z;
            if z_offset <= crate::render::NEAR_PLANE_THRESHOLD {
                continue;
            }
            let projected_size = TETRA_SCALE_WAYPOINT * left_projector.scale() / z_offset;
            let tap_radius =
                (projected_size * TAP_RADIUS_MULTIPLIER).clamp(TAP_RADIUS_MIN, TAP_RADIUS_MAX);
            self.waypoint_tap_zones
                .push((left_p.screen_pos, right_p.screen_pos, tap_radius, idx));
        }
    }

    #[must_use]
    pub fn find_tapped_waypoint(&self, tap_pos: egui::Pos2) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for &(left_pos, right_pos, radius, wp_index) in &self.waypoint_tap_zones {
            let dist_left = (tap_pos - left_pos).length();
            let dist_right = (tap_pos - right_pos).length();
            let dist = dist_left.min(dist_right);
            if dist <= radius && best.is_none_or(|(_, d)| dist < d) {
                best = Some((wp_index, dist));
            }
        }
        best.map(|(idx, _)| idx)
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::*;

    #[test]
    fn test_find_tapped_waypoint_no_zones() {
        let renderer = MapRenderer::new();
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(100.0, 100.0)),
            None
        );
    }

    #[test]
    fn test_find_tapped_waypoint_hit_left_eye() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            20.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(105.0, 205.0)),
            Some(0)
        );
    }

    #[test]
    fn test_find_tapped_waypoint_hit_right_eye() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            20.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(305.0, 205.0)),
            Some(0)
        );
    }

    #[test]
    fn test_find_tapped_waypoint_miss() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![(
            egui::Pos2::new(100.0, 200.0),
            egui::Pos2::new(300.0, 200.0),
            10.0,
            0,
        )];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(50.0, 50.0)),
            None
        );
    }

    #[test]
    fn test_find_tapped_waypoint_closest_wins() {
        let mut renderer = MapRenderer::new();
        renderer.waypoint_tap_zones = vec![
            (
                egui::Pos2::new(100.0, 100.0),
                egui::Pos2::new(100.0, 100.0),
                30.0,
                0,
            ),
            (
                egui::Pos2::new(115.0, 100.0),
                egui::Pos2::new(115.0, 100.0),
                30.0,
                1,
            ),
        ];
        assert_eq!(
            renderer.find_tapped_waypoint(egui::Pos2::new(112.0, 100.0)),
            Some(1)
        );
    }
}
