# Code Smell Audit - FourDeers Codebase

## Summary Statistics

| Category | Count |
|----------|-------|
| Logic Duplication | 6 instances |
| Data Duplication | 4 instances |
| Redundant Computations | 4 instances |
| Magic Numbers | ~40+ unique values |
| Unnecessary Clones | 13 instances |
| Dead Code | 7 instances |
| Inefficient Patterns | 5 instances |
| Anti-patterns (`new()` + Default) | 13 instances |
| Missing const | ~15+ values |

---

## 1. LOGIC DUPLICATION

| File | Lines | Issue |
|------|-------|-------|
| `src/render/ui.rs` | 88-114 | `render_outlined_text` and `render_dual_outlined_text` - duplicated text rendering with outline |
| `src/input/zone_debug.rs` | 93-144 | `render_4zone_boundaries` vs `render_9zone_boundaries` - nearly identical |
| `src/input/zone_debug.rs` | 158-261 | `render_4zone_labels` vs `render_9zone_labels` - same pattern |
| `src/map/renderer.rs` | 433-532 | `draw_waypoints` vs `draw_camera_position` - similar tetrahedron + circle + arrow logic |
| `src/geometry.rs` | 89-145 | `convex_hull_2d_indexed` and `convex_hull_2d` - nearly identical |

---

## 2. DATA DUPLICATION

| File | Lines | Issue |
|------|-------|-------|
| `src/geometry.rs` | - | `Bounds4D` struct mirrors 4D vertex structure |
| `src/render/projection.rs` | 163-167 | `StereoSettings` duplicates `stereo` field in toy structs |
| `src/app.rs` | 23-28 | `CommonSettings` contains copy of `FourDSettings` + `StereoSettings` |
| `src/render/tesseract.rs` | 32-36 | `TesseractRenderConfig` wraps same fields as `FourDSettings` + `StereoSettings` |

---

## 3. REDUNDANT COMPUTATIONS

| File | Lines | Issue |
|------|-------|-------|
| `src/toys/polytopes.rs` | 236,414 | `compass_waypoints()` called twice in same function |
| `src/map/renderer.rs` | 141-145 | `compute_bounds` called every frame, could cache until camera/waypoints change |
| `src/render/tesseract.rs` | 156-158 | `basis_vectors()` computed in both `render_zone_labels` and `render_tetrahedron_gadget` |
| `src/map/renderer.rs` | 627-643 | `compute_waypoint_tap_zones` iterates waypoints twice (left and right projector) - could combine |

---

## 4. MAGIC NUMBERS

### Not yet converted to named constants:

| File | Line | Value | Context |
|------|------|-------|---------|
| `src/app.rs` | 16 | `0.3` | TAP_MAX_TIME |
| `src/app.rs` | 17 | `0.15` | DOUBLE_TAP_SUPPRESSION_TIME |
| `src/app.rs` | 19 | `12` | MENU_INNER_MARGIN |
| `src/input/zones.rs` | 206-207 | `1.0/3.0` | zone_9way threshold |
| `src/input/zones.rs` | 219-220 | `2.0 * third_x`, `2.0 * third_y` | zone_9way thresholds |
| `src/render/projection.rs` | 179 | `0.12` | default eye_separation |
| `src/render/style.rs` | 55-67 | `0.8`, `0.5`, `0.6` | w_to_color intensity multipliers |
| `src/render/tetra.rs` | 54 | `16.0` | vertex_label_font_size |
| `src/render/tetra.rs` | 81 | `14.0` | vertex_label_font_size |
| `src/render/tetra.rs` | 62 | `3.0` | arrow_stroke_width |
| `src/render/tetra.rs` | 64 | `20.0` | arrow_head_scale |
| `src/render/tetra.rs` | 67 | `4.0` | tip_dot_radius |
| `src/render/tetra.rs` | 68 | `12.0` | tip_label_font_size |
| `src/render/tetra.rs` | 69 | `15.0` | tip_label_offset_y |
| `src/render/tetra.rs` | 91 | `15.0` | arrow_head_scale |
| `src/render/tetra.rs` | 95 | `10.0` | tip_label_font_size |
| `src/render/tetra.rs` | 96 | `12.0` | tip_label_offset_y |
| `src/map/helpers.rs` | 51 | `0.12` | label_normal_offset |
| `src/map/helpers.rs` | 74 | `2.0` | origin_dot_radius |
| `src/map/helpers.rs` | 76 | `0.0` | tip_dot_radius |
| `src/map/slice.rs` | 31 | `2.0` | w_half multiplier |
| `src/map/visibility.rs` | 346 | `5.0` | orbit_radius |
| `src/map/visibility.rs` | 347 | `12` | steps |
| `src/tetrahedron/color.rs` | 34-35 | `0.8`, `0.5` | intensity multipliers |
| `src/tetrahedron/color.rs` | 41-42 | `0.6`, `0.6` | intensity multipliers |
| `src/tetrahedron/gadget.rs` | 170 | `0.15` | arrow_head_size scale |
| `src/tetrahedron/types.rs` | 11-14 | `0.05`, `0.07` | layout scales |
| `src/toys/polytopes.rs` | 21-22 | `-10.0..=10.0`, `-3.0..=3.0` | slider ranges |

---

## 5. UNNECESSARY CLONES

| File | Lines | Issue |
|------|-------|-------|
| `src/toys/polytopes.rs` | 96,101,109,113 | `.clone()` on ranges (Copy types) - unnecessary |
| `src/map/renderer.rs` | 242 | `map_transform.clone()` - could use reference |
| `src/map/renderer.rs` | 404 | `poly.clone()` - cloned for clipping, could use & reference |
| `src/render/ui.rs` | 64,97,111-112 | `font_id.clone()` - FontId is small, could pass by value |
| `src/render/tetra.rs` | 183 | `font_id.clone()` - same |
| `src/view/compass_view.rs` | 142 | `waypoints[idx].clone()` - returns owned String, could return &str |

---

## 6. DEAD CODE

| File | Lines | Issue |
|------|-------|-------|
| `src/render/tesseract.rs` | 160-169 | `label_offsets[4..]` entries all `(0.0, 0.0)` - unused indices 4-7 |
| `src/input/mod.rs` | 43 | `mod zone_debug` - only used in debug mode |
| `src/toys/scene_view.rs` | 31-32 | `right_view_4d_rotation` field - set but appears unused |
| `src/map/view.rs` | 13 | `MAP_KEYBOARD_SPEED` - appears unused, no keyboard handling in map |
| `src/map/mod.rs` | 44-46 | `TAP_RADIUS_*` constants - appear unused |
| `src/render/tesseract.rs` | 19 | `TETRA_FOCAL_LENGTH_SCALE` - could be simplified |
| `src/camera/projection.rs` | 29-32 | `project_direction` function - defined but not called |

---

## 7. INEFFICIENT PATTERNS

| File | Lines | Issue |
|------|-------|-------|
| `src/map/renderer.rs` | 82 | `waypoint_tap_zones: Vec<...>` - could use smallvec for small fixed count |
| `src/map/renderer.rs` | 404 | Clones whole polyhedron before each clipping iteration - could iterate with references |
| `src/render/batch.rs` | 14-15 | `mesh.reserve_triangles(128)` - magic numbers for capacity |
| `src/geometry.rs` | 102-103 | `let mut hull = Vec::new()` - could pre-allocate with capacity |
| `src/map/slice.rs` | 88 | Fixed-size array `[Vector4<f32>; 4]` copied for each face |

---

## 8. ANTI-PATTERNS: `new()` + Default duplication (13 instances)

| File | Lines | Issue |
|------|-------|-------|
| `src/toys/polytopes.rs` | 45-49 | `Default` delegates to `new()` - redundant |
| `src/toys/scene_view.rs` | 201-205 | Same pattern |
| `src/toys/debug_scratchpad.rs` | 11-15 | Same pattern |
| `src/render/style.rs` | 45-52 | `FourDSettings::default()` duplicates values that `new()` sets |
| `src/map/view.rs` | 146-150 | Same pattern |
| `src/map/renderer.rs` | 85-89 | Same pattern |
| `src/view/compass_view.rs` | 146-150 | Same pattern |
| `src/camera/mod.rs` | 60-64 | Same pattern |
| `src/render/projection.rs` | 176-190 | `StereoSettings::default()` re-sets values already set in `new()` |
| `src/rotation4d.rs` | 59-63 | Same pattern |
| `src/input/mod.rs` | 57-61 | Same pattern |
| `src/input/zone_debug.rs` | 17-21 | Same pattern |
| `src/toy/manager.rs` | 83-87 | Same pattern |

---

## 9. MISSING CONST (values that could be const)

| File | Lines | Value | Could be const |
|------|-------|-------|----------------|
| `src/render/tetra.rs` | 54 | `16.0` | Yes |
| `src/render/tetra.rs` | 81 | `14.0` | Yes |
| `src/render/tetra.rs` | 62 | `3.0` | Yes |
| `src/render/tetra.rs` | 64 | `20.0` | Yes |
| `src/render/tetra.rs` | 67 | `4.0` | Yes |
| `src/render/tetra.rs` | 91 | `15.0` | Yes |
| `src/render/tetra.rs` | 95 | `10.0` | Yes |
| `src/map/helpers.rs` | 51 | `0.12` | Yes |
| `src/map/helpers.rs` | 74 | `2.0` | Yes |
| `src/map/slice.rs` | 31 | `2.0` | Yes |
| `src/tetrahedron/gadget.rs` | 170 | `0.15` | Yes |

---

## Recommended Priority Order

1. **FIXED**: `new()` + Default anti-pattern (13 files) - Remove redundant Default impls
2. **IN PROGRESS**: Magic numbers - Convert to named constants
3. **PENDING**: Dead code - Remove unused fields and functions
4. **PENDING**: Unnecessary clones - Use references where possible
5. **LOWER PRIORITY**: Logic duplication - More complex refactoring