# Architecture Review — FourDeers Codebase

## Module Map

```
src/
  app.rs          (~900 lines)  — Main application: UI, input routing, 3 view modes
  camera.rs       (~1130 lines) — Camera model with split 3D/4D rotation semantics
  colors.rs       (80 lines)   — Named color constants
  rotation4d.rs   (~760 lines)  — 4D rotation math (SO(4) via double quaternion)
  render/
    mod.rs        (~880 lines)  — Stereo rendering, projection, compass tetrahedron, UI helpers
    tesseract.rs  (~570 lines)  — Tesseract rendering pipeline, zone tetrahedron rendering
    ui.rs         (118 lines)   — UI primitives (background, dividers, text, arrow heads)
  map.rs          (~2100 lines) — 4D tesseract map view, cross-section geometry, slice rendering
  tetrahedron.rs  (~830 lines)  — Tetrahedron gadget for 4D direction visualization
  polytopes.rs    (~250 lines)  — 4D polytope type definitions + factory
  polytopes_data.rs (~1050 lines) — Generated vertex/index data (excluded from review)
  input/
    mod.rs        (67 lines)    — Shared keyboard handling + DragState
    zones.rs      (~422 lines)  — Touch zone analysis (4-way and 9-way)
    zone_debug.rs (261 lines)   — Debug overlay for zone boundaries
  toy/
    mod.rs        (~105 lines)  — Toy trait definition
    manager.rs    (~140 lines)  — Runtime toy switching via HashMap
    registry.rs   (~69 lines)   — Static toy registry (ID/name lookup)
  toys/
    polytopes.rs  (~501 lines)  — Main polytopes toy
    debug_scratchpad.rs (75 lines) — Empty scratchpad for experiments
  test_utils.rs   (9 lines)    — Shared test assertions
  lib.rs          (26 lines)   — Crate root
  main.rs         (26 lines)   — Native entry point
  wasm.rs         (39 lines)   — WASM entry point
```

Total: ~10,900 LOC (excluding generated data).

---

## Completed Quick Fixes (Phase 1)

| # | Fix | Files changed |
|---|-----|---------------|
| 1.1 | Added `Vertex4D::to_vector()` to eliminate 4 duplicate `Vector4::new(v.position[0],...)` conversions | `polytopes.rs`, `render/tesseract.rs`, `map.rs`, `render/mod.rs`, `toys/polytopes.rs` |
| 1.2 | Removed dead code: `with_auto_magnitude_label`, `StereoSettings::with_eye_separation/with_projection_mode`, `StereoProjector::center/with_center/with_scale`, `Rotation4D::basis_x/y/z` | `tetrahedron.rs`, `render/mod.rs`, `rotation4d.rs` |
| 1.3 | Renamed all `get_`-prefixed methods to follow Rust conventions (`get_tetrahedron_vertices` → `tetrahedron_vertices`, `get_vertex_3d` → `vertex_3d`, etc.) | `tetrahedron.rs`, `input/zones.rs`, `render/mod.rs`, `render/tesseract.rs`, `map.rs`, `app.rs` |
| 1.4 | Deduplicated 5 shared constants (`NEAR_PLANE_THRESHOLD`, `ARROW_STROKE_WIDTH`, `BASE_LABEL_FONT_SIZE`, `BASE_LABEL_OFFSET_Y`, `ARROW_END_DOT_RADIUS`) between `render/mod.rs` and `render/tesseract.rs` | `render/mod.rs`, `render/tesseract.rs` |
| 1.5 | Added `Camera::slice_rotation()` method, eliminating 4 duplicate `Rotation4D::new(UnitQuaternion::identity(), *q_right)` constructions | `camera.rs`, `map.rs` |
| 1.6 | Consolidated default camera position into `DEFAULT_CAMERA_POSITION` constant | `camera.rs` |
| 1.7 | Fixed toy registry default ID duplication: `ToyManager::new()` now derives default from `toy_id_order()[0]` | `toy/manager.rs` |
| 1.8 | Removed misleading `DragState` re-export from `toy/mod.rs`; `toys/polytopes.rs` now imports directly from `crate::input` | `toy/mod.rs`, `toys/polytopes.rs` |
| 1.9 | Simplified `compute_weighted_direction` to delegate to `compute_weighted_direction_3d` instead of constructing a full `TetrahedronGadget` | `tetrahedron.rs` |

---

## Remaining Issues

### 1. Three Tetrahedron Renderers (HIGH)

Three independent implementations render the same tetrahedron shape with different projection modes and styling:

| # | Location | Projection | Context |
|---|----------|-----------|---------|
| A | `render/mod.rs:305-436` | `StereoProjector` | Compass view |
| B | `render/tesseract.rs:422-572` | Inline perspective math | Tesseract zone view |
| C | `map.rs:722-837` | `StereoProjector` + 3D offset + alpha | Map waypoints |

All three follow the same rendering sequence: **edges → vertex labels (with component colors) → direction arrow → tip/base labels**. They share ~60% structural code but differ in projection method, styling constants, and feature flags.

**Recommendation:** Extract a `TetrahedronRenderer` with configurable styling and a projection trait/closure.

### 2. Code Duplication (remaining)

| # | Pattern | Sites | Severity |
|---|---------|-------|----------|
| 2A | `compute_weighted_direction_3d` and `compute_vector_arrow` share core weight-computation logic | `tetrahedron.rs:248-292, 325-347` | Medium |
| 2B | 3D→4D basis projection matrix multiply (`project_3d_to_4d` vs `project_camera_3d_to_world_4d`) — identical structure, differ only in which rotation's basis is used | `camera.rs:261-268, 276-286` | Medium |
| 2C | `normalize_4d_vector` and `compute_weighted_direction` are only used in tests (dead in production) | `tetrahedron.rs` | Low |
| 2D | `from_4d_vector` is never called in production code (25 test calls only) | `tetrahedron.rs` | Low |

### 3. API Design Issues

| # | Issue | Location |
|---|-------|----------|
| 3A | Wide `Toy` trait: 23 methods spanning 5 concerns (metadata, rendering, input, navigation, settings). `DebugScratchpadToy` provides no-op implementations for all 23. | `toy/mod.rs` |
| 3B | Push-based settings propagation: `FourDeersApp` copies settings into each toy every frame via `set_stereo_settings()` / `set_four_d_settings()`, requiring every toy to store a copy. | `app.rs`, `toys/polytopes.rs` |
| 3C | `handle_drag` takes `_is_left_view: bool` that `PolytopesToy` ignores (uses its own `drag_state.drag_view` instead) | `toy/mod.rs:44`, `toys/polytopes.rs:398` |
| 3D | `ToyManager::active_toy()` panics via `.expect()`, while similar queries return `Option` | `toy/manager.rs:30-34` |
| 3E | `PolytopesToy` has 18 fields; the 6 rotation angles could be a `RotationAngles` struct | `toys/polytopes.rs:25-44` |

### 4. Structural/Module Issues

| # | Issue | Location |
|---|-------|----------|
| 4A | `map.rs` is ~2100 lines, mixing rendering, geometry, coordinate transforms, and ~900 lines of tests | `map.rs` |
| 4B | `render/mod.rs` is still ~880 lines after submodule split — contains `StereoProjector`, settings types, compass tetrahedron rendering, and compass label logic | `render/mod.rs` |
| 4C | `app.rs` contains ~163 lines of pointer event processing that could be extracted to `input/pointer.rs` | `app.rs:290-452` |
| 4D | Dependency inversion: `tetrahedron.rs` (geometry primitive) imports `input::Zone` for `for_zone()` method | `tetrahedron.rs:10` |
| 4E | Two parallel drag-view tracking mechanisms: `FourDeersApp::drag_view` and `PolytopesToy::drag_state` | `app.rs:122`, `toys/polytopes.rs:39` |

---

## Refactoring Plan

### Phase 2: Tetrahedron Unification (Next)

Unify the three tetrahedron renderers into a shared `TetrahedronRenderer` with configurable parameters:
- Extract common rendering loop (edges, labels, arrow, tip/base)
- Parameterize projection (StereoProjector vs inline perspective vs offset projector)
- Parameterize styling (stroke widths, font sizes, alpha, color)
- Consolidate remaining per-renderer constants

### Phase 3: Structural Improvements (Future)

| # | Item | Effort |
|---|------|--------|
| 3.1 | Extract `project_3d_to_4d_with_basis` helper to unify projection methods in camera.rs | Easy |
| 3.2 | Extract geometry primitives from `map.rs` into `geometry.rs` | High |
| 3.3 | Further split `render/mod.rs` (projector, compass, settings) | Medium |
| 3.4 | Extract pointer event processing from `app.rs` to `input/pointer.rs` | Medium |
| 3.5 | Decompose `FourDeersApp` into view-specific state structs | Medium |
| 3.6 | Consider capability traits for Toy (e.g., `CompassProvider`, `MapProvider`) | High |
