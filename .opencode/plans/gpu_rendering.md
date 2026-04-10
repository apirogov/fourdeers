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

### Challenge: Dual Backend

The project uses **wgpu on native** and **glow/WebGL on web** (eframe defaults).
`PaintCallback` is backend-specific:
- Native: `egui_wgpu::Callback` (implements `CallbackTrait`)
- Web: `egui_glow::CallbackFn`

We need a cross-backend abstraction.

### Architecture

```
src/gpu/
├── mod.rs              — public API: GpuRenderer trait
├── vertex.rs           — vertex data types (shared)
├── wgpu_backend.rs     — native rendering via wgpu
├── glow_backend.rs     — web rendering via glow/WebGL
└── shaders/
    ├── wireframe.vert   — GLSL vertex shader (shared logic)
    ├── wireframe.frag   — GLSL fragment shader (shared logic)
    └── wireframe_wgsl.wgsl — WGSL equivalent for wgpu
```

### Core abstraction

```rust
pub(crate) trait GpuRenderer: Send + Sync + 'static {
    fn upload_vertices(&mut self, vertices: &[GpuVertex]);
    fn upload_indices(&mut self, indices: &[u32]);
    fn render(&self, info: PaintCallbackInfo);
}

pub(crate) struct GpuVertex {
    pub pos: [f32; 4],    // 4D position
    pub color: [f32; 4],  // RGBA
}
```

The `GpuRenderer` is created once (during `App::new` via `CreationContext`) and stored
in the app. Each frame:
1. CPU computes which edges are visible (slice culling, near-plane culling)
2. GPU vertex buffer is updated with visible edge endpoints
3. Vertex shader does 4D→3D projection (camera transform) and 3D→2D (perspective)
4. Fragment shader does w-based coloring

### Phase B1: Glow backend (WebGL)

1. Get `glow::Context` from `CreationContext::gl()` on WASM
2. Compile GLSL shaders
3. Create VAO, VBO, EBO
4. Implement `egui_glow::CallbackFn` that:
   - Sets viewport/scissor from `PaintCallbackInfo`
   - Binds shader program + buffers
   - Issues draw call
   - Restores GL state

### Phase B2: Wgpu backend (native)

1. Get `wgpu::Device` + `wgpu::Queue` from `CreationContext::wgpu_render_state()`
2. Create WGSL shaders (same math as GLSL, different syntax)
3. Create render pipeline, vertex/index buffers
4. Implement `egui_wgpu::CallbackTrait` with `prepare()` (buffer upload) and `paint()` (draw)

### Phase B3: Cross-backend dispatch

In `App::new()`:
```rust
let gpu_renderer: Option<Box<dyn GpuRenderer>> = if let Some(gl) = cc.gl() {
    Some(Box::new(GlowRenderer::new(gl)))
} else if let Some(rs) = cc.wgpu_render_state() {
    Some(Box::new(WgpuRenderer::new(rs)))
} else {
    None
};
```

Store in the app. When rendering, check if GPU renderer is available:
- Yes → submit `PaintCallback` with the GPU renderer
- No (fallback) → use the batched Mesh approach from Approach A

### Phase B4: GPU-side vertex transformation

Move the 4D→3D projection into the vertex shader:

```glsl
// GLSL vertex shader
uniform mat4 mat_4d;       // CameraProjection::mat_4d
uniform vec4 offset_4d;    // CameraProjection::offset_4d
uniform mat3 mat_3d;       // CameraProjection::mat_3d
uniform float proj_dist;   // projection_distance
uniform vec2 viewport;     // viewport size for 2D projection
uniform float eye_offset;  // stereo eye separation

attribute vec4 pos_4d;     // 4D vertex position
attribute vec4 color_in;   // RGBA

void main() {
    vec4 r = mat_4d * pos_4d - offset_4d;
    vec3 xyz = mat_3d * r.xyz;
    // Perspective projection to 2D
    float z_offset = proj_dist + xyz.z;
    float scale = proj_dist / z_offset;
    vec2 screen = viewport * 0.5 + vec2(xyz.x * scale + eye_offset, -xyz.y * scale);
    gl_Position = vec4(screen, 0.0, 1.0);
}
```

This eliminates ALL per-vertex CPU work. The CPU only uploads the raw 4D vertex positions
and edge indices once (or updates when the polytope changes).

### Estimated Impact

- **Before A+B**: ~200 CPU-side shape submissions → tessellation → GPU per frame
- **After A+B**: ~2-3 PaintCallback submissions (left eye, right eye) + ~5 text shapes
- **Speedup**: ~50-100x for wireframe rendering. Frame time dominated by egui UI text.

### Risks and Considerations

1. **WebGL compatibility**: GLSL ES 3.0 required for uniform buffers. GLSL ES 1.0 (WebGL 1)
   may need workarounds. eframe uses WebGL 2 by default.
2. **State leakage**: Custom GPU rendering must not corrupt egui's GL state. The backend
   callbacks must save/restore all state.
3. **MSAA**: Custom rendering needs to match egui's anti-aliasing settings.
4. **Stereo**: Each eye needs its own PaintCallback with different `eye_offset` uniform.
   The vertex data is shared (same VBO), only the uniform changes.
5. **Fallback**: Approach A's batched Mesh must remain as fallback for any platform where
   GPU renderer initialization fails.

---

## Execution Order

1. **Phase A1-A5**: Batched Mesh (simpler, immediate benefit, no GPU code)
2. **Phase B1-B4**: PaintCallback GPU rendering (larger change, maximum speedup)

Each phase: implement → cargo fmt/clippy/test → just wasm → commit.
