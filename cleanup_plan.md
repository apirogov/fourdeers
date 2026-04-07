# Cleanup Plan — FourDeers Codebase

Comprehensive audit of duplication, code quality, test coverage, and documentation issues
with proposed solutions for each.

---

## Phase 1: Code Duplication (Critical)

### #1 Zone-to-CameraAction mapping duplication — DONE
- **Solution applied:** Extracted `pub fn zone_to_movement_action(zone: Zone) -> Option<CameraAction>` into `src/input/zones.rs`. Both `app.rs` and `polytopes.rs` import and use it.

### #2 Repeated zone-checking boilerplate (7x in app.rs) — DONE
- **Solution applied:** `handle_tap_zone` restructured to compute `left_zone` once via `get_zone_from_rect`, then match on it. Extracted `handle_compass_tap` and `handle_map_tap` helper methods.

### #3 Arrow head drawing duplication (3x) — DONE
- **Solution applied:** Extracted `pub fn draw_arrow_head(...)` into `src/render.rs`. All 3 call sites (2 in render.rs, 1 in map.rs) use it.

### #4 Two `render_tetrahedron_with_projector` functions — SKIPPED
- **Reason:** The two functions diverge significantly in their details (alpha blending, center_3d offset, different projection paths, different label rendering). Forcing shared helpers would require many parameters, making them more complex than the current code.

### #5 Keyboard movement handling duplication — DONE
- **Solution applied:** Extracted `pub fn handle_movement_keys(ctx, speed, apply)` into `src/input/mod.rs`. Both `app.rs` and `polytopes.rs` delegate to it.

### #6 Text outline rendering pattern duplication (7x) — DONE
- **Solution applied:** Extracted `render_outlined_text` (single-outline) and `render_dual_outlined_text` (dual-offset outline) into `src/render.rs`. Applied across render.rs and map.rs.

### #7 Convex hull algorithm duplication — DONE
- **Solution applied:** `convex_hull_2d` (point-based) now delegates to `convex_hull_2d_indexed` (indexed), eliminating the duplicate gift-wrapping implementation.

### #8 `find_or_add` vertex dedup closure duplication — DONE
- **Solution applied:** Extracted `VertexDedup` struct with `find_or_add` method. Used in both `build_cross_section_polyhedron` and `clip_polyhedron_by_plane`.

---

## Phase 2: Magic Numbers & Shared Constants

### #9 `STEREO_SCALE_FACTOR` hardcoded in map.rs — DONE
- **Solution applied:** Made `STEREO_SCALE_FACTOR` pub in `render.rs`, map.rs now uses `crate::render::STEREO_SCALE_FACTOR`.

### #10 Default settings duplicated between render.rs and map.rs — DONE
- **Solution applied:** Extracted `DEFAULT_W_THICKNESS` and `DEFAULT_W_COLOR_INTENSITY` as pub constants in `render.rs`. MapRenderer uses them.

### #11 Remaining magic numbers — PARTIALLY DONE
- Most render.rs constants were already extracted in prior work. Some font sizes and offsets remain inline but are local to their functions.

---

## Phase 3: Long Functions & Structural Issues

### #12 Break up long functions — NOT STARTED
- `render_single_tetrahedron` and `PolytopesToy::render_sidebar` remain long. Lower priority since Phase 1 dedup already reduced overall line count.

### #13 9-parameter function in map.rs → parameter struct — DONE
- **Solution applied:** Created `struct TetraRenderParams<'a>` holding all parameters. Function takes `&TetraRenderParams`.

### #14 CompassFrameMode label duplication — DONE
- **Solution applied:** Added `CompassFrameMode::display_label()` method. Both call sites in `app.rs` use it.

### #15 View toggle pattern (4x) — DONE
- **Solution applied:** Added `fn toggle_view(&mut self, view: ActiveView)` helper to `FourDeersApp`.

### #16 Menu overlay frame duplication — DONE
- **Solution applied:** Extracted shared `panel_frame` variable used by both left and right menu areas.

---

## Phase 4: Missing Trait Implementations

### #17 Add `Display` impls — DONE
- `Zone` — `Display` impl added in `src/input/zones.rs`
- `CameraAction` — `Display` impl added in `src/camera.rs`
- `PolytopeType` — `Display` impl added in `src/polytopes.rs` (keeps `name()` for `&str` API)
- `RotationPlane` — `Display` impl added in `src/rotation4d.rs`

---

## Phase 5: Test Coverage

### #18 Add tests for core logic — IN PROGRESS
- **`src/app.rs`** (zero tests): Test zone detection, tap routing, view toggling
- **`src/toys/polytopes.rs`** (zero tests): Test zone_to_action, handle_tap, handle_hold
- **`src/toy/manager.rs`** (zero tests): Test toy switching, active_toy access, reset
- **`src/render.rs:w_to_color`**: Test color computation edge cases
- **`src/camera.rs`**: Test `project_3d_to_4d`, `get_slice_w_axis`, `get_4d_direction_label`

---

## Phase 6: Minor Issues

### #19 TetrahedronGadget Vec → fixed arrays — DEFERRED
- Larger API change touching construction code and iteration patterns. The existing `Vec` fields work correctly.

### #20 PolytopesToy pub fields — DONE
- `camera` and `drag_state` made private. No external accessors needed.

### #21 Duplicate `ui.add_space(4.0)` — DONE
- Removed duplicate call in `src/toys/polytopes.rs`.

---

## Phase 7: Documentation

### #22 Add doc comments to public APIs — NOT STARTED
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

1. ~~Phase 1 (#1-8): Code duplication~~ — DONE
2. ~~Phase 2 (#9-11): Magic numbers~~ — DONE
3. ~~Phase 4 (#17): Display impls~~ — DONE
4. ~~Phase 3 (#12-16): Long functions & structure~~ — MOSTLY DONE (#12 deferred)
5. Phase 5 (#18): Tests — IN PROGRESS
6. ~~Phase 6 (#19-21): Minor fixes~~ — MOSTLY DONE (#19 deferred)
7. Phase 7 (#22): Documentation — NOT STARTED
