# Cleanup Plan — FourDeers Codebase

Comprehensive audit of duplication, code quality, test coverage, and documentation issues
with proposed solutions for each.

---

## Phase 1: Code Duplication (Critical)

### #1 Zone-to-CameraAction mapping duplication
- **Files:** `src/toys/polytopes.rs:102-118`, `src/app.rs:760-776`
- **Problem:** Identical 8-arm match mapping Zone variants to CameraAction variants
- **Solution:** Extract `pub fn zone_to_movement_action(zone: Zone) -> Option<CameraAction>` into `src/input/zones.rs` (where Zone is defined). Both call sites import and use it.

### #2 Repeated zone-checking boilerplate (7x in app.rs)
- **File:** `src/app.rs:790-868` (`handle_tap_zone`)
- **Problem:** `left_rect.contains(pos) && get_zone_from_rect(left_rect, pos, ZoneMode::NineZones) == Some(Zone::...)` repeated 7 times
- **Solution:**
  1. Add `fn get_left_zone(rect: egui::Rect, pos: egui::Pos2) -> Option<Zone>` helper in `app.rs`
  2. Restructure `handle_tap_zone` to compute `left_zone` and `right_zone` once, then match on them

### #3 Arrow head drawing duplication (3x)
- **Files:** `src/render.rs:844-864`, `src/render.rs:1047-1059`, `src/map.rs:705-714`
- **Problem:** Identical arrow head triangle computation (direction, perpendicular, triangle vertices, convex_polygon shape)
- **Solution:** Extract `fn draw_arrow_head(painter: &egui::Painter, tip: egui::Pos2, direction: egui::Vec2, head_size: f32, color: egui::Color32)` into `src/render.rs` (pub), import in `map.rs`.

### #4 Two `render_tetrahedron_with_projector` functions
- **Files:** `src/render.rs:936` (4 params, public), `src/map.rs:722` (9 params, private)
- **Problem:** Both iterate gadget edges, project vertices, draw line segments, compute component colors, draw vertex labels with outlines, draw arrows with arrowheads, draw tip/base labels. The map.rs version adds center_3d offset, edge_color, alpha, distance_label, labels_visible.
- **Solution:** Extract shared sub-operations from the render.rs version into reusable internal functions:
  - `fn draw_gadget_edges(painter, gadget, projector, edge_color, near_threshold)`
  - `fn draw_gadget_vertex_labels(painter, gadget, projector, frame_mode, font_size, outline_color)`
  - `fn draw_gadget_arrow(painter, gadget, projector, arrow_stroke, head_scale, head_half_width, head_color)`
  - `fn draw_gadget_base_label(painter, projector, gadget, font_size)`
  Both `render_tetrahedron_with_projector` versions call these shared helpers. The map.rs version passes custom params.

### #5 Keyboard movement handling duplication
- **Files:** `src/app.rs:712-748`, `src/toys/polytopes.rs:451-479`
- **Problem:** Identical key-to-action mapping (ArrowUp→MoveUp, ArrowDown→MoveDown, etc.)
- **Solution:** Extract `fn handle_movement_keys(ctx: &egui::Context, speed: f32, mut apply: impl FnMut(CameraAction, f32))` into a shared location (e.g., `src/input/mod.rs` or `src/camera.rs`).

### #6 Text outline rendering pattern duplication (7x)
- **Files:** `src/render.rs` (6 occurrences), `src/map.rs` (1 occurrence)
- **Problem:** The triple-text-draw pattern (draw outline at +offset, draw outline at -offset, draw text at center) repeated everywhere
- **Solution:** Extract:
  ```rust
  pub fn render_outlined_text(
      painter: &egui::Painter,
      pos: egui::Pos2,
      align: egui::Align2,
      text: impl ToString,
      font_id: egui::FontId,
      text_color: egui::Color32,
      outline_color: egui::Color32,
  )
  ```
  With an optional `render_dual_outlined_text` variant for the two-outline-offset version used in vertex labels.

### #7 Convex hull algorithm duplication
- **File:** `src/map.rs:919-969` (indexed), `src/map.rs:1365-1411` (point-based)
- **Problem:** Same gift-wrapping algorithm implemented twice with different return types
- **Solution:** Implement one generic `convex_hull_2d<T>(points: &[T], to_coords: impl Fn(&T) -> (f32, f32)) -> Vec<usize>` that returns indices. The point-based version becomes a thin wrapper that indexes into the input.

### #8 `find_or_add` vertex dedup closure duplication
- **File:** `src/map.rs:898-905`, `src/map.rs:990-997`
- **Problem:** Identical closure for vertex deduplication with epsilon comparison
- **Solution:** Extract into a `VertexDedup` struct:
  ```rust
  struct VertexDedup {
      vertices: Vec<Vector3<f32>>,
      eps_sq: f32,
  }
  impl VertexDedup {
      fn find_or_add(&mut self, v: Vector3<f32>) -> usize { ... }
  }
  ```

---

## Phase 2: Magic Numbers & Shared Constants

### #9 `STEREO_SCALE_FACTOR` hardcoded in map.rs
- **Files:** `src/map.rs:212`, `src/map.rs:1076`
- **Problem:** Uses `0.35` directly instead of the named constant from `render.rs`
- **Solution:** Make `STEREO_SCALE_FACTOR` in `render.rs` pub, import in `map.rs`. Or expose `pub fn compute_stereo_scale(rect: egui::Rect) -> f32`.

### #10 Default settings duplicated between render.rs and map.rs
- **Files:** `src/render.rs:170-173`, `src/map.rs:133-135`
- **Problem:** `FourDSettings` defaults (`w_thickness: 2.5`, `w_color_intensity: 0.35`) and `StereoSettings::projection_distance: 3.0` are hardcoded in map.rs instead of using the derived Default impls
- **Solution:** Replace map.rs manual construction with `FourDSettings::default()` and `StereoSettings::default()`.

### #11 Remaining magic numbers
- **Files:** `src/render.rs` (~15 instances), `src/map.rs` (~15 instances)
- **Problem:** Font sizes, offsets, thresholds, alpha values, dot radii, label offsets, near-plane thresholds all inline
- **Solution:** Extract into named constants at module top level. Key ones:
  - render.rs: divider stroke width, label offsets, dot radii, near thresholds
  - map.rs: font sizes, label offsets, cross-section stroke widths, arrow head scale

---

## Phase 3: Long Functions & Structural Issues

### #12 Break up long functions
- `render_single_tetrahedron` (191 lines) → split into edge drawing, vertex label drawing, arrow drawing, base label drawing
- `PolytopesToy::render_sidebar` (202 lines) → split into position controls, rotation controls, polytope selector, view options
- `handle_tap_zone` (123 lines) → simplified by #2 (zone computed once, match-based dispatch)
- `render_ui` (132 lines) → extract `render_overlay_labels`, `render_map_controls`, `render_compass_controls`
- `render_menu_overlay` (67 lines) → extract `render_menu_half` helper
- `draw_common_controls` (67 lines) → extract toy selector, debug settings, 4D settings, stereo settings blocks

### #13 9-parameter function in map.rs → parameter struct
- **File:** `src/map.rs:722` — `render_tetrahedron_with_projector`
- **Solution:** Create `struct TetraRenderParams<'a>` with builder-pattern or struct literal syntax. Also `MapRenderer::render` (8 params).

### #14 CompassFrameMode label duplication
- **File:** `src/app.rs:478-481`, `src/app.rs:488-491`
- **Problem:** Same `match { World => "Frame: World", Camera => "Frame: Camera" }` twice
- **Solution:** Add `fn display_label(&self) -> &'static str` to `CompassFrameMode` (done alongside Display impl)

### #15 View toggle pattern (4x)
- **File:** `src/app.rs:97-109, 800-815`
- **Problem:** `if self.active_view == X { Main } else { X }` repeated 4 times
- **Solution:** Add `fn toggle_view(&mut self, view: ActiveView)` helper

### #16 Menu overlay frame duplication
- **File:** `src/app.rs:562-612`
- **Problem:** Left and right menu areas use identical Area + Frame blocks
- **Solution:** Extract `fn render_menu_panel(ui, id, rect, content_fn)`

---

## Phase 4: Missing Trait Implementations

### #17 Add `Display` impls
- `Zone` — currently stringified ad-hoc in 3+ places
- `CompassFrameMode` — manual match for "World"/"Camera"
- `CameraAction` — no human-readable format
- `PolytopeType` — `name()` method duplicates what Display should provide
- `RotationPlane` — useful for debug/logging

---

## Phase 5: Test Coverage

### #18 Add tests for core logic
- **`src/app.rs`** (902 lines, zero tests): Test zone detection, tap routing, view toggling, keyboard handling
- **`src/toys/polytopes.rs`** (547 lines, zero tests): Test zone_to_action, handle_keyboard, handle_tap, handle_hold
- **`src/toy/manager.rs`** (8 public methods, zero tests): Test toy switching, active_toy access, reset
- **`src/render.rs:w_to_color`**: Test color computation edge cases
- **`src/camera.rs`**: Test `project_3d_to_4d`, `get_slice_w_axis`, `get_4d_direction_label`

---

## Phase 6: Minor Issues

### #19 TetrahedronGadget Vec → fixed arrays
- **File:** `src/tetrahedron.rs`
- **Problem:** `Vec<TetrahedronVertex>` for always-4 items, `Vec<TetrahedronEdge>` for always-6
- **Solution:** Use `[TetrahedronVertex; 4]` and `[TetrahedronEdge; 6]`

### #20 PolytopesToy pub fields
- **File:** `src/toys/polytopes.rs:25-44`
- **Problem:** `pub camera` and `pub drag_state` leak implementation details
- **Solution:** Make private, add accessors where needed

### #21 Duplicate `ui.add_space(4.0)`
- **File:** `src/toys/polytopes.rs:334-336`
- **Solution:** Remove duplicate call

---

## Phase 7: Documentation

### #22 Add doc comments to public APIs
Priority order (highest impact first):
1. `src/input/zones.rs` — Zone, ZoneMode, TapAnalysis are core types used everywhere
2. `src/toy/mod.rs` — Toy trait (15 methods) is the extension point
3. `src/render.rs` — 20+ public rendering functions
4. `src/camera.rs` — 25+ public methods
5. `src/rotation4d.rs` — 28+ public methods
6. `src/colors.rs` — 16 public color functions

---

## Execution Order

Each step should be: implement → `cargo fmt` → `cargo clippy` → `cargo test` → commit

1. Phase 1 (#1-8): Code duplication — highest impact, reduces line count
2. Phase 2 (#9-11): Magic numbers — quick wins
3. Phase 4 (#17): Display impls — quick, enables Phase 3 simplifications
4. Phase 3 (#12-16): Long functions & structure — depends on Phase 1
5. Phase 5 (#18): Tests — depends on Phases 1-4 being stable
6. Phase 6 (#19-21): Minor fixes
7. Phase 7 (#22): Documentation
