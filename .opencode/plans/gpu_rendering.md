# GPU Rendering Optimization Plan

Status: **ACTIVE** — Phase A complete, Phase B1 in progress

## Context

The app renders 4D polytopes through egui's Painter API. Phase A (batched Mesh via
`LineBatch`) is already implemented and merged. This plan covers Phase B: moving rendering
to custom wgpu shaders via egui's `PaintCallback` API.

---

## Architecture Overview

### Single wgpu Backend

The project uses **wgpu on both native and web** (WebGPU primary, WebGL fallback via wgpu's
`webgl` feature). A single `egui_wgpu::Callback` handles both platforms. WGSL only, no GLSL.

### Pipeline Split: CPU Culling + GPU Projection

```
┌──────────────────────────────────────────────────────────────┐
│                         CPU SIDE                             │
│                                                              │
│  Per 4D Object:                                              │
│    1. CameraProjection::project(v4) → (xyz, w)              │
│       (4D world → camera-frame 3D + W)                       │
│    2. W-slice truncation (edge clipping, changes topology)   │
│    3. Near-plane culling (reject edges behind camera)        │
│    4. Per-vertex color/alpha from W (LUT + gradient)         │
│    5. Expand line segments → quads (4 verts, 6 indices each) │
│    6. Collect text label screen positions                    │
│                                                              │
│  Assemble GpuScene { vertices, indices, labels }             │
│                                                              │
├──────────────────────────────┬───────────────────────────────┤
│         prepare()            │         paint()               │
│  Upload vertex buffer        │  Set pipeline + bind group    │
│  Upload index buffer         │  Set viewport + scissor       │
│  Upload uniform buffer       │  draw_indexed                 │
│                              │                               │
├──────────────────────────────┴───────────────────────────────┤
│                      GPU VERTEX SHADER                       │
│                                                              │
│  Perspective project: xyz → screen_xy                        │
│    scale = proj_dist / (proj_dist + z)                       │
│    screen = viewport_center + [x*scale + eye_offset,         │
│                                 -y*scale]                    │
│  Convert to NDC                                              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                     GPU FRAGMENT SHADER                      │
│                                                              │
│  Output premultiplied alpha color                            │
│  (future: anti-alias via screen-space line distance)         │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                    CPU POST-PASS (egui Painter)              │
│                                                              │
│  Render text labels at projected screen positions            │
│  Render UI overlays (zone labels, backgrounds)              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Why This Split

| Stage | CPU or GPU | Rationale |
|-------|-----------|-----------|
| 4D→3D camera transform | CPU | Per-object CameraProjection, topology-aware |
| W-slice truncation | CPU | Edge clipping changes index count |
| Near-plane culling | CPU | Must reject entire edges |
| Color/alpha from W | CPU | LUT lookup, per-vertex rules |
| Line expansion → quads | CPU | wgpu has no thick line primitive |
| 3D→2D perspective | GPU | Pure parallel math per vertex |
| NDC conversion | GPU | Matches egui's coordinate system |
| Text rendering | CPU (egui) | Complex, not worth custom GPU |

### Multi-Object Support

Each view assembles a `Vec<GpuScene>` per frame. The GPU renderer merges all scenes
into a single vertex/index buffer upload and a single draw call per eye. Objects are
distinguished by vertex data, not separate draw calls.

```
SceneView::render_scene():
  scenes = [
    tesseract_wireframe_scene(camera, vertices, indices, settings),
    waypoint_scenes(waypoints, camera, settings),
    camera_marker_scene(camera, settings),
  ]
  gpu_renderer.render_stereo(left_painter, right_painter, scenes, stereo);
  // Then render text labels via egui Painter
  for label in scenes.flat_map(|s| &s.labels) { painter.text(...) }
```

### Extensibility for Surfaces

The architecture supports adding surface rendering:

1. **New primitive**: `GpuPrimitive::Triangles` alongside existing `Lines`
2. **Same vertex format**: `custom` fields reserved for normals/UVs
3. **Same pipeline**: Add a second render pipeline variant with depth + backface culling
4. **Depth buffer**: Enable via `depth_stencil_format` in wgpu pipeline
5. **Alpha blending**: Already premultiplied alpha
6. **Face data**: Would need extending `create_polytope()` to return face lists

---

## Module Structure

```
src/gpu/
├── mod.rs              — GpuRenderer, GpuScene, GpuLabel
├── vertex.rs           — GpuVertex struct, vertex buffer layout
├── pipeline.rs         — wgpu pipeline + bind group creation
├── callback.rs         — CallbackTrait impl (prepare + paint)
├── buffers.rs          — Vertex/index/uniform buffer management
└── shaders/
    └── wireframe.wgsl  — Vertex + fragment shader
```

---

## Key Types

### GpuVertex (vertex.rs)

```rust
/// 40 bytes per vertex. Uploaded to GPU.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuVertex {
    pub pos: [f32; 2],        // screen-space position (after CPU projection)
    pub color: [f32; 4],      // RGBA premultiplied alpha
    pub uv: [f32; 2],         // line-distance for AA (or WHITE_UV for surfaces)
}
```

This matches egui's own vertex layout (`pos + uv + color = 20 bytes`) to stay compatible
with the render target format. Screen-space positions are computed on CPU (the perspective
projection happens CPU-side for now, same as current code). The GPU shader converts to NDC.

**Future optimization**: Move perspective projection into the GPU vertex shader by adding
a 3D position variant:
```rust
// Future: GPU-side projection
pub(crate) struct GpuVertex3D {
    pub pos_3d: [f32; 3],    // camera-frame XYZ
    pub color: [f32; 4],
    pub w: f32,              // for depth fog / coloring
    pub line_width: f32,
    pub custom: [f32; 2],    // normals, UVs, etc.
}
```

### GpuScene (mod.rs)

```rust
/// CPU-assembled scene data for one PaintCallback submission.
pub(crate) struct GpuScene {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub labels: Vec<GpuLabel>,
}

pub(crate) struct GpuLabel {
    pub screen_pos: egui::Pos2,
    pub text: String,
    pub color: egui::Color32,
    pub font_id: egui::FontId,
    pub anchor: egui::Align2,
}
```

### SceneUniforms (buffers.rs)

```rust
/// Uniform buffer for screen-size conversion.
#[repr(C)]
struct SceneUniforms {
    screen_size: [f32; 2],  // logical screen size in points
}
```

Minimal uniforms — perspective projection is CPU-side. The shader only needs screen_size
to convert from points to NDC (same as egui's own shader).

### GpuRenderer (mod.rs)

```rust
/// Owns wgpu resources, created once from CreationContext.
pub(crate) struct GpuRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    // Buffers grown as needed, reused across frames
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertex_count: usize,
    index_count: usize,
}
```

Created in `FourDeersApp::new()` from `cc.wgpu_render_state()`. Stored in the app.
Passed to views for rendering.

---

## Wireframe Shader (WGSL)

```wgsl
// Uniforms
struct Uniforms {
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Vertex input (matches egui's vertex layout for render target compatibility)
struct VertexInput {
    @location(0) pos: vec2<f32>,         // screen-space position in points
    @location(1) uv: vec2<f32>,          // texture coords / line distance
    @location(2) color: u32,             // packed sRGBA premultiplied
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Convert screen-space points to NDC (same as egui's shader)
    out.clip_pos = vec4<f32>(
        2.0 * input.pos.x / uniforms.screen_size.x - 1.0,
        1.0 - 2.0 * input.pos.y / uniforms.screen_size.y,
        0.0, 1.0,
    );
    out.uv = input.uv;
    out.color = unpack4x8unorm(input.color);
    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
```

This is intentionally simple — same vertex layout as egui, same NDC conversion, just
bypassing egui's shape tessellation overhead. The CPU continues to do all the math
(quads for lines, perspective projection, colors) and submits ready-to-rasterize
triangles.

**Future upgrade path**: When we want GPU-side projection, change the vertex layout to
accept 3D positions and move the perspective math into the vertex shader. The pipeline
and callback structure stay the same.

---

## Integration with Views

### SceneView (primary target)

```rust
fn render_scene(&mut self, ui: &mut egui::Ui, rect: egui::Rect, gpu: Option<&GpuRenderer>) {
    let (left_rect, right_rect) = split_stereo_views(rect);
    let (left_proj, right_proj) = create_stereo_projectors(rect, &self.stereo);
    let camera_proj = CameraProjection::new(&self.camera);

    // CPU: assemble geometry
    let scenes = self.assemble_gpu_scenes(&camera_proj, &left_proj, &right_proj, &self.four_d);

    if let Some(gpu) = gpu {
        // GPU path: submit PaintCallbacks
        gpu.submit(left_painter, &scenes.left, &left_rect);
        gpu.submit(right_painter, &scenes.right, &right_rect);
    } else {
        // Fallback: LineBatch (existing CPU path)
        scenes.left.submit_batch(&left_painter);
        scenes.right.submit_batch(&right_painter);
    }

    // Text labels always via egui Painter
    for label in scenes.labels { painter.text(...) }
}
```

### MapView (secondary target)

Same pattern but with additional filled polygons (slice volume, visibility cone).
These can use the same PaintCallback — filled polygons are just triangles in the
same vertex/index buffer.

### CompassView (keep CPU for now)

Small geometry (1 tetrahedron), lots of text, orthographic projection. Not worth
migrating. Stays with LineBatch + egui Painter.

---

## Implementation Phases

### Phase B1: GPU renderer foundation ✦ current

- Create `src/gpu/` module with `mod.rs`, `vertex.rs`, `pipeline.rs`, `callback.rs`, `buffers.rs`
- `GpuRenderer::new(render_state)` — create pipeline, uniform buffer, bind group
- `GpuVertex` struct with bytemuck derive
- Wireframe shader in `src/gpu/shaders/wireframe.wgsl`
- `CallbackTrait` impl with prepare (buffer upload) and paint (draw_indexed)
- Initialize in `FourDeersApp::new()`, store in app

### Phase B2: Migrate scene view wireframes

- Add `assemble_gpu_scenes()` to SceneView
- CPU-side quad expansion (reuse LineBatch geometry logic)
- Submit via PaintCallback for each stereo eye
- Render text labels via egui Painter on top
- Keep LineBatch as fallback when no GPU renderer

### Phase B3: Migrate map view

- Migrate tesseract wireframe edges to GPU
- Migrate cross-section edges to GPU
- Migrate slice fill + visibility polygon to GPU (triangulated convex polygons)
- Migrate camera marker and waypoint geometry to GPU
- Text labels remain CPU

### Phase B4: Performance tuning

- Buffer reuse across frames (grow-only, never shrink)
- Object-level frustum culling
- Batch merging across objects (single draw call per eye)
- Profile and optimize vertex/index buffer sizes

### Phase B5 (Future): GPU-side projection

- New vertex format: `GpuVertex3D { pos_3d, color, w, line_width, custom }`
- Move perspective projection into vertex shader
- Keep CPU-side 4D→3D transform and W-slice clipping
- New uniform: `CameraUniforms { viewport_center, scale, proj_dist, eye_offset }`

### Phase B6 (Future): Surface rendering

- Extend `create_polytope()` to return face lists (triangles/quads)
- New `GpuPrimitive::Triangles` pipeline variant
- Enable depth buffer in wgpu pipeline
- Per-face normal computation
- Surface shader with basic lighting
- Transparent face rendering (depth sort or order-independent transparency)

---

## Risks and Considerations

1. **Fallback required**: LineBatch must remain as fallback for platforms where
   `wgpu_render_state()` is unavailable (shouldn't happen with our config, but defensive).

2. **Buffer management**: Vertex/index buffers must grow dynamically. Strategy: double
   the buffer size when exceeded, never shrink. Pre-allocate for 10K vertices.

3. **Stereo rendering**: Each eye gets its own PaintCallback with different viewport rect.
   Vertex data is the same (pre-projected to screen space per eye on CPU).

4. **wgsl include at compile time**: Shader source is included via `include_str!()` at
   compile time. No runtime shader compilation needed.

5. **Blend state**: Must match egui's premultiplied alpha blending:
   `color: One + OneMinusSrcAlpha, alpha: OneMinusDstAlpha + One`

6. **Render target format**: Use `RenderState::target_format` (typically `Rgba8Unorm` or
   `Bgra8Unorm`). Same format egui uses.

---

## Execution

Each phase: implement → cargo fmt/clippy/test → just wasm → commit.
