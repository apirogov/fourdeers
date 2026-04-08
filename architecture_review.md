# Architecture Review — FourDeers Codebase

## Module Map

```
src/
  app.rs          (856 lines)  — Main application: UI, input routing, 3 view modes
  camera.rs       (1134 lines) — Camera model with split 3D/4D rotation semantics
  colors.rs       (80 lines)   — Named color constants
  rotation4d.rs   (767 lines)  — 4D rotation math (SO(4) via double quaternion)
  render.rs       (1608 lines) — Stereo rendering, projection, tesseract pipeline, UI helpers
  map.rs          (2507 lines) — 4D tesseract map view, cross-section geometry, slice rendering
  tetrahedron.rs  (979 lines)  — Tetrahedron gadget for 4D direction visualization
  polytopes.rs    (244 lines)  — 4D polytope type definitions + factory
  polytopes_data.rs (1054 lines) — Generated vertex/index data (excluded from review)
  input/
    mod.rs        (50 lines)   — Shared keyboard handling
    zones.rs      (422 lines)  — Touch zone analysis (4-way and 9-way)
    zone_debug.rs (261 lines)  — Debug overlay for zone boundaries
  toy/
    mod.rs        (120 lines)  — Toy trait + DragState
    manager.rs    (140 lines)  — Runtime toy switching via HashMap
    registry.rs   (68 lines)   — Static toy registry (ID/name lookup)
  toys/
    polytopes.rs          (506 lines) — Main polytopes toy
    debug_scratchpad.rs   (75 lines)  — Empty scratchpad for experiments
  test_utils.rs   (9 lines)   — Shared test assertions
  lib.rs          (25 lines)  — Crate root
  main.rs         (26 lines)  — Native entry point
  wasm.rs         (39 lines)  — WASM entry point
```

Total: ~10,977 LOC (excluding generated data).

---

## Trait Design: `Toy`

### Current Design

The `Toy` trait in `src/toy/mod.rs` has 18 methods covering:

- Identity: `name()`, `id()`
- Lifecycle: `reset()`
- UI: `render_sidebar()`, `render_scene()`, `render_toy_menu()`
- Input: `handle_tap()`, `handle_drag()`, `handle_hold()`, `handle_drag_start()`, `handle_keyboard()`
- State: `get_visualization_rect()`, `clear_interaction_state()`
- Compass integration: `compass_vector()`, `compass_reference_position()`, `compass_world_to_camera_frame()`, `compass_waypoints()`
- Map integration: `map_camera()`, `scene_geometry_bounds()`, `map_waypoints()`
- Settings: `set_stereo_settings()`, `set_four_d_settings()`, `zone_mode_for_view()`

Six methods have default implementations returning `None`/empty, making them opt-in.

### Assessment

**Good:** Clean separation of scene rendering, sidebar controls, input handling, and compass/map integration. Builder-pattern on `TetrahedronGadget` is idiomatic.

**Concerns:**

1. **18 methods is wide.** With only 2 implementations, this is acceptable, but any new capability adds more methods. A future direction would be capability traits (e.g., `CompassProvider`, `MapProvider`) that toys opt into, but not worth doing now.

2. **`get_visualization_rect` still has `get_` prefix** — inconsistent with the Phase 8F rename that covered camera/rotation4d methods.

3. **`handle_drag` receives `is_left_view: bool` that `PolytopesToy` ignores.** The implementation already knows which view via `self.drag_state.drag_view`. The parameter is redundant for this implementation.

4. **`DragState` lives in `toy/mod.rs`** but is a general input-tracking struct, not toy-specific. It belongs in `input/`.

---

## Struct Design

### `FourDeersApp` (17 fields, 856 lines)

Handles 3 distinct view modes (Main, Compass, Map). The fields break down as:

- Core app state: `toy_manager`, `menu_open`, `settings`, `visualization_rect`, `active_view`
- Compass-specific: `compass_rotation`, `compass_waypoint_index`, `compass_frame_mode`
- Map-specific: `map_renderer`, `map_frame_mode`, `map_rotation_3d`
- Input state: `last_tap_time`, `mouse_down_pos`, `mouse_down_time`, `is_drag_mode`, `drag_view`, `last_drag_pos`

**Recommendation:** Extract compass state into `CompassState` and map state into `MapState`. This would reduce `FourDeersApp` to ~11 fields and make view-specific state self-contained. Low priority — works fine as-is.

### `MapRenderer` (9 fields, 2507 lines)

**Issue:** Has `projection_distance` as a plain field that shadows `StereoSettings.projection_distance`. The `render()` method receives `StereoSettings` but the renderer also uses its own `self.projection_distance`. This dual-source is a latent inconsistency.

### `TetrahedronGadget` (979 lines)

**Issue:** Uses `Vec<TetrahedronVertex>` and `Vec<TetrahedronEdge>` which are always exactly 4 vertices and 6 edges. These could be fixed-size arrays `[TetrahedronVertex; 4]` and `[TetrahedronEdge; 6]`, avoiding per-frame heap allocation. Low priority — the gadget is small.

### `Pos3D` vs `nalgebra::Vector3`

**Issue:** The codebase has its own `Pos3D` struct that duplicates much of `Vector3<f32>`. Conversion methods (`to_vector3`, `from_vector3`) exist, but the duality means constant back-and-forth conversions. This was likely done for field-name clarity (`x`, `y`, `z` vs indexing). Replacing `Pos3D` with `Vector3<f32>` would be a large refactor for modest gain.

---

## Module Boundaries & Responsibility

### `render.rs` (1608 lines) — Too Many Concerns

Contains 6 distinct responsibilities:

1. **Stereo projection infrastructure** — `StereoProjector`, `ProjectedPoint`, `render_stereo_views`, `split_stereo_views`
2. **Tesseract rendering pipeline** — `TesseractRenderContext`, `TesseractRenderConfig`, `ObjectRotationAngles`
3. **Tetrahedron rendering** — `render_single_tetrahedron`, `render_tetrahedron_with_projector`, `TetraRenderSpec`
4. **UI primitives** — `draw_background`, `draw_center_divider`, `render_tap_zone_label`, `render_common_menu_half`
5. **Settings types** — `FourDSettings`, `StereoSettings`, `ProjectionMode`, `CompassFrameMode`
6. **Utilities** — `w_to_color`, `format_4d_vector_compact`, `draw_arrow_head`, `render_outlined_text`

**Recommendation:** Move the tesseract rendering pipeline into its own module (`src/render/tesseract.rs`) and the settings types into a shared location. The UI primitives could become `src/render/ui.rs`. This is a significant refactor — defer until there's a concrete reason.

### `map.rs` (2507 lines) — Largest File

Contains:
- Map rendering pipeline
- Cross-section geometry: `clip_polyhedron_by_plane`, `build_cross_section_polyhedron`, `convex_hull_2d_indexed`, `VertexDedup`
- Edge/vertex/label rendering helpers
- Slice computation: `SliceInfo`
- Frustum ray computation
- A duplicate `render_tetrahedron_with_projector` function

**Recommendation:** Extract geometry primitives into `src/geometry.rs`. This would remove ~400 lines from `map.rs` and make the clipping/hull code reusable.

---

## Code Duplication

### 1. `format_4d_vector` (camera.rs) vs `format_4d_vector_compact` (render.rs)

`camera.rs:401` has `format_4d_vector` (threshold 0.01, formats as `+X-Y`):
```
[0.0, 1.0, 0.0, 0.0] → "+Y"
[0.5, 0.5, 0.0, 0.0] → "+0.50X+0.50Y"
```

`render.rs:960` has `format_4d_vector_compact` (threshold 0.05, formats as `+X +Y`):
```
[0.0, 1.0, 0.0, 0.0] → "+Y"
[0.5, 0.5, 0.0, 0.0] → "+0.5X +0.5Y"
```

**Fix:** Consolidate into a single function with a threshold parameter in a shared location.

### 2. Duplicate `render_tetrahedron_with_projector`

`render.rs:984` uses `StereoProjector` for projection. `map.rs` has its own version with inline projection logic. They share ~60% structural similarity (edge loop, vertex label loop, arrow rendering, tip/base label rendering) but differ in projection mode.

**Assessment:** These were previously evaluated as too divergent to meaningfully abstract. The structural similarity is inherent to rendering the same shape. Not worth forcing into a shared function — the resulting abstraction would be harder to understand than the current duplication.

### 3. Registry metadata duplication

`registry.rs` has `toy_ids()` and `toy_name_by_id()` that manually duplicate what each toy's `id()` and `name()` already provides:

```rust
pub fn toy_ids() -> Vec<&'static str> {
    vec!["polytopes", "debug_scratchpad"]  // must match each toy's id()
}

pub fn toy_name_by_id(id: &str) -> Option<&'static str> {
    match id {
        "polytopes" => Some("Polytopes"),          // must match PolytopesToy::name()
        "debug_scratchpad" => Some("DebugScratchpad"), // must match DebugScratchpadToy::name()
        _ => None,
    }
}
```

**Fix:** Derive from toy instances at construction time.

### 4. `render_sidebar` angle reset duplication

`PolytopesToy::render_sidebar` (line 283-288) manually zeroes all 6 rotation angles instead of calling `self.reset_rotation_angles()`:

```rust
self.rot_xy = 0.0;
self.rot_xz = 0.0;
self.rot_yz = 0.0;
self.rot_xw = 0.0;
self.rot_yw = 0.0;
self.rot_zw = 0.0;
```

**Fix:** Replace with `self.reset_rotation_angles()`.

---

## Cross-Cutting Issues

### Settings Propagation

`FourDeersApp` pushes stereo/4D settings into the active toy every frame:
```rust
self.toy_manager.active_toy_mut().set_stereo_settings(&self.settings.stereo);
self.toy_manager.active_toy_mut().set_four_d_settings(&self.settings.four_d);
```

This is fragile — adding a new setting requires updating the push path. A pull-based approach (toys receive a reference to shared settings) would be more robust, but requires lifetime management. Acceptable for now.

### Dead Parameters

`SliceInfo::new` takes `_bounds` and `_map_camera` parameters that are completely unused. The struct only uses `scene_camera` and `w_thickness`. Remove them.

### Unused `move_along`

`Camera::move_along` uses `project_3d_to_4d` (full rotation), while `apply_action` uses `project_camera_3d_to_world_4d` (split frame model). `move_along` is only called from tests. It either:
- Should be deleted and tests rewritten to use `apply_action`
- Should have a doc comment explaining when it's appropriate (test utility)

### Arrow rendering magic numbers

Several small constants in `render.rs` are still hardcoded:
- `12.0` tip label offset (line 927, 1103)
- `12.0` compass tip font size (line 1109)
- `2.0`, `3.0`, `4.0` dot radii (lines 924, 937, 1086, 1113)

These should be named constants for consistency with the rest of the codebase.

---

## Refactoring Plan

### Phase 1: Low-Hanging Fruit (Quick Fixes)

These are single-location changes that improve consistency and eliminate dead code.

| # | Item | File(s) | Effort |
|---|------|---------|--------|
| 1.1 | Rename `get_visualization_rect` → `visualization_rect` in Toy trait + impls | `toy/mod.rs`, `toys/polytopes.rs`, `toys/debug_scratchpad.rs` | Trivial |
| 1.2 | Remove unused `_bounds` and `_map_camera` params from `SliceInfo::new` | `map.rs` | Trivial |
| 1.3 | Replace manual angle zeroing with `self.reset_rotation_angles()` | `toys/polytopes.rs` | Trivial |
| 1.4 | Derive registry metadata from toy instances | `toy/registry.rs`, `toy/manager.rs` | Easy |
| 1.5 | Move `DragState` from `toy/mod.rs` to `input/mod.rs` | `toy/mod.rs`, `input/mod.rs`, `toys/polytopes.rs` | Easy |

### Phase 2: Code Consolidation (Medium Effort)

These eliminate duplication across files.

| # | Item | File(s) | Effort |
|---|------|---------|--------|
| 2.1 | Consolidate `format_4d_vector` and `format_4d_vector_compact` into one function | `camera.rs`, `render.rs` | Medium |
| 2.2 | Extract `compute_weighted_direction` to use `compute_weighted_direction_3d` directly | `tetrahedron.rs` | Easy |
| 2.3 | Name remaining magic numbers in `render.rs` (tip offsets, dot radii, font sizes) | `render.rs` | Easy |

### Phase 3: Structural Improvements (Future, Larger Scope)

These would require more planning and testing.

| # | Item | File(s) | Effort |
|---|------|---------|--------|
| 3.1 | Split `render.rs` into submodules (projection, tesseract, ui) | `render.rs` → `render/mod.rs`, `render/tesseract.rs`, etc. | High |
| 3.2 | Extract geometry primitives from `map.rs` into `geometry.rs` | `map.rs` → `geometry.rs` | High |
| 3.3 | Decompose `FourDeersApp` into view-specific state structs | `app.rs` | Medium |
| 3.4 | Replace `Pos3D` with `Vector3<f32>` throughout | `tetrahedron.rs`, `render.rs`, `map.rs` | High |
| 3.5 | Change `TetrahedronGadget` to use fixed-size arrays | `tetrahedron.rs` | Medium |

Phase 1 and 2.3 should be done now. Phase 2.1-2.2 after that. Phase 3 is deferred until there's a concrete need.
