# Cleanup Plan 2: Second Pass

Based on the comprehensive audit of the codebase. Ordered by safety and impact.

## Phase 1: Quick Fixes & Constants Consolidation

1. **Fix camera dot radius bug** — `map/renderer.rs:497` uses hardcoded `4.0` instead of `MAP_CAMERA_DOT_RADIUS` (3.0). Use the named constant.
2. **Consolidate EDGE_STROKE_WIDTH** — Same constant (2.5) exists in both `map/mod.rs` and `render/tesseract.rs`. Move to a single location.
3. **Centralize map colors** — Move `SLICE_GREEN`, `DIM_GRAY`, `VISIBILITY_DARK_GREEN` from `map/mod.rs` to `colors.rs`.
4. **Extract hardcoded magic numbers to constants** in `map/renderer.rs`:
   - `1.5` → slice fill stroke width
   - `2.0` → slice edge stroke width
   - `2.0` → forward arrow stroke width
   - `10.0` → forward arrow head size
   - `0.1` → near threshold (use `NEAR_PLANE_THRESHOLD` from render)
5. **Extract hardcoded numbers in `app.rs`** — panel margin (12), font size (12.0), GRAY color for build info.
6. **Extract hardcoded `10.0` font size** in `render/tesseract.rs:285` zone labels to a named constant.
7. **Fix inline `use` in closure** in `render/tesseract.rs:256` — move to module-level import.
8. **Fix non-portable `build.rs`** — Use `chrono` crate or `humantime` for UTC timestamp instead of `date -u`.
9. **Consistent egui import style** — `colors.rs` uses `use egui::Color32` instead of `use eframe::egui::Color32`.

## Phase 2: Eliminate Double Projector Creation in Map Renderer

- `MapRenderer::render()` creates projectors inside `render_stereo_views`, then creates them again for `compute_waypoint_tap_zones`.
- Refactor: extract projector creation into a helper, pass projectors to both the render closure and the tap zone computation.

## Phase 3: Extract Duplicate Zone-Analysis Pattern

- The pattern of getting zone modes + calling `analyze_tap_in_stereo_view_with_modes` is repeated in `process_hold` and `handle_tap_zone` in `app.rs`.
- Extract into a shared method.

## Phase 4: Decompose `render_tetrahedron`

- Split the 143-line function (with `#[allow(clippy::too_many_lines)]`) into:
  - `render_tetra_edges`
  - `render_tetra_labels`
  - `render_tetra_arrow`

## Phase 5: Split `camera.rs` Tests into Separate Module

- Production code is ~360 lines, test code is ~770 lines.
- Move `#[cfg(test)] mod tests` to `camera/tests.rs`.

## Phase 6: Split `app.rs` into Submodules

- `app/mod.rs` — core struct, `ActiveView`, state types, `eframe::App` impl
- `app/pointer.rs` — `process_pointer_events`, `process_drag_or_hold`, `process_drag`, `process_hold`, `handle_pointer_up`
- `app/views.rs` — `render_compass_scene`, `render_map_scene`, `render_overlay_labels`
- `app/menu.rs` — `render_menu_overlay`, `draw_common_controls`
