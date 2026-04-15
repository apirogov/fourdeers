# 4D Stereo W-Panning Plan

## Concept
New parameter `w_eye_offset` (0.0-1.0, default 0.0) shifts each eye's W-slice window in opposite directions along W. Left-drag vertical controls it. At 0.0 both eyes see the same slice. At 1.0 eyes see mostly disjoint W-ranges with 10% overlap at center.

## Math
```rust
fn eye_w_params(w_half: f32, w_eye_offset: f32, eye_sign: f32) -> (f32, f32) {
    let shift = w_eye_offset * w_half * 0.45 * eye_sign;
    let sub_half = w_half * (1.0 - w_eye_offset * 0.45);
    (shift, sub_half)
}
```

Per-edge rendering (shift-and-truncate):
- Shift w: `w' = w - w_shift` before truncation
- Truncate with `sub_w_half` using existing `truncate_segment_to_slice`
- Alpha: `compute_vertex_alpha(w', sub_w_half)` — relative to sub-slice
- Color: `w_to_color(w / self.w_half, alpha)` — true W position

## Control
Left-view vertical drag (currently unused):
- Drag up → increase w_eye_offset
- Drag down → decrease w_eye_offset
- Sensitivity: W_EYE_OFFSET_DRAG_SENSITIVITY = 0.01

## Files Modified
1. src/render/style.rs — w_eye_offset field, constants, eye_w_params(), tests
2. src/render/tesseract.rs — render_edges + collect_edge_vertices accept w_shift, sub_w_half
3. src/render/mod.rs — re-exports
4. src/toys/scene_view.rs — manual stereo loop with per-eye w params, vertical left-drag
5. src/map/renderer.rs — MapRenderParams gets w_eye_offset, per-eye w in render loop
6. src/map/view.rs — vertical left-drag, pass w_eye_offset to params
7. src/toy/mod.rs — handle_drag signature adds w_eye_offset
8. src/toys/polytopes.rs — thread w_eye_offset through
9. src/app.rs — pass &mut w_eye_offset to handle_drag

## Tests (style.rs)
- test_sub_slice_center_alpha_constant_across_panning
- test_sub_slice_edge_alpha_is_min
- test_zero_panning_matches_current_behavior
- test_max_panning_left_eye_fades_positive_w
- test_panning_overlap_region_soft_fade
