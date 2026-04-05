# 4D Map Geometry

This document describes the geometry pipeline used by the 4D compass/map view — a miniature
tesseract rendered alongside the main scene that shows the player's position, orientation, slice
cross-section, and camera visibility cone within the 4D volume.

## 1. Overview

The map renders a tesseract (hypercube / 8-cell) wireframe whose vertices correspond to the axis
extremes of the scene's bounding box. Inside this wireframe, the map displays:

- The **slice cross-section** — a green filled polygon showing where the scene camera's 3D
  hyperplane intersects the tesseract.
- The **visibility cone** — a darker green filled polygon showing the portion of that cross-section
  the scene camera can actually see (frustum intersection).
- **Waypoints** and the **scene camera position** as labeled tetrahedral gadgets.

The map has its own independent `Camera`, allowing the player to rotate and navigate the map view
separately from the main scene.

## 2. Coordinate Systems

The geometry pipeline involves four coordinate systems:

| Stage | Space | Range | Description |
|-------|-------|-------|-------------|
| World 4D | `Vector4<f32>` | Unbounded | Scene objects in simulation space |
| Tesseract | `Vector4<f32>` | [-1, 1]⁴ | World 4D normalized to the bounding box |
| Map 3D | `Vector3<f32>` | Unbounded | After MapViewTransform projection from 4D |
| Screen 2D | `egui::Pos2` | Viewport rect | After StereoProjector perspective projection |

### World 4D → Tesseract

`normalize_to_tesseract(pos, bounds)` maps each axis independently:

```
tesseract[i] = 2 * (pos[i] - bounds.min[i]) / (bounds.max[i] - bounds.min[i]) - 1
```

The bounding box is computed by `compute_bounds()` from the scene camera position and all
waypoints, with 20% padding (`BOUNDS_PADDING_FACTOR`).

### Tesseract → Map 3D

`MapViewTransform` handles this two-step projection:

1. **4D rotation**: Apply the map camera's inverse 4D rotation matrix, then subtract the rotated
   camera offset. This brings the map camera to the origin looking down -Z (in the 4D-rotated
   frame), dropping the W component to collapse to 3D.
2. **3D rotation**: Apply the map camera's left quaternion (q_left) inverse rotation matrix to
   orient the view.

This is analogous to a "view matrix" in a 4D→3D pipeline.

### Map 3D → Screen 2D

The `StereoProjector` applies perspective projection with configurable eye separation for
stereoscopic display. The map renders two views (left eye, right eye) side-by-side.

## 3. The Cross-Section Algorithm

The cross-section is computed by intersecting the scene camera's 3D hyperplane with each tesseract
edge. There are two separate functions for this:

### `compute_slice_cross_section()` — Point Cloud

Returns an **unordered** set of intersection points. For each tesseract edge, if the signed
distance from the two endpoints to the slice hyperplane has opposite signs, the edge is clipped at
the crossing point (t = -d0 / (d1 - d0)).

This produces 8 points for a w=0 slice of a standard tesseract (a cube), but they come as an
unordered point cloud — no connectivity information.

### `compute_cross_section_edges()` — Edge List

Returns an **ordered list of edge segments** by intersecting the slice hyperplane with each
tesseract **face** (quad). Each face that the plane crosses produces exactly 2 intersection points,
forming one edge of the cross-section polygon.

This gives both the geometry and the connectivity, but as individual edge segments, not as an
ordered polygon boundary.

## 4. The Ordered Polygon Problem

The cross-section is a convex polytope (guaranteed by the convexity of the tesseract). For
rendering as a filled polygon, we need an **ordered** set of vertices forming the boundary.

`compute_slice_cross_section()` returns unordered points. `convex_hull_2d()` computes the
2D convex hull of these points after projecting to screen, producing a correctly ordered polygon.

**Why this is correct:** The tesseract cross-section at any slice position is always a convex
polygon (or polyhedron). The convex hull of the projected vertices equals the boundary of the
projected cross-section. This is the convexity guarantee.

The edge list from `compute_cross_section_edges()` is used for rendering the outline strokes,
where connectivity matters but ordering does not.

## 5. Near-Plane Clipping

`clip_segment_to_screen()` handles the case where 3D points fall behind the map camera's near
projection plane (z ≤ near_z). If both endpoints are behind the plane, the segment is discarded.
If exactly one is behind, the segment is clipped at the near plane using linear interpolation in z.

This prevents the perspective projection from inverting geometry that crosses the camera.

## 6. The Visibility Cone

### Purpose

The visibility cone shows the **intersection of the scene camera's view frustum with the slice
cross-section**, rendered as a darker green filled polygon on the map. This tells the player which
portion of the cross-section is actually visible on screen.

### Chosen Approach: 2D Post-Projection Clipping

The intersection is computed entirely in screen 2D space after projecting through the map's own
camera/projector. This avoids needing an ordered 3D polygon representation of the cross-section
(which doesn't currently exist — see Section 4).

### Algorithm

1. **Compute angular half-FOV** — `compute_frustum_half_angles()` derives the scene camera's
   angular half-FOV from the viewport rect and projection distance. The scale formula
   `rect.height().min(rect.width() * 0.5) * 0.35` matches `render_stereo_views()` in
   `render.rs`. The stereo split gives each eye half the rect width, so the horizontal half-extent
   is `rect.width() * 0.25`.

2. **Build frustum corner directions** — In camera-local 3D, the four frustum corner directions
   are `(±tan_x, ±tan_y, 1)` using the right/up/forward vectors from the scene camera.

3. **Project to screen** — Each direction is projected through:
   - Camera-local 3D → World 4D via `project_camera_3d_to_world_4d()`
   - Far point = camera position + direction × `FRUSTUM_FAR_DISTANCE`
   - World 4D → Tesseract normalization
   - Tesseract → Map 3D via `MapViewTransform`
   - Map 3D → Screen 2D via `StereoProjector`

4. **Clip the cross-section polygon** — The cross-section's screen-space convex hull is clipped
   against 4 half-planes forming the frustum cone (apex at camera screen position, edges through
   the 4 projected frustum corners). This uses the Sutherland-Hodgman algorithm.

5. **Render** — If the result has ≥ 3 points, draw as a dark green filled polygon.

### Key Insight

Because we work in 2D screen space, the frustum cone is defined purely by angular relationships
which perspective projection preserves. The exact far distance (step 3) doesn't matter — only the
direction of each ray matters, and the screen-space cone is the same regardless.

### Near-Plane Guard

If the scene camera's projected position falls behind the map camera's near plane
(`cam_3d.z ≤ near_z`), the visibility cone is skipped entirely — the camera cannot be projected
and no meaningful cone can be drawn.

## 7. Sutherland-Hodgman Polygon Clipping

### `clip_polygon_by_half_plane()`

Clips a convex polygon against a single half-plane. Points on the LEFT side of the directed line
`edge_start → edge_end` are kept (2D cross product ≥ 0).

For each edge of the input polygon, the algorithm:
- Emits the current vertex if it's inside
- Emits the intersection point if the edge crosses the boundary
- Skips vertices that are outside

### `clip_polygon_to_frustum_cone()`

Clips a polygon against 4 half-planes defining the frustum cone. For each ray from camera to
frustum corner, the half-plane is oriented so the centroid of the frustum corners is on the
"inside" — this auto-detects the correct orientation regardless of coordinate system handedness.

## 8. MapViewTransform

### Position vs. Direction

`MapViewTransform` has two projection methods:

- **`project_to_3d(pos_4d)`** — For positions. Applies the rotation matrix AND subtracts the
  camera offset (translation). This is the standard "view matrix" transform.

- **`direction_to_3d(dir_4d)`** — For direction vectors. Applies the rotation matrix only, without
  subtracting the camera offset. This is essential for transforming frustum edge directions:
  subtracting the offset would corrupt direction-only transforms, since directions are not rooted
  at any position.

The relationship is:
```
direction_to_3d(d) = project_to_3d(origin + d) - project_to_3d(origin)
```

## 9. FOV Derivation

The scene camera's field of view is not stored explicitly — it's implicit in the viewport
dimensions and projection parameters used by `render_stereo_views()`. The function
`compute_frustum_half_angles()` reconstructs it:

```rust
let scale = rect.height().min(rect.width() * 0.5) * 0.35;
let half_width = rect.width() * 0.25;   // stereo split: each eye gets half width
let half_height = rect.height() * 0.5;  // height not split
let tan_half_fov_x = half_width / (scale * projection_distance);
let tan_half_fov_y = half_height / (scale * projection_distance);
```

- `scale` matches the rendering pipeline's internal scale factor
- `half_width = rect.width() * 0.25` because the stereo split gives each eye half the rect width,
  and we want the half-extent of one eye's view
- `half_height = rect.height() * 0.5` because height is not split between eyes
- The resulting tangent values define the angular extent of the view frustum
