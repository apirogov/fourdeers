# Implementation Plan: 3D Visibility Cone via Polyhedron Clipping

## Goal

Replace the broken 2D post-projection visibility cone with a correct 3D approach: clip the green
cross-section polyhedron against the scene camera's frustum planes in map 3D space, then render
the result as a darker convex hull — same rendering approach as the existing green cross-section.

## Why the 2D Approach Failed

The old approach projected frustum far points to screen via the map's projector pipeline. With
`FRUSTUM_FAR_DISTANCE = 5.0`, far points routinely ended up behind the map camera (projector
returned `None`), so the cone was never drawn. The 2D approach is inherently fragile because it
depends on arbitrary far distances surviving the full 4D→3D→screen pipeline.

The 3D approach avoids this entirely by working in map 3D space where all cross-section geometry
already lives, and only projecting the final clipped result to screen.

---

## Geometry Pipeline

```
Eye rect corners (screen 2D)
  → unproject to camera-local 3D directions:
      dir = ((sx - center.x)/scale, (center.y - sy)/scale, proj_dist)
  → project_3d_to_4d (full rotation q_left * v * q_right⁻¹)
  → scale to tesseract units: dir_tess[i] = dir_world[i] * 2.0 / range[i]
  → direction_to_3d (map camera rotation, no offset) → map 3D ray directions

4 rays + cam_3d → 4 planes (cross product of adjacent ray pairs, normal inward)
cross_section_3d + cs_edges → ConvexPolyhedron in map 3D
clip polyhedron by each plane sequentially → clipped polyhedron
project surviving vertices → 2D convex hull on screen → draw
```

### Unprojection Math

The projector maps 3D → 2D as:

```
screen_x = center.x + (x - eye_offset) * scale * proj_dist / (proj_dist + z)
screen_y = center.y - y * scale * proj_dist / (proj_dist + z)
```

For a point at `z = 0` (the projection plane), inverting:

```
x = (screen_x - center.x) / scale + eye_offset
y = (center.y - screen_y) / scale
z = 0
```

But we want a *direction*, not a projected point. The camera looks along +Z. A ray from the eye
point through screen corner `(sx, sy)` at the projection plane has direction:

```
dx = (sx - center.x) / scale + eye_offset
dy = (center.y - sy) / scale
dz = projection_distance
```

This is the direction in camera-local 3D space.

### Eye Rect

The `render_slice_volume` callback receives a `view_rect` (the current eye's half of the stereo
split) via `render_stereo_views`. But it also receives `rect` (the full rect). We need the single
eye's rect. The callback already receives `view_rect` — use that directly.

**Wait**: `render_slice_volume` currently receives `rect` (full) and `stereo` (settings). It does
NOT receive the per-eye `view_rect`. But the outer `render_stereo_views` callback does get
`view_rect`. Currently the signature is:

```rust
fn render_slice_volume(
    &self,
    painter: &egui::Painter,
    projector: &StereoProjector,
    scene_camera: &Camera,
    bounds: &(Vector4<f32>, Vector4<f32>),
    rect: egui::Rect,
    stereo: StereoSettings,
)
```

We need `view_rect` (the per-eye rect). We already have `projector` which knows the center and
scale. We can reconstruct the eye rect from the projector's center, the stereo split geometry, and
the overall rect dimensions:

```
eye_rect = split the full rect in half based on which eye the projector is for
```

Actually, the simplest approach: derive the eye rect from `projector.center()` and the full `rect`.

For the left eye: `left_rect = Rect { min: rect.min, max: pos2(rect.center().x, rect.max.y) }`
For the right eye: `right_rect = Rect { min: pos2(rect.center().x, rect.min.y), max: rect.max }`

The projector's `center.x` tells us which eye we're in:
- Left eye: center.x is in the left half of `rect`
- Right eye: center.x is in the right half of `rect`

```rust
let (eye_rect, _) = split_stereo_views(rect);
if projector.center().x > rect.center().x {
    (_, eye_rect) = split_stereo_views(rect);
}
```

Or even simpler: we can compute the 4 corner directions directly from the projector parameters
without needing the rect at all. The projector knows `center`, `scale`, `eye_offset`, and
`projection_distance`. The view extents at the projection plane are determined by the half-width
and half-height of the eye's view. But we need those from the rect.

**Simplest correct approach**: pass `view_rect` to `render_slice_volume`.

Change the call site in `render()` (line ~190) from:

```rust
self.render_slice_volume(painter, projector, scene_camera, &bounds, rect, stereo);
```

to also pass `view_rect`:

```rust
self.render_slice_volume(painter, projector, scene_camera, &bounds, view_rect, stereo);
```

And update the signature to take `view_rect` instead of `rect`.

---

## New Types & Functions

All in `src/map.rs`.

### `ConvexPolyhedron`

```rust
struct ConvexPolyhedron {
    vertices: Vec<Vector3<f32>>,
    edges: Vec<[usize; 2]>,
}
```

Built from `cross_section_3d` + projected `cs_edges`:

```rust
impl ConvexPolyhedron {
    fn from_cross_section(
        cross_section_3d: &[Vector3<f32>],
        cs_edges_4d: &[[Vector4<f32>; 2]],
        map_transform: &MapViewTransform,
        near_z: f32,
    ) -> Self { ... }
}
```

For each edge `[p0_4d, p1_4d]`, project both endpoints to 3D via `map_transform.project_to_3d`.
Find matching vertices in `cross_section_3d` (within epsilon). Build index-based edges.

Actually, simpler approach: project cs_edges endpoints to 3D independently, collect all unique
vertices (merge within epsilon), and build edge index pairs. The `cross_section_3d` filter by
`near_z` already handles near-plane culling.

**Even simpler**: Don't try to match with `cross_section_3d` at all. Just project all cs_edges to
3D, collect vertices and edges as a fresh polyhedron. Filter out edges with vertices behind near_z
at the end (or just let the projection handle it).

```rust
fn build_cross_section_polyhedron(
    cs_edges: &[[Vector4<f32>; 2]],
    map_transform: &MapViewTransform,
) -> ConvexPolyhedron
```

Steps:
1. For each edge, project both endpoints to 3D
2. Build vertex list with deduplication (within `VERTEX_MERGE_EPS_SQ`)
3. Build edge index pairs referencing the deduplicated vertex list

### `direction_to_tesseract`

```rust
fn direction_to_tesseract(
    dir_world: Vector4<f32>,
    bounds: &(Vector4<f32>, Vector4<f32>),
) -> Vector4<f32>
```

Scales a world-space 4D direction to tesseract units:

```rust
Vector4::new(
    dir_world[0] * 2.0 / (bounds.1[0] - bounds.0[0]),
    dir_world[1] * 2.0 / (bounds.1[1] - bounds.0[1]),
    dir_world[2] * 2.0 / (bounds.1[2] - bounds.0[2]),
    dir_world[3] * 2.0 / (bounds.1[3] - bounds.0[3]),
)
```

This is the directional analog of `normalize_to_tesseract` (which handles positions). Directions
only scale, they don't translate.

### `compute_frustum_rays`

```rust
fn compute_frustum_rays(
    scene_camera: &Camera,
    view_rect: egui::Rect,
    stereo: StereoSettings,
    bounds: &(Vector4<f32>, Vector4<f32>),
    map_transform: &MapViewTransform,
) -> [Vector3<f32>; 4]
```

Steps:
1. Compute `scale` same as `render_stereo_views`: `rect.height().min(rect.width() * 0.5) * 0.35`
   Wait — the `view_rect` is already one eye's half. The scale was computed from the *full* rect
   in `render_stereo_views` (line 35: `let scale = rect.height().min(rect.width() * 0.5) * 0.35`).
   But inside `render_slice_volume`, we now receive `view_rect` not the full `rect`.
   
   The projector already has the correct `scale` baked in. We can extract it: `projector.scale()`.

2. Compute eye rect corners in screen space:
   ```
   TL = view_rect.min
   TR = pos2(view_rect.max.x, view_rect.min.y)
   BL = pos2(view_rect.min.x, view_rect.max.y)
   BR = view_rect.max
   ```

3. Unproject each corner to camera-local 3D direction:
   ```
   dx = (sx - center.x) / projector.scale() + projector.eye_offset()
   dy = (center.y - sy) / projector.scale()
   dz = stereo.projection_distance
   ```
   
   Wait — the `center` of the projector is the eye rect center, and `scale` is the same. So:
   ```
   dx = (sx - eye_center_x) / scale + eye_offset
   dy = (eye_center_y - sy) / scale
   dz = projection_distance
   ```
   
   For a corner like TL where `sx = view_rect.min.x`:
   ```
   dx = (view_rect.min.x - eye_center_x) / scale + eye_offset
   ```
   This is the negative half-width in projection-plane units + eye offset.
   
   Actually, `projector.center()` already accounts for which eye. And `projector.eye_offset()` is
   `±eye_separation * 0.5`. The unprojection correctly accounts for stereo.

4. Transform each camera-local direction to map 3D:
   ```rust
   let dir_local_3d = Vector3::new(dx, dy, dz);
   let dir_4d = scene_camera.project_3d_to_4d(dir_local_3d);
   let dir_tess = direction_to_tesseract(dir_4d, bounds);
   let dir_map_3d = map_transform.direction_to_3d(dir_tess);
   ```
   
   We normalize the result to unit vectors (directions only, lengths don't matter for plane
   construction).

5. Return the 4 unit ray directions: `[TL, TR, BR, BL]` (or any consistent winding order).

**Note on `projector.eye_offset()`**: The `StereoProjector` has a private `eye_offset` field.
We need to expose it or compute it. Let's add a public accessor:

```rust
// In render.rs, StereoProjector impl:
pub fn eye_offset(&self) -> f32 {
    self.eye_offset
}
```

Or we can compute the frustum rays without needing `eye_offset` by using the fact that the corner
directions in camera-local space are symmetric around the Z axis when measured from the eye
position (not the center). The corners of the view at the projection plane, relative to the eye,
are:

```
half_w = view_rect.width() * 0.5 / scale
half_h = view_rect.height() * 0.5 / scale
```

And the 4 directions are:
```
(+half_w, +half_h, projection_distance)
(-half_w, +half_h, projection_distance)
(-half_w, -half_h, projection_distance)
(+half_w, -half_h, projection_distance)
```

Wait, but the eye is offset from center by `eye_offset`. The actual visible region at the
projection plane goes from `(center.x - eye_offset - half_w_view)` to
`(center.x - eye_offset + half_w_view)` in world X. But the *directions from the eye* through the
corners of the screen rect are what matters for the frustum.

Let me think again. The projector is:
```
screen_x = center.x + (x - eye_offset) * final_scale
```
where `final_scale = scale * proj_dist / (proj_dist + z)`.

At `z = 0` (projection plane), `final_scale = scale`. So:
```
screen_x = center.x + (x - eye_offset) * scale
```
→ `x = (screen_x - center.x) / scale + eye_offset`

For the 4 corners of `view_rect`:
- `screen_x` ranges from `view_rect.min.x` to `view_rect.max.x`
- `screen_y` ranges from `view_rect.min.y` to `view_rect.max.y`

The eye is at `(eye_offset, 0, -projection_distance)` in camera-local space (looking along +Z).

A ray from the eye through screen point `(sx, sy)` at the projection plane (`z=0`):
```
direction = (x_at_z0 - eye_offset, y_at_z0 - 0, 0 - (-projection_distance))
          = ((sx - center.x)/scale + eye_offset - eye_offset, (center.y - sy)/scale, projection_distance)
          = ((sx - center.x)/scale, (center.y - sy)/scale, projection_distance)
```

The `eye_offset` cancels out! The frustum ray directions from the eye through the screen corners
do NOT depend on the eye offset. This makes sense: the screen is centered on each eye, and the
parallax is baked into the center position, not the angular extent.

So the 4 directions are simply:
```rust
let scale = projector.scale();
let cx = projector.center().x;
let cy = projector.center().y;
let pd = stereo.projection_distance;

[
    Vector3::new((view_rect.left()  - cx) / scale, (cy - view_rect.top())    / scale, pd),
    Vector3::new((view_rect.right() - cx) / scale, (cy - view_rect.top())    / scale, pd),
    Vector3::new((view_rect.right() - cx) / scale, (cy - view_rect.bottom()) / scale, pd),
    Vector3::new((view_rect.left()  - cx) / scale, (cy - view_rect.bottom()) / scale, pd),
]
```

We don't need `eye_offset` at all. This simplifies things.

### `compute_frustum_planes`

```rust
fn compute_frustum_planes(
    rays: &[Vector3<f32>; 4],
    cam_3d: Vector3<f32>,
) -> [(Vector3<f32>, Vector3<f32>); 4]
```

For each pair of adjacent rays `rays[i]` and `rays[(i+1)%4]`, the plane passes through `cam_3d`
and contains both rays. The plane normal is the cross product:

```rust
let normal = rays[i].cross(&rays[(i+1) % 4]).normalize();
```

Ensure the normal points inward (toward the interior of the frustum). The interior is the side
where the average of the 4 rays points. Check:

```rust
let forward = (rays[0] + rays[1] + rays[2] + rays[3]) * 0.25;
if normal.dot(&forward) < 0.0 {
    normal = -normal;
}
```

Return `[(plane_point, plane_normal); 4]` where `plane_point = cam_3d` for all planes.

### `clip_polyhedron_by_plane`

```rust
fn clip_polyhedron_by_plane(
    poly: &ConvexPolyhedron,
    plane_point: Vector3<f32>,
    plane_normal: Vector3<f32>,
) -> ConvexPolyhedron
```

Sutherland-Hodgman in 3D:

1. Compute signed distance for each vertex: `d = (v - plane_point).dot(&plane_normal)`
2. Classify: inside if `d >= 0`
3. For each edge `[i, j]`:
   - Both inside → keep edge
   - Both outside → discard edge
   - Crossing → compute intersection, add to vertex list, create partial edge
4. **Cap face**: collect all intersection points (from crossing edges). They lie on the clip
   plane. Project to 2D on the clip plane, compute convex hull, add hull edges.

Cap-face 2D projection:
- Pick an arbitrary basis on the plane. If `|normal.z| < 0.9`, use `u = normal × (0,0,1)`
  normalized, `v = normal × u`. Otherwise use `u = normal × (1,0,0)`, `v = normal × u`.
- Project intersection points: `(p - plane_point).dot(&u), (p - plane_point).dot(&v)`
- `convex_hull_2d` on these 2D points
- Add edges from the hull indices

The vertex deduplication after clipping uses `VERTEX_MERGE_EPS_SQ` to merge vertices that are
within sqrt(1e-6) ≈ 0.001 of each other. This handles floating-point imprecision from edge-plane
intersections.

### `convex_hull_2d_indexed`

```rust
fn convex_hull_2d_indexed(points: &[(f32, f32)]) -> Vec<usize>
```

Returns ordered indices forming the convex hull. Same algorithm as existing `convex_hull_2d` but
returns indices instead of points. Needed for cap-face edge construction.

### `build_cross_section_polyhedron`

```rust
fn build_cross_section_polyhedron(
    cs_edges: &[[Vector4<f32>; 2]],
    map_transform: &MapViewTransform,
) -> ConvexPolyhedron
```

Steps:
1. For each edge, project both endpoints to 3D via `map_transform.project_to_3d`
2. Build vertex list with deduplication (within `VERTEX_MERGE_EPS_SQ`):
   - For each new 3D point, check distance² to existing vertices
   - If match found, reuse index; otherwise add new vertex
3. Build edge index pairs referencing the deduplicated vertex list
4. Return the polyhedron

---

## Changes to `render_slice_volume`

### Signature Change

```rust
fn render_slice_volume(
    &self,
    painter: &egui::Painter,
    projector: &StereoProjector,
    scene_camera: &Camera,
    bounds: &(Vector4<f32>, Vector4<f32>),
    view_rect: egui::Rect,       // changed from rect
    stereo: StereoSettings,
)
```

The call site in `render()` (line ~190) changes to pass `view_rect` instead of `rect`.

### Visibility Cone Block

After the existing green cross-section fill (line ~385), replace the old 2D visibility block
with:

```rust
// ── Visibility cone: 3D polyhedron clipping ──────────────────────
//
// 1. Build cross-section polyhedron from edges in map 3D.
// 2. Compute 4 frustum rays by unprojecting eye-rect corners through
//    the scene camera, then transforming to map 3D.
// 3. Derive 4 frustum planes from adjacent ray pairs.
// 4. Clip polyhedron against each plane.
// 5. If result has ≥ 3 vertices, project to screen and draw as
//    dark green filled convex hull.

let cam_3d = map_transform.project_to_3d(norm_cam);
if cam_3d.z > near_z {
    let poly = build_cross_section_polyhedron(&cs_edges, &map_transform);
    if poly.vertices.len() >= 3 {
        let rays = compute_frustum_rays(
            scene_camera, view_rect, stereo, bounds, &map_transform,
        );
        let planes = compute_frustum_planes(&rays, cam_3d);
        let mut clipped = poly;
        for (pp, pn) in &planes {
            clipped = clip_polyhedron_by_plane(&clipped, *pp, *pn);
            if clipped.vertices.is_empty() {
                break;
            }
        }
        if clipped.vertices.len() >= 3 {
            let vis_screen = convex_hull_screen_3d(&clipped, projector);
            if vis_screen.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    vis_screen,
                    visibility_dark_green_fill(),
                    egui::Stroke::new(1.0, VISIBILITY_DARK_GREEN),
                ));
            }
        }
    }
}
```

---

## Remove

- `FRUSTUM_FAR_DISTANCE` constant
- `compute_frustum_half_angles` function
- `clip_polygon_by_half_plane` function
- `clip_polygon_to_frustum_cone` function
- Old 2D visibility tests:
  - `test_visibility_polygon_is_subset_of_cross_section`
  - `test_visibility_polygon_with_identity_cam`
  - `test_visibility_polygon_with_rotated_cam`
  - `test_clip_polygon_by_half_plane_square`
  - `test_clip_polygon_by_half_plane_triangle`
  - `test_clip_polygon_empty_result`
  - `test_clip_polygon_unchanged`
  - `test_clip_polygon_to_frustum_cone_basic`
  - `test_clip_polygon_to_frustum_cone_full_containment`
  - `test_clip_polygon_to_frustum_cone_no_overlap`
  - `test_compute_frustum_half_angles`
- `#[allow(dead_code)]` on `direction_to_3d` (now used)

## Keep

- `VISIBILITY_DARK_GREEN` and `visibility_dark_green_fill()` (same colors)
- `direction_to_3d` (now actively used)
- `convex_hull_2d` (reused by `convex_hull_2d_indexed` and `convex_hull_screen`)

---

## New Tests

### Unit: `clip_polyhedron_by_plane`

```rust
#[test]
fn test_clip_polyhedron_by_plane_cube()
```
Build a unit cube as a ConvexPolyhedron (8 vertices, 12 edges). Clip against a plane that cuts
through diagonally. Verify the result has the expected number of vertices and edges.

```rust
#[test]
fn test_clip_polyhedron_preserves_fully_inside()
```
A plane that doesn't intersect the cube. Result should be identical.

```rust
#[test]
fn test_clip_polyhedron_empties_fully_outside()
```
A plane that culls everything. Result should be empty.

```rust
#[test]
fn test_clip_polyhedron_half_cube()
```
Clip a cube at x=0. Result should be a half-cube (a square-based prism) with 6 original vertices
+ 4 intersection vertices = 10 vertices, and correct edge count.

### Unit: `compute_frustum_rays`

```rust
#[test]
fn test_frustum_ray_directions_identity()
```
With identity camera and a square rect, verify rays point roughly forward (+Z in map 3D) and
spread symmetrically.

### Integration: visibility cone

```rust
#[test]
fn test_visibility_cone_3d_identity_cam()
```
Build the cross-section polyhedron for a default w=0 slice. Compute frustum rays with identity
camera. Clip. Assert non-empty result (≥ 3 vertices).

```rust
#[test]
fn test_visibility_cone_3d_rotated_cam()
```
Same but with `scene_camera.rotate(0.5, 0.3)`. Assert non-empty.

```rust
#[test]
fn test_visibility_cone_3d_subset_of_cross_section()
```
After clipping, verify all visibility vertices lie inside the original cross-section hull
(projected to screen, point-in-polygon test — reuse the existing ray-casting test approach).

### Unit: `direction_to_tesseract`

```rust
#[test]
fn test_direction_to_tesseract_identity_bounds()
```
With bounds (-1,-1,-1,-1) to (1,1,1,1), `direction_to_tesseract` should be identity (2.0/2.0 = 1.0
per component).

```rust
#[test]
fn test_direction_to_tesseract_scaled()
```
With bounds (-2,-2,-2,-2) to (2,2,2,2), should scale by 0.5 (2.0/4.0).

### Unit: `build_cross_section_polyhedron`

```rust
#[test]
fn test_build_cross_section_polyhedron_cube()
```
For the default w=0 slice, the cross-section is a cube. Building the polyhedron from cs_edges
should give 8 vertices and 12 edges.

---

## Execution Order

1. Add `VERTEX_MERGE_EPS_SQ` constant
2. Add `ConvexPolyhedron` struct + `build_cross_section_polyhedron`
3. Add `direction_to_tesseract`
4. Add `convex_hull_2d_indexed`
5. Add `clip_polyhedron_by_plane`
6. Add `compute_frustum_rays` + `compute_frustum_planes`
7. Change `render_slice_volume` signature to take `view_rect`
8. Replace visibility block in `render_slice_volume` with new 3D approach
9. Update call site in `render()` to pass `view_rect`
10. Remove dead code (`FRUSTUM_FAR_DISTANCE`, `compute_frustum_half_angles`,
    `clip_polygon_by_half_plane`, `clip_polygon_to_frustum_cone`)
11. Remove `#[allow(dead_code)]` from `direction_to_3d`
12. Remove old 2D visibility tests, add new 3D tests
13. `cargo fmt`, `cargo clippy`, `cargo test` — fix issues
14. `just wasm`
15. Commit
