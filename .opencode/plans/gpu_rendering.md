# GPU Rendering Optimization Plan

Status: **PLANNING**

## Context

The app renders 4D polytopes as 2D line segments through egui's CPU-side Painter API.
Each `painter.line_segment()` call creates a separate `Shape`, which egui's tessellator
processes individually. For a tesseract (32 edges × 2 eyes), this creates 64+ individual
shapes per frame. The 120-cell would create ~2400. The per-shape overhead (allocation,
tessellation dispatch, GPU draw call batching) is the bottleneck.

There are two approaches: **A** (batched Mesh — bypass per-shape overhead) and **B**
(PaintCallback with custom GPU rendering — bypass egui's pipeline entirely).

---

## Approach A: Batched Mesh Rendering

### Goal
Replace all individual `Shape::line_segment` submissions with single pre-built `epaint::Mesh`
objects. A Mesh is a raw triangle buffer submitted as one GPU draw call.

### Key API

```rust
// epaint::Vertex — 160 bits each
pub struct Vertex {
    pub pos: Pos2,       // screen coordinates
    pub uv: Pos2,        // WHITE_UV for untextured
    pub color: Color32,  // premultiplied alpha
}

// Build mesh, submit as Shape
let mut mesh = Mesh::default();
// ... add vertices and triangle indices ...
painter.add(mesh);  // single GPU draw call
```

A line segment (A→B, width W, color C) becomes a quad (4 vertices, 2 triangles, 6 indices):
```
    v1 ---- v3
    |  \    |
    |   \   |
    |    \  |
    v0 ---- v2
```

### Shape Breakdown (what to batch)

**HIGH IMPACT — wireframe edges:**
| Source | Current shapes/frame | After batching |
|--------|---------------------|----------------|
| `render_edges()` (tesseract) | up to 64 line_segments | 1 Mesh |
| `render_tetra_edges()` (map tetrahedra) | ~42 line_segments | 1 Mesh |
| `draw_slice_volume` cs_edges | up to 24 line_segments | 1 Mesh |
| `draw_camera_position` arrow | 2 line_segments | same Mesh |
| Zone debug boundaries | 4-8 line_segments | 1 Mesh |
| Scene view zone tetra edges | ~48 line_segments | 1 Mesh |

**MEDIUM IMPACT — filled polygons:**
| Source | Current | After |
|--------|---------|-------|
| Slice fill convex_polygon | 0-2 PathShapes | 1 Mesh (fan triangulation) |
| Visibility polygon | 0-2 PathShapes | same Mesh |
| Arrow heads | 0-8 convex_polygons | same Mesh |

**LOW IMPACT (leave as-is):**
- `painter.text()` — text is complex, egui handles it well
- `painter.circle_filled()` — small count (dots), not worth batching
- `draw_background`, `draw_center_divider` — 2 calls total

### Architecture: `LineBatch` utility

New file: `src/render/batch.rs`

```rust
pub(crate) struct LineBatch {
    mesh: Mesh,
    stroke_width: f32,
}

impl LineBatch {
    pub fn new(stroke_width: f32) -> Self;
    pub fn add_segment(&mut self, a: Pos2, b: Pos2, color: Color32);
    pub fn add_circle_filled(&mut self, center: Pos2, radius: f32, color: Color32);
    pub fn add_convex_polygon_filled(&mut self, points: &[Pos2], color: Color32, stroke: Stroke);
    pub fn is_empty(&self) -> bool;
    pub fn into_mesh(self) -> Mesh;
    pub fn submit(self, painter: &Painter);  // painter.add(mesh)
}
```

`add_segment` builds the 4-vertex, 2-triangle quad inline.
`add_circle_filled` builds a fan of triangles (center + perimeter vertices).
`add_convex_polygon_filled` builds a triangle fan from the first vertex.

### Phase A1: Create `LineBatch` utility

- New file `src/render/batch.rs`
- `LineBatch` struct with pre-allocation
- Unit tests for geometry correctness (vertex positions, winding, colors)

### Phase A2: Batch tesseract wireframe edges

Replace `render_edges()` in `src/render/tesseract.rs`:
- Instead of collecting `Vec<egui::Shape>` and calling `painter.extend()`
- Build a `LineBatch` with all visible edges
- Submit as single `painter.add(mesh)`

### Phase A3: Batch tetrahedron edges

Replace `render_tetra_edges()` in `src/render/tetra.rs`:
- Accept a `&mut LineBatch` parameter instead of calling `painter.line_segment()` directly
- Caller creates the batch and submits it after all tetrahedra are rendered

This changes the tetrahedron rendering API:
```rust
fn render_tetra_edges(batch: &mut LineBatch, projector: ..., edges: ..., style: ...);
```

### Phase A4: Batch map renderer lines

In `src/map/renderer.rs`, the draw methods should:
1. Create a `LineBatch` at the start of each eye's render pass
2. Accumulate all line segments: wireframe edges, cs_edges, tetra edges, arrow shafts
3. Submit one Mesh at the end

### Phase A5: Batch convex polygons

Add `add_convex_polygon_filled` to `LineBatch` for:
- Slice cross-section fill
- Visibility frustum polygon
- Arrow heads (small triangles)

### Estimated Impact

- **Before**: ~150-200 individual `Shape` submissions per frame → ~150-200 tessellation passes
- **After**: ~3-5 Mesh submissions per frame (lines, fills, text/UI stays as-is)
- **Speedup**: ~10-20x for the wireframe rendering path, ~3-5x overall frame time

---

## Approach B: PaintCallback with Custom GPU Rendering

### Goal
Bypass egui's shape pipeline entirely for the wireframe. Upload vertex data to GPU
buffers and render with custom vertex/fragment shaders. The vertex shader does the
4D→3D→2D projection; the fragment shader handles coloring.

### Challenge: ~~Dual Backend~~ Single wgpu Backend

The project uses **wgpu on both native and web** (WebGPU primary, WebGL fallback).
`PaintCallback` uses `egui_wgpu::Callback` on both platforms — no backend abstraction needed.

### Architecture

```
src/gpu/
├── mod.rs              — public API: GpuRenderer
├── vertex.rs           — vertex data types
└── shaders/
    └── wireframe.wgsl  — WGSL vertex/fragment shaders
```

### Core types

```rust
pub(crate) struct GpuVertex {
    pub pos: [f32; 4],    // 4D position
    pub color: [f32; 4],  // RGBA
}
```

The renderer is created once (during `App::new` via `CreationContext::wgpu_render_state()`)
and stored in the app. Each frame:
1. CPU computes which edges are visible (slice culling, near-plane culling)
2. GPU vertex buffer is updated with visible edge endpoints
3. Vertex shader does 4D→3D projection (camera transform) and 3D→2D (perspective)
4. Fragment shader does w-based coloring

### Phase B1: Wgpu backend (both native and web)

1. Get `wgpu::Device` + `wgpu::Queue` from `CreationContext::wgpu_render_state()`
2. Create WGSL shaders
3. Create render pipeline, vertex/index buffers
4. Implement `egui_wgpu::CallbackTrait` with `prepare()` (buffer upload) and `paint()` (draw)

### Phase B2: Renderer initialization

In `App::new()`:
```rust
let gpu_renderer = if let Some(rs) = cc.wgpu_render_state() {
    Some(GpuRenderer::new(rs))
} else {
    None
};
```

Store in the app. When rendering, check if GPU renderer is available:
- Yes → submit `PaintCallback` with the GPU renderer
- No (fallback) → use the batched Mesh approach from Approach A

### Phase B3: GPU-side vertex transformation

Move the 4D→3D projection into the vertex shader (WGSL):

```wgsl
struct Uniforms {
    mat_4d: mat4x4<f32>,
    offset_4d: vec4<f32>,
    mat_3d: mat3x3<f32>,
    proj_dist: f32,
    viewport: vec2<f32>,
    eye_offset: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos_4d: vec4<f32>,
    @location(1) color_in: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let r = uniforms.mat_4d * input.pos_4d - uniforms.offset_4d;
    let xyz = uniforms.mat_3d * r.xyz;
    let z_offset = uniforms.proj_dist + xyz.z;
    let scale = uniforms.proj_dist / z_offset;
    out.clip_pos = vec4<f32>(
        uniforms.viewport.x * 0.5 + xyz.x * scale + uniforms.eye_offset,
        uniforms.viewport.y * 0.5 - xyz.y * scale,
        0.0, 1.0
    );
    out.color = input.color_in;
    return out;
}
```

This eliminates ALL per-vertex CPU work. The CPU only uploads the raw 4D vertex positions
and edge indices once (or updates when the polytope changes).

### Estimated Impact

- **Before A+B**: ~200 CPU-side shape submissions → tessellation → GPU per frame
- **After A+B**: ~2-3 PaintCallback submissions (left eye, right eye) + ~5 text shapes
- **Speedup**: ~50-100x for wireframe rendering. Frame time dominated by egui UI text.

### Risks and Considerations

1. **WebGPU fallback**: wgpu's WebGL fallback via `webgl` feature ensures older browsers work.
   WebGPU is available in Chrome 113+, Edge 113+. Firefox/Safari support is in progress.
2. **State leakage**: Custom GPU rendering must not corrupt egui's GPU state. The wgpu
   callback's `paint()` method receives the render pass and must restore state.
3. **MSAA**: Custom rendering needs to match egui's anti-aliasing settings.
4. **Stereo**: Each eye needs its own PaintCallback with different `eye_offset` uniform.
   The vertex data is shared (same VBO), only the uniform changes.
5. **Fallback**: Approach A's batched Mesh must remain as fallback for any platform where
   GPU renderer initialization fails.

---

## Execution Order

1. **Phase A1-A5**: Batched Mesh (simpler, immediate benefit, no GPU code)
2. **Phase B1-B3**: PaintCallback GPU rendering via wgpu (single backend, WGSL only)

Each phase: implement → cargo fmt/clippy/test → just wasm → commit.
