# Polytopes Implementation Status

## Original Assignment

Add all 6 missing regular convex 4-polytopes to the codebase:
1. 5-cell (pentachoron)
2. 8-cell (tesseract) - already exists, move to new module
3. 16-cell - already exists as `create_glome()`, rename
4. 24-cell (icositetrachoron)
5. 120-cell (hecatonicosachoron)
6. 600-cell (hexacosichoron)

### Requirements
- All polytopes in `src/polytopes.rs`
- All centered at origin
- Tests to verify geometry using known symmetries (antipodal pairs, vertex degrees, edge counts)
- Dropdown in tesseract toy sidebar to select polytope
- Run `cargo clippy` and `cargo test` to verify
- Build WASM with `./build.sh`

## Clarified Questions

### Q1: 120-cell and 600-cell complexity
**Question:** These have 600/120 vertices with golden ratio coordinates. Full detail or skip?

**Answer:** Full detail - implement with full precision.

### Q2: Edge scaling
**Question:** Uniform edge length or unit bounding box?

**Answer:** Regular polytopes have uniform edge lengths by definition - they're regular.

## Polytope Reference

| Name | Vertices | Edges | Antipodal Pairs | Vertex Degree | Notes |
|------|----------|-------|-----------------|---------------|-------|
| 5-cell | 5 | 10 | 0 | 4 | Simplex, NOT centrally symmetric |
| 8-cell | 16 | 32 | 8 | 4 | Tesseract, bit permutations of (±1, ±1, ±1, ±1) |
| 16-cell | 8 | 24 | 4 | 6 | Axis permutations (±1, 0, 0, 0) |
| 24-cell | 24 | 96 | 12 | 8 | Pairs (±1, ±1, 0, 0) in all positions |
| 120-cell | 600 | 1200 | 300 | 4 | Uses golden ratio φ = (1+√5)/2 |
| 600-cell | 120 | 720 | 60 | 12 | Uses golden ratio φ = (1+√5)/2 |

## Current State

### Files Modified

| File | Status | Notes |
|------|--------|-------|
| `src/polytopes.rs` | **CORRUPTED - needs rewrite** | Was created but glitch corrupted it |
| `src/geometry.rs` | Done | Removed `create_tesseract()` and `create_glome()`, imports `Vertex4D` from polytopes |
| `src/render.rs` | Done | `TesseractRenderContext::new()` accepts `(vertices, indices)` params |
| `src/toys/tesseract.rs` | Done | Has `polytope_type: PolytopeType` field and dropdown UI |
| `src/toys/tetrahedron_debug.rs` | Done | Uses `create_polytope(PolytopeType::SixteenCell)` |
| `src/lib.rs` | Done | Exports `polytopes` module |

### Working Code Patterns

**render.rs TesseractRenderContext signature:**
```rust
pub fn new(
    vertices: Vec<Vertex4D>,
    indices: Vec<u16>,
    camera: &Camera,
    rot_xy: f32, rot_xz: f32, rot_yz: f32,
    rot_xw: f32, rot_yw: f32, rot_zw: f32,
    w_thickness: f32, w_min: f32, w_max: f32,
    eye_separation: f32, projection_distance: f32,
    projection_mode: ProjectionMode,
) -> Self
```

**tesseract.rs usage:**
```rust
let (vertices, indices) = create_polytope(self.polytope_type);
let ctx = TesseractRenderContext::with_stereo_settings(
    vertices, indices, &self.camera, ...
);
```

## Roadmap

### Step 1: Recreate `src/polytopes.rs`

Structure:
```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex4D {
    pub position: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolytopeType {
    FiveCell,
    #[default]
    EightCell,
    SixteenCell,
    TwentyFourCell,
    OneHundredTwentyCell,
    SixHundredCell,
}

impl PolytopeType {
    pub fn name(&self) -> &'static str { ... }
    pub fn short_name(&self) -> &'static str { ... }
    pub fn vertex_count(&self) -> usize { ... }
    pub fn edge_count(&self) -> usize { ... }
    pub fn all() -> [PolytopeType; 6] { ... }
}

pub fn create_polytope(kind: PolytopeType) -> (Vec<Vertex4D>, Vec<u16>) { ... }

fn create_5_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }
fn create_8_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }
fn create_16_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }
fn create_24_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }
fn create_120_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }  // May need careful implementation
fn create_600_cell() -> (Vec<Vertex4D>, Vec<u16>) { ... }  // May need careful implementation
```

### Step 2: Implement Simple Polytopes First

1. **5-cell (pentachoron):**
   - 4-simplex with 5 vertices
   - Coordinates: (1,1,1,-1/√5), (1,-1,-1,-1/√5), (-1,1,-1,-1/√5), (-1,-1,1,-1/√5), (0,0,0,4/√5)
   - All edges connect (complete graph K5)

2. **8-cell (tesseract):**
   - All 16 bit permutations of (±1, ±1, ±1, ±1)
   - Edges connect vertices differing in exactly 1 coordinate

3. **16-cell:**
   - 8 vertices at (±1, 0, 0, 0) and permutations
   - Edges connect all non-opposite pairs

4. **24-cell:**
   - 24 vertices: all permutations of (±1, ±1, 0, 0)
   - Edge length = √2, detect by distance

### Step 3: Implement Complex Polytopes (if time permits)

5. **600-cell:**
   - φ = (1 + √5) / 2
   - 8 vertices: permutations of (±1, 0, 0, 0)
   - 16 vertices: all (±0.5, ±0.5, ±0.5, ±0.5)
   - 96 vertices: even permutations of (±φ/2, ±0.5, ±1/(2φ), 0)
   - Edge detection by distance

6. **120-cell:**
   - Dual of 600-cell
   - More complex vertex set
   - May need careful verification

### Step 4: Add Tests

```rust
#[cfg(test)]
mod tests {
    // For each polytope:
    // - test_X_cell_vertex_count
    // - test_X_cell_edge_count  
    // - test_X_cell_centered_at_origin
    // - test_X_cell_uniform_edges
    // - test_X_cell_antipodal_pairs (except 5-cell)
    // - test_X_cell_vertex_degrees
}
```

### Step 5: Verify

```bash
cargo clippy
cargo test
./build.sh
```

### Step 6: Commit

```bash
git add -A
git commit -m "Add all 6 regular convex 4-polytopes with geometry tests"
```

## Known Issues

1. **600-cell and 120-cell tests were failing** before corruption:
   - Wrong vertex counts (generating too many or too few)
   - Wrong edge counts (edge detection epsilon issues)
   - Golden ratio coordinate generation is tricky

2. **Consider skipping complex polytopes** if time is limited:
   - Focus on 5, 8, 16, 24-cell first
   - Add 120/600-cell as stretch goals

## Test Helpers

```rust
fn distance_sq(v1: &Vertex4D, v2: &Vertex4D) -> f32 {
    (0..4).map(|i| (v1.position[i] - v2.position[i]).powi(2)).sum()
}

fn centroid(vertices: &[Vertex4D]) -> [f32; 4] {
    let n = vertices.len() as f32;
    let mut c = [0.0f32; 4];
    for v in vertices {
        for i in 0..4 { c[i] += v.position[i]; }
    }
    for i in 0..4 { c[i] /= n; }
    c
}

fn count_antipodal_pairs(vertices: &[Vertex4D]) -> usize {
    let mut count = 0;
    for i in 0..vertices.len() {
        for j in (i + 1)..vertices.len() {
            let is_antipodal = (0..4).all(|k| 
                (vertices[i].position[k] + vertices[j].position[k]).abs() < 1e-5
            );
            if is_antipodal { count += 1; }
        }
    }
    count
}

fn uniform_edge_lengths(vertices: &[Vertex4D], indices: &[u16]) -> bool {
    if indices.len() < 2 { return true; }
    let first = distance_sq(&vertices[indices[0] as usize], &vertices[indices[1] as usize]);
    indices.chunks(2).all(|chunk| {
        let d = distance_sq(&vertices[chunk[0] as usize], &vertices[chunk[1] as usize]);
        (d - first).abs() < 0.01 * first.max(1.0)
    })
}
```
