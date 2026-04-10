# View Architecture Refactor Plan

## Goal

Refactor the app/toy/view architecture so that toys own their views, views are self-contained
reusable components, and app.rs becomes a thin dispatch shell with zero view-specific branches.

## Core Design Decisions

- **Toy dispatches internally** — no `View` trait. The toy has scene/map/compass as struct fields
  and dispatches via `active_view_id`.
- **Views return `ViewAction`** — `handle_tap()` returns `ViewAction::SwitchView("map")` etc. so
  views can trigger navigation without knowing about the toy.
- **Views take data as params** — `MapView::render(camera, waypoints, bounds)`, no generic data
  source trait.
- **Each view renders its own nav labels** — the active view draws "Map", "Compass", "Close" etc.
  in `render_overlays()`.
- **Overlays are owned by views** — direction tetrahedra toggle, labels toggle, etc. are internal
  to the view that renders them.

## Phases

### Phase 1: Define ViewAction + extend Toy trait

- Add `ViewAction` enum to `src/toy/mod.rs`
- Add to `Toy` trait with default no-op implementations:
  - `render_view_overlays(&self, left_painter, left_rect, right_painter, right_rect)`
  - `set_active_view(&mut self, id: &str)`
  - `active_view_id(&self) -> &str`
  - `available_views(&self) -> Vec<(&str, &str)>`
- Change `handle_tap` return type from `()` to `ViewAction`
- Update `app.rs` to handle `ViewAction` from `handle_tap`
- Remove `toggle_directions()` and `directions_visible()` from trait
- Backwards compatible: default implementations keep current behavior

### Phase 2: Extract SceneView from PolytopesToy

- Create `src/toys/scene_view.rs` with `SceneView` struct containing:
  - `show_directions: bool`
  - `right_view_4d_rotation: bool`
  - `zone_mode: ZoneMode`
  - `visualization_rect: Option<egui::Rect>`
  - `drag_state: DragState`
  - `tetrahedron_rotations: HashMap<TetraId, UnitQuaternion<f32>>`
- `SceneView` methods: `render()`, `render_overlays()`, `handle_tap()`, `handle_drag()`,
  `handle_hold()`, `handle_drag_start()`, `clear_interaction_state()`, `handle_keyboard()`
- "Dir:On/Off" toggle is entirely internal to SceneView
- PolytopesToy delegates to SceneView for scene rendering/input

### Phase 3: Extract MapView from app.rs

- Create `src/map/view.rs` with `MapView` struct containing:
  - `renderer: MapRenderer`
  - `frame_mode: CompassFrameMode`
  - `rotation_3d: bool`
- Move `MapState` fields, `handle_map_tap`, map overlay rendering into `MapView`
- `MapView::render()` takes `camera: &Camera, waypoints: &[CompassWaypoint], bounds: Option<Bounds4D>`
- `MapView::handle_tap()` returns `ViewAction`
- Remove map-specific code from `app.rs`

### Phase 4: Extract CompassView from app.rs

- Create `src/view/compass_view.rs` with `CompassView` struct containing:
  - `rotation: UnitQuaternion<f32>`
  - `waypoint_index: usize`
  - `frame_mode: CompassFrameMode`
- Move `CompassState` + compass rendering + compass tap handling into `CompassView`
- `CompassView::render()` takes vector/waypoints/reference/camera_frame_fn as params
- Remove compass-specific code from `app.rs`

### Phase 5: Toy dispatches internally

- `PolytopesToy::render_active_view()` matches on `active_view_id` and dispatches
- `PolytopesToy::handle_tap()` does the same
- Remove `ActiveView` enum from `app.rs`
- Gut `app.rs` routing to thin dispatch

### Phase 6: Cleanup

- Remove dead code from `app.rs` (target: ~200 lines)
- Remove old trait methods no longer needed
- Full test/lint/build cycle

## Guardrails

- Each phase must compile, pass 199 tests, and produce a working WASM build before committing.
- No phase should break the UI — the app should look and behave identically before and after.
- Commit after each phase.
