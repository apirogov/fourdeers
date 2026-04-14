# Multi-Polytope Scene Plan

## Overview
Replace single-polytope mode with a scene containing all 6 regular convex 4-polytopes arranged in 4D space. Each is a compass waypoint. Camera can jump to any polytope.

## Data Model (polytopes.rs only)

New `ScenePolytope`:
- `polytope_type: PolytopeType`
- `position: Vector4<f32>` — world-space offset
- `world_vertices: Vec<Vector4<f32>>` — pre-offset vertices
- `indices: Vec<u16>` — edge indices (local to this polytope)

`PolytopesToy` changes:
- Replace `polytope_type`, `cached_vertices`, `cached_indices` with:
  - `polytopes: Vec<ScenePolytope>`
  - `merged_vertices: Vec<Vector4<f32>>` — all polytopes concatenated
  - `merged_indices: Vec<u16>` — indices remapped into merged vertex buffer
- Remove `ensure_polytope_cached()`

## Polytope Positions (~10-15 units apart)

| Polytope      | Position           | Notes |
|---            |---                 |---    |
| 5-cell        | (-8, 4, 6, 3)      | upper-left-forward-kata |
| 8-cell        | (2, 0, 0, 0)       | visible from camera start, in default w-slice |
| 16-cell       | (-4, -8, 8, -4)    | lower-forward-ana |
| 24-cell       | (10, 5, -4, 5)     | far right, deep kata |
| 600-cell      | (-6, -6, 12, -2)   | far forward, slightly ana |
| 120-cell      | (6, 10, 4, -7)     | upper-right, deep ana |

Camera starts at (0, 0, -5, 0) — tesseract ~5.4 units ahead, in w-slice.

## Waypoints
- `compass_waypoints()` returns one per polytope using `short_name()` as title
- Remove "Origin" and "TestPoint" waypoints

## Jump to Waypoint
- **J key**: works in any active view
- **Tap zone**: Compass view, right half, Zone::North — labeled "Jump"
- On jump:
  1. Get current compass waypoint
  2. Set `camera.position = waypoint.position + Vector4::new(0, 0, -5, 0)`
  3. Reset camera rotation to identity (look straight ahead)
  4. Switch to Scene view

## Sidebar Changes
- Remove: polytope type selector dropdown, single polytope stats
- Add: summary "6 polytopes • {total_v} vertices • {total_e} edges"
- Update Controls section with complete key list

## Keyboard Controls (complete list for sidebar)
Movement (hold): Arrows=Up/Down/Left/Right, PgUp/PgDn=Forward/Back, ,/.=Ana/Kata
Views: C=Compass, G=Map, M=Menu, U=Debug info
Waypoint: ArrowLeft/Right=cycle, J=jump, F=toggle frame mode

## Files Modified
- `src/toys/polytopes.rs` — all changes
- `src/view/compass_view.rs` — add Jump tap zone (right North) + label
