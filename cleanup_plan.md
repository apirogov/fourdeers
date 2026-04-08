# Cleanup Plan — FourDeers Codebase

## Prior Work (Completed)

Phases 1–7 from the original cleanup plan are done:
- 8 code duplication items resolved (shared helpers, parameter structs, VertexDedup)
- Magic number extraction, Display impls, toggle_view helper, menu dedup
- 202 tests (up from 172), doc comments on core APIs
- See git history for details

---

## Phase 8: Quality & Hygiene Audit

Comprehensive pass to bring the codebase to a professional standard.

---

### A. Clippy Hygiene (Mechanical, Zero-Risk)

Clippy pedantic/nursery reports ~1400+ warnings. The actionable ones (excluding the
1637 `unreadable_literal` in `polytopes_data.rs` which is raw vertex data):

#### A1. Add `#[must_use]` to pure functions (162 instances)

Clippy reports 102 methods, 39 functions, and 21 constructors that return values
without side effects. Every pure function should be annotated so the compiler warns
callers who ignore the return value.

**Scope:**
- `src/camera.rs` — ~25 methods: `forward_vector`, `right_vector`, `up_vector`,
  `get_4d_basis`, `get_slice_w_axis`, `is_slice_tilted`, `get_4d_direction_label`,
  `project_3d_to_4d`, `project_camera_3d_to_world_4d`, `world_vector_to_camera_frame`,
  `yaw_l`, `pitch_l`, `yaw_r`, `pitch_r`, etc.
- `src/rotation4d.rs` — ~20 methods: `identity`, `new`, `q_left`, `q_right`,
  `inverse`, `inverse_q_right_only`, `then`, `rotate_point`, `rotate_vector`,
  `basis_vectors`, `to_matrix`, `basis_x/y/z/w`, `is_pure_3d`, etc.
- `src/render.rs` — ~10 functions: `w_to_color`, `compute_stereo_scale`,
  `draw_arrow_head`, `render_outlined_text`, `render_dual_outlined_text`,
  `compass_vertex_label`, `StereoProjector::project_3d`, etc.
- `src/colors.rs` — all 16 color functions
- `src/polytopes.rs` — `Vertex4D::new`, `PolytopeType::name`, `short_name`,
  `vertex_count`, `edge_count`, `all`, `create_polytope`
- `src/tetrahedron.rs` — `TetrahedronGadget` accessor methods
- `src/input/zones.rs` — `Zone::is_cardinal`, `Zone::all`, `Zone::cardinals`,
  `zone_to_movement_action`, `get_zone_from_rect`, `analyze_tap_in_stereo_view_with_modes`

**Action:** Add `#[must_use]` to all public functions/methods that return a value
and have no side effects. Do NOT add it to methods that mutate state
(e.g. `apply_action`, `rotate`, `reset`).

#### A2. Fix truncating `f32 → u8` casts (20 instances)

Clippy: `casting f32 to u8 may truncate the value / may lose the sign`

These occur in color construction where `(alpha * 255.0) as u8` can produce values
outside `0..=255` if alpha is not clamped. The pattern appears in:
- `src/render.rs` — `w_to_color` (lines 285-293): the computed r/g/b values
  should be clamped before casting
- `src/map.rs` — 12 instances of `(alpha * N) as u8` in rendering functions

**Action:** Create a helper `fn to_u8_color(v: f32) -> u8` that clamps to `0.0..=255.0`
and rounds, then use it everywhere. Alternatively, for the alpha computations,
ensure alpha is always `0.0..=1.0` and use `(alpha * 255.0).round() as u8` with a
prior `.clamp(0.0, 1.0)` on alpha.

#### A3. Replace `u8 as f32` with `f32::from()` (6 instances)

Clippy: `casts from u8 to f32 can be expressed infallibly using From`

**Files:** `src/map.rs:651-660` in `lerp_color`:
```rust
let ar = a.r() as f32;  // → f32::from(a.r())
```

**Action:** Replace all `x as f32` (where x: u8) with `f32::from(x)`.

#### A4. Replace `.cloned()` with `.copied()` (3 instances)

For `Copy` types, `.copied()` is more idiomatic than `.cloned()`.

**Files:** Occur in iterator chains over `Option<&CopyType>`.

**Action:** Replace `.cloned()` with `.copied()` where the inner type is `Copy`.

#### A5. Remove redundant closures (5 instances)

Clippy: `redundant_closure` — `|x| f(x)` should be just `f`.

**Action:** Replace `|x| some_fn(x)` with `some_fn` where the closure is a direct
pass-through.

#### A6. Inline format variables (13 instances)

Clippy: `uninlined_format_args` — `format!("{}", x)` → `format!("{x}")`

**Action:** Inline all format variables. This is purely a style improvement.

#### A7. Mark eligible functions as `const fn` (58 instances)

Clippy: `this could be a const fn`

**Scope:** Pure functions in `colors.rs` (all 16), simple accessors in
`rotation4d.rs`, `camera.rs`, `tetrahedron.rs`, `polytopes.rs`.

**Action:** Add `const` to functions that compute a value from their arguments
with no runtime dispatch, heap allocation, or trait method calls.

#### A8. Add `#[allow(clippy::too_many_lines)]` for data functions (2)

The `create_120_cell` (845 lines) and `create_600_cell` (269 lines) functions
in `polytopes_data.rs` are lookup table generators — not logic that should be
split into smaller functions.

**Action:** Add `#[allow(clippy::too_many_lines)]` to both functions.

---

### B. Inline Color Construction → `colors.rs` (23 instances)

`map.rs` has 16 inline `Color32::from_*` calls, `render.rs` has 4, `tetrahedron.rs`
has 1. These should live in `colors.rs` as named constants or parameterized functions.

#### B1. Duplicates of existing color functions

| Location | Inline color | Should use |
|----------|-------------|------------|
| `render.rs:668` | `Color32::from_rgba_unmultiplied(200, 200, 200, 150)` | `colors::text_dim()` |
| `render.rs:929` | `Color32::from_rgba_unmultiplied(0, 0, 0, 180)` | `colors::outline_default()` |

#### B2. Named colors to extract to `colors.rs`

| Location | Value | Name |
|----------|-------|------|
| `map.rs:362` | `Color32::from_rgb(255, 230, 50)` | `AXIS_LABEL_YELLOW` |
| `map.rs:581` | `Color32::from_rgba_unmultiplied(255, 255, 255, dot_alpha)` | `MAP_CAMERA_DOT` (fn with alpha) |
| `map.rs:855` | `Color32::from_rgba_unmultiplied(255, 180, 80, a)` | `MAP_TIP_LABEL` (fn with alpha) |
| `map.rs:867` | `Color32::from_rgba_unmultiplied(200, 200, 220, a)` | `MAP_DISTANCE_LABEL` (fn with alpha) |
| `map.rs:125` | `Color32::from_rgba_unmultiplied(60, 180, 60, 40)` | (already `slice_green_fill()`) |
| `map.rs:131` | `Color32::from_rgba_unmultiplied(15, 70, 15, 100)` | (already `visibility_dark_green_fill()`) |

For colors that vary by alpha, create `fn name_with_alpha(a: u8) -> Color32` in
`colors.rs`.

#### B3. Colors constructed from computed values

`map.rs:722,763,813-814` construct colors from `edge_color.r()/g()/b()` with
alpha multiplication. These are contextual (depend on the edge color parameter),
so they should stay inline but use the `to_u8_color` helper from A2.

---

### C. `unwrap()` → `expect()` in Production Code (8 instances)

Non-test `unwrap()` calls that could panic without a useful message:

| File:Line | Code | Fix |
|-----------|------|-----|
| `map.rs:772` | `gadget.get_vertex_3d(...).unwrap()` | `.expect("edge references valid vertex index")` |
| `map.rs:777` | `gadget.get_vertex_3d(...).unwrap()` | `.expect("edge references valid vertex index")` |
| `map.rs:641` | `best.unwrap().1` inside loop | Use `if let Some((_, dist)) = best` |
| `map.rs:1072` | `hull_idx.last().unwrap()` | `.expect("hull_idx has >= 3 elements")` |
| `wasm.rs:12` | `web_sys::window().unwrap()` | `.expect("running in browser with window object")` |
| `wasm.rs:14` | `.document().unwrap()` | `.expect("window has document")` |
| `wasm.rs:16` | `.get_element_by_id(...).unwrap()` | `.expect("canvas element exists")` |
| `wasm.rs:18` | `.dyn_into().unwrap()` | `.expect("element is canvas")` |

**Action:** Replace each with `.expect("reason")`.

---

### D. Remaining Magic Numbers

#### D1. `map.rs` inline numeric literals (~15 instances)

| Location | Value | Purpose | Constant name |
|----------|-------|---------|---------------|
| line 835 | `15.0` | Arrow head scale multiplier | `MAP_ARROW_HEAD_SCALE` |
| line 810 | `10.0` (font size) | Vertex label monospace font | `MAP_VERTEX_FONT_SIZE` |
| line 854 | `9.0` (font size) | Tip label proportional font | `MAP_TIP_FONT_SIZE` |
| line 866 | `8.0` (font size) | Distance label proportional font | `MAP_DISTANCE_FONT_SIZE` |
| line 305 | `8.0` (font size) | Axis label monospace font | `MAP_AXIS_FONT_SIZE` |
| line 321 | `3.0` (dot radius) | Waypoint dot radius | `MAP_WAYPOINT_DOT_RADIUS` |
| line 537 | `3.0` (dot radius) | Camera position dot radius | `MAP_CAMERA_DOT_RADIUS` |
| line 580 | `4.0` (dot radius) | Camera center dot radius | `MAP_CAMERA_CENTER_DOT_RADIUS` |
| line 851 | `-12.0` (offset) | Tip label Y offset | `MAP_TIP_LABEL_OFFSET_Y` |
| line 863 | `12.0` (offset) | Distance label Y offset | `MAP_DISTANCE_LABEL_OFFSET_Y` |
| line 312 | `8.0` (offset) | Axis label Y offset | `MAP_AXIS_LABEL_OFFSET_Y` |
| line 355 | `4.0, -6.0` | Edge label offset | `MAP_EDGE_LABEL_OFFSET` |
| line 598 | `10.0` (font size) | Edge label font | `MAP_EDGE_FONT_SIZE` |
| line 158 | `3.0` (proj dist) | Default projection distance | Already `StereoSettings::default().projection_distance` — use that |

#### D2. `render.rs` tap-zone layout multipliers

`render.rs:607-648` uses proportional offsets (`0.5`, `0.7`, `0.4`) for positioning
tap-zone labels within their rects. These are layout-specific and sufficiently
self-documenting in context — leave as inline.

#### D3. `colors.rs` functions → `const` values

All 16 functions in `colors.rs` return compile-time-known values. They should be
`pub const` instead of `pub fn`. For example:

```rust
// Before:
#[inline]
pub fn label_default() -> Color32 { Color32::from_rgb(255, 180, 80) }

// After:
pub const LABEL_DEFAULT: Color32 = Color32::from_rgb(255, 180, 80);
```

**Note:** This requires updating all call sites from `label_default()` to
`LABEL_DEFAULT`. The `#[inline]` makes the performance identical, so this is
purely a readability/style change — but it makes the const-ness explicit.

**Action:** Convert all `colors.rs` functions to `pub const` values. Update all
call sites. Naming convention: `UPPER_SNAKE_CASE`.

---

### E. Long Function Decomposition (5 functions)

#### E1. `PolytopesToy::render_sidebar` — 176 lines → ~4 methods

Split into focused helper methods called from `render_sidebar`:

| Helper | Lines | Responsibility |
|--------|-------|----------------|
| `render_polytope_selector` | ~20 | ComboBox + cache invalidation |
| `render_position_controls` | ~50 | X/Y/Z/W position sliders |
| `render_rotation_controls` | ~50 | 6 rotation plane sliders + 4D toggle |
| `render_view_options` | ~30 | Stereo/debug/zone mode checkboxes |

#### E2. `render_ui` — 113 lines → extract `render_overlay_labels`

The tap-zone label rendering block (lines 435–506) is a self-contained concern:
it computes painters, then renders all the overlay labels for the current view.
Extract into `fn render_overlay_labels(&self, left_painter, right_painter, left_rect, right_rect)`.

#### E3. `render_single_tetrahedron` — 132 lines

Already uses the extracted `draw_arrow_head` and `render_dual_outlined_text` helpers.
The remaining length comes from the vertex label loop and value text loop.
These are sequential, self-contained blocks — extracting them into helpers would
add parameter-passing overhead without meaningful clarity gain.

**Action:** Add `#[allow(clippy::too_many_lines)]`. Not worth splitting further.

#### E4. `render_tetrahedron_with_projector` (render.rs) — 110 lines

Same situation as E3. The vertex label loop and arrow block are already as
decomposed as they can be.

**Action:** Add `#[allow(clippy::too_many_lines)]`.

#### E5. `render_tetrahedron_with_projector` (map.rs) — 115 lines

Same as E4. The edge loop, vertex label loop, arrow, and label blocks are
sequential rendering passes.

**Action:** Add `#[allow(clippy::too_many_lines)]`.

---

### F. Naming Inconsistencies

#### F1. Drop `get_` prefix from accessor methods

Rust convention: accessor methods don't use `get_` prefix.

**Affected methods in `camera.rs`:**
- `get_4d_basis()` → `basis_4d()` or `four_d_basis()`
- `get_slice_w_axis()` → `slice_w_axis()`
- `get_4d_direction_label()` → `direction_label_4d()` or `four_d_direction_label()`
- `get_q_left_as_yaw_pitch()` → `q_left_yaw_pitch()`
- `get_q_right_as_yaw_pitch()` → `q_right_yaw_pitch()`

**Affected methods in `rotation4d.rs`:**
- `get_w_component_of_basis()` → `basis_w_component()` (or just use `basis_w()`)

**Action:** Rename methods. Since this is an application (not a library), no
deprecation period needed — just rename and update all call sites.

#### F2. Remove `PolytopeType::name()` — now redundant with `Display`

The `name()` method was added before `Display`. Now that `Display` exists, all
callers should use `.to_string()` or `format!("{polytope_type}")` instead.

**Call sites:** `toys/polytopes.rs:131,134` — `.selected_text(ty.name())` and
`ui.selectable_value(..., ty.name())`.

**Action:** Replace `ty.name()` with `ty.to_string()` (or just `ty` where the
Display impl is used implicitly). Remove the `name()` method.

#### F3. `set_q_left_from_yaw_pitch` / `set_q_right_from_yaw_pitch`

These are fine as-is (the `set_` prefix is appropriate for mutation methods).

#### F4. `slice_green_fill` / `visibility_dark_green_fill` → const

These are already functions returning constant values (with fixed alpha). Once
D3 converts `colors.rs` to const, these should follow the same pattern.
However, they live in `map.rs` not `colors.rs` — move them to `colors.rs`
alongside the other color constants.

---

### G. Structural Notes (Not Actionable Now)

These are observations worth tracking but too large/risky to address in this pass:

1. **`app.rs` is 838 lines, 24 methods.** It handles UI rendering, input routing,
   keyboard handling, tap zones, menu, and compass logic. A proper decomposition
   would split it into `InputHandler`, `OverlayRenderer`, and `App` modules. This
   is a future refactor.

2. **`PolytopesToy` has 20 fields.** The sidebar directly mutates camera position
   fields via sliders. This mixes UI and model concerns. Acceptable for now.

3. **`TesseractRenderConfig`** has 15 fields. Could use the same parameter-struct
   treatment as `TetraRenderParams`, but `render.rs` only has one call site.

4. **`MapRenderer::render`** takes 8 parameters. Could use a parameter struct,
   but there's only one call site in `app.rs`.

---

## Execution Order

Each step: implement → `cargo fmt` → `cargo clippy` → `cargo test` → commit

1. **A. Clippy hygiene** — A1 through A8 (mechanical, zero-risk, ~30 min)
2. **B. Inline colors → `colors.rs`** — B1 through B3 (eliminates magic values)
3. **C. `unwrap()` → `expect()`** — 8 instances
4. **D. Magic numbers → named constants** — D1 and D3
5. **E. Long function decomposition** — E1 and E2 only (the real wins)
6. **F. Naming cleanup** — F1 and F2
7. **Final pass:** `cargo clippy -- -W clippy::all -W clippy::pedantic` and fix remaining
