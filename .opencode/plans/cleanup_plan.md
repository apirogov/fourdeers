# Cleanup Plan — Pre-Feature Codebase Hardening

## Phase 1: Quick Fixes (low risk, high impact)

### 1.1 Gate dead Camera methods with `#[cfg(test)]`
- `basis_4d()` — never called in production
- `direction_label_4d()` — only called in tests
- `is_slice_tilted()` — only called in tests
- `slice_w_axis()` — only called in tests
- **File**: `src/camera.rs`
- Also gate `SliceDirection` enum if only used by `direction_label_4d`

### 1.2 Remove dead `Rotation4D::from_axis_angle_3d()`
- Identical to `from_3d_rotation()`, only used in one test
- Update that test to use `from_3d_rotation()` instead
- **File**: `src/rotation4d.rs`

### 1.3 Remove dead TetrahedronGadget accessors
- `tip_label()` and `base_label()` never called (fields accessed directly)
- **File**: `src/tetrahedron.rs`

### 1.4 Remove dead `MapRenderer::camera()`
- Never called externally
- **File**: `src/map.rs`

### 1.5 Fix hardcoded `0.35` in `compute_frustum_rays`
- Replace `* 0.35` with `* crate::render::STEREO_SCALE_FACTOR`
- **File**: `src/map.rs:835`

### 1.6 Fix duplicated rect splitting in app.rs
- Replace inline rect computation at `app.rs:596-603` with call to `split_stereo_views()`
- **File**: `src/app.rs`

### 1.7 Unify rotation sensitivity constant
- `ROTATION_SENSITIVITY` in `camera.rs` and `COMPASS_ROTATION_SENSITIVITY` in `app.rs` both equal `0.005`
- Move to `camera.rs` as `pub const ROTATION_SENSITIVITY`, import in `app.rs`
- **Files**: `src/camera.rs`, `src/app.rs`

### 1.8 Fix dead binding in `render_single_tetrahedron`
- Change `if let Some(label) = spec.base_label { ... let _ = label; }` to `if spec.base_label.is_some()`
- **File**: `src/render/tesseract.rs:405-409`

### 1.9 Unify default projection distance constant
- `3.0` hardcoded in both `MapRenderer::new()` and `StereoSettings::default()`
- Extract to `render/mod.rs` as `pub const DEFAULT_PROJECTION_DISTANCE: f32 = 3.0`
- **Files**: `src/render/mod.rs`, `src/map.rs`

---

## Phase 2: Introduce `Bounds4D` type

### 2.1 Define `Bounds4D` struct
```rust
pub struct Bounds4D {
    pub min: Vector4<f32>,
    pub max: Vector4<f32>,
}
```
- Add helper methods: `contains()`, `padded(factor: f32)`, `is_degenerate()`
- **File**: `src/geometry.rs` (already exists for geometry utilities)

### 2.2 Migrate all `(Vector4, Vector4)` bounds to `Bounds4D`
- `compute_bounds()` return type
- `normalize_to_tesseract()` parameter
- `direction_to_tesseract()` parameter
- `MapRenderer::render()` parameter
- `MapRenderer::reset_to_fit()` parameter
- `Toy::scene_geometry_bounds()` return type
- `PolytopesToy::scene_geometry_bounds()` return type
- All test sites
- **Files**: `src/map.rs`, `src/geometry.rs`, `src/toy/mod.rs`, `src/toys/polytopes.rs`, `src/app.rs`

---

## Phase 3: Extract zone-to-position mapping in tesseract.rs

### 3.1 Create shared `zone_layout_entries()` function
- Both `render_zone_labels()` and `render_tetrahedron_gadget()` build the same 8-entry zone→(basis_vector, Zone, x, y) mapping
- Extract to a function returning `Vec<([f32; 4], Zone, f32, f32)>`
- **File**: `src/render/tesseract.rs`

---

## Phase 4: Standardize `new()`/`Default` delegation

Rust convention: `Default::default()` should delegate to `new()`.

### 4.1 Fix types where `new()` delegates to `default()` (backwards)
- `Camera`: `new() -> Self::default()` → flip so `default()` calls `new()`
- `StereoSettings`: `new() -> Self::default()` → flip
- `DragState`: `new() -> Self::default()` → flip
- `ZoneDebugOptions`: `new() -> Self::default()` → flip

Already correct (default calls new): `MapRenderer`, `PolytopesToy`, `DebugScratchpadToy`, `ToyManager`

---

## Phase 5: Split `map.rs` into submodules

### 5.1 Create `src/map/` directory structure
```
src/map/
├── mod.rs          — re-exports, constants, pub types
├── renderer.rs     — MapRenderer struct and methods
├── transform.rs    — MapViewTransform
├── bounds.rs       — Bounds4D, compute_bounds, normalize_to_tesseract, direction_to_tesseract
├── slice.rs        — SliceInfo, compute_slice_cross_section, compute_cross_section_edges, compute_in_band_segments
├── visibility.rs   — compute_frustum_rays, compute_frustum_planes, build_cross_section_polyhedron, clip_segment_to_screen, convex_hull_screen
└── helpers.rs      — lerp_color, render_tetrahedron_in_map, edge_axis
```

### 5.2 Split plan (by line ranges in current map.rs)

| Lines | Content | Target Module |
|-------|---------|---------------|
| 1-28 | Module doc comment, imports | `mod.rs` |
| 30-57 | Constants (SLICE_GREEN, DIM_GRAY, etc.) | `mod.rs` |
| 59-94 | Test-only constants, TESSERACT_FACES, AXIS_CHARS | `mod.rs` |
| 96-109 | `edge_axis()` | `helpers.rs` |
| 111-653 | `MapRenderer` struct + all methods | `renderer.rs` |
| 655-668 | `lerp_color()` | `helpers.rs` |
| 670-698 | `SliceInfo` | `slice.rs` |
| 700-766 | `render_tetrahedron_in_map()` | `helpers.rs` |
| 767-805 | `MapViewTransform` | `transform.rs` |
| 807-826 | `build_cross_section_polyhedron()` | `visibility.rs` |
| 828-897 | `compute_frustum_rays`, `compute_frustum_planes` | `visibility.rs` |
| 898-962 | `compute_bounds`, `normalize_to_tesseract`, `direction_to_tesseract` | `bounds.rs` |
| 963-996 | `clip_segment_to_screen` | `visibility.rs` |
| 998-1062 | `compute_slice_cross_section`, `compute_cross_section_edges` | `slice.rs` |
| 1063-1117 | `compute_in_band_segments` (test-only) | `slice.rs` |
| 1118-1125 | `convex_hull_screen` | `visibility.rs` |
| Tests | Split into corresponding `#[cfg(test)] mod tests` in each submodule | Each module |

### 5.3 Module visibility
- `mod.rs` re-exports: `MapRenderer`, `compute_bounds`, `normalize_to_tesseract`
- Internal modules use `pub(super)` or `pub(crate)` as appropriate
- `SliceInfo`, `MapViewTransform`, visibility functions → `pub(crate)` (only used within `map/` and `app.rs`)

---

## Phase 6: Extract tetrahedron rendering from render/mod.rs

### 6.1 Create `src/render/tetra.rs`
Move the following from `render/mod.rs`:
- `TetraLabelMode` enum
- `TetraStyle` struct + `compass()`, `zone_tetra()` constructors
- `render_tetrahedron()` function
- `render_tetrahedron_with_projector()` function
- `compass_vertex_label()` function
- Related constants: `NEAR_PLANE_THRESHOLD`, `ARROW_STROKE_WIDTH`, `BASE_LABEL_FONT_SIZE`, `BASE_LABEL_OFFSET_Y`, `ARROW_END_DOT_RADIUS`

### 6.2 Update re-exports
- `render/mod.rs` re-exports everything from `tetra.rs`
- Consumers unchanged (they import from `crate::render::`)

---

## Execution Order

1. Phase 1 (all 9 items) → commit
2. Phase 2 (Bounds4D) → commit
3. Phase 3 (zone positions) → commit
4. Phase 4 (new/Default) → commit
5. Phase 5 (map.rs split) → commit
6. Phase 6 (tetra.rs extract) → commit

Each phase: `cargo fmt` → `cargo clippy` → `cargo test` → `just wasm` → commit
