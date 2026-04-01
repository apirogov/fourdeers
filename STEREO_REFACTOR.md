# Stereo Pipeline Refactor Plan

## Goal

One common 3D→stereo pipeline. All 3D render code is stereo-agnostic — it takes a `&StereoProjector` and calls `project_3d(x, y, z)` with no eye parameter. A single orchestration function handles splitting views and creating per-eye projectors.

## Changes

### 1. `StereoProjector` — bake eye offset at construction

- Add `eye_offset: f32` field
- `new()` creates mono projector (`eye_offset = 0`)
- `for_eye(..., eye_sign)` pre-computes `eye_sign * eye_separation * 0.5`
- `project_3d(x, y, z)` — **no `eye_sign` param**, uses `self.eye_offset` internally
- Remove `project_3d_no_eye` (mono projector handles it)
- `with_center()` propagates `eye_offset`

### 2. Add `render_stereo_views` — single orchestration point

```rust
pub fn render_stereo_views(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    eye_separation: f32,
    projection_distance: f32,
    mode: ProjectionMode,
    render_fn: impl Fn(&Painter, &StereoProjector, egui::Rect),
)
```

Splits rect, creates per-eye projectors, calls `render_fn` twice.

### 3. Render functions become stereo-agnostic

- `render_edges(painter, projector, transformed, clip_rect)` — no `eye_sign`
- `render_tetrahedron_with_projector(painter, gadget, projector, frame_mode)` — no `eye_sign`
- All 3D rendering: takes `(&Painter, &StereoProjector)`, zero stereo knowledge

### 4. Update callers

**Polytope toy:** calls `render_stereo_views` with closure that renders edges, then draws flat overlays (gadgets, labels) once on top.

**Compass overlay:** calls `render_stereo_views` with closure that renders the tetrahedron.

### 5. Remove dead code

- `EyeRenderOptions` struct
- `render_eye_view` method on `TesseractRenderContext`
- `render_stereo_tetrahedron_overlay`
- `project_3d_no_eye`
- `eye_separation`, `projection_distance`, `projection_mode` fields on `TesseractRenderContext`

### 6. `TesseractRenderContext` — purely 4D→3D

Only holds 4D transform state and w-slice config. Produces `TransformedVertex[]`. Draws edges given a projector. No stereo knowledge.

### 7. `render_single_tetrahedron` — unchanged

Flat overlay UI with inline projection. Not part of the 3D pipeline.

### 8. Update tests

All tests create per-eye projectors instead of passing `eye_sign` to `project_3d`.

## File impact

- `src/render.rs` — most changes (projector, orchestration, render functions, tests)
- `src/toys/polytopes.rs` — call `render_stereo_views` instead of `render_eye_view`
- `src/app.rs` — call `render_stereo_views` for compass instead of `render_stereo_tetrahedron_overlay`
