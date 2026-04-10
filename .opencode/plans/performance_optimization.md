# Performance Optimization Plan

Status: **COMPLETE** (Phases 1-4 implemented)

Goal: Reduce per-frame heap allocations and duplicated computation in the rendering pipeline,
particularly for map view stereo rendering on mobile/WASM targets.

## Phase 1: Quick wins — DONE (`2d3ba5a`)

- **F8**: Remove `screen_pts.clone()` in `src/map/renderer.rs` — move value into `convex_polygon()` since it's never reused.
- **F5**: Change `camera: Camera` to `&'a Camera` in `TesseractRenderContext` (`src/render/tesseract.rs`) — avoids cloning Camera every frame.

## Phase 2: Stack allocations in hot loop — DONE (`acf55b1`)

- **F3**: Replace 3 `Vec` heap allocations per face in `compute_cross_section_edges` (`src/map/slice.rs`) with stack arrays `[Vector4<f32>; 4]`, `[f32; 4]`, `[Vector4<f32>; 4]`. Eliminates 72-144 heap allocs/frame.

## Phase 3: Deduplicate CameraProjection — DONE (`8e8098d`)

- **F4**: Compute `CameraProjection::new()` once at top of `MapRenderer::render()`, thread it through all sub-methods instead of reconstructing 9x/frame (each involves quaternion inverse + matrix conversion).

## Phase 4: Hoist eye-independent work out of map eye loop — DONE (`ca1b18e`)

- **F6**: Restructure `MapRenderer::render()` eye loop — compute 4D cross-section, transform_vertices, and convex hull *once* before the loop; only do per-eye 2D projection + painting inside.
- **F1**: `transform_vertices()` in map view now called once outside the eye loop (matching SceneView pattern).
- **F7**: Pre-format waypoint distance labels once before the eye loop, not 2x per frame.

## Phase 5 (not yet scheduled): Stream shapes in render_edges

- **F2**: Replace `collect::<Vec<Shape>>()` + `painter.extend()` in `render_edges()` with a direct streaming approach.
