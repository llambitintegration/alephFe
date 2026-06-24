## 1. Sim Data Model: ExploredMap and MapMetadata

- [x] 1.1 Add `MapMetadata` resource to `marathon-sim/src/world.rs` storing per-line flags (`LineFlags`), per-line side-has-control-panel (`bool`), per-polygon type (`i16`), per-polygon media_index (`i16`), per-line adjacent polygon indices (`(i16, i16)`), and media types (`Vec<i16>`). Build it alongside `MapGeometry` in `build_map_geometry()` (or a new `build_map_metadata()` function) from `MapData`.
- [x] 1.2 Add `ExploredMap` resource to `marathon-sim/src/world.rs` with `explored_polygons: Vec<bool>` and `explored_lines: Vec<bool>`, initialized to all `false`, sized to `map_data.polygons.len()` and `map_data.lines.len()` respectively. Insert it in `SimWorld::new()`.
- [x] 1.3 Add `OverheadMapState` resource to `marathon-sim/src/world.rs` with `visible: bool` (default false) and `zoom: f32` (default 12.0). Insert it in `SimWorld::new()`.
- [x] 1.4 Add unit test: construct `SimWorld` from test map data, verify `ExploredMap` has correct sizes and all entries are false.
- [x] 1.5 Add unit test: verify `OverheadMapState` initializes with visible=false, zoom=12.0.

## 2. Exploration Update System

- [x] 2.1 Implement `update_explored_map()` method on `SimWorld` that reads the player's `PolygonIndex`, marks that polygon as explored, marks all of its lines as explored (by iterating `polygon_adjacency` for the polygon and extracting line indices), then for each adjacent polygon reachable through a non-solid line, marks that polygon and its border lines as explored.
- [x] 2.2 Call `update_explored_map()` in `SimWorld::tick()` after `run_player_physics()`.
- [x] 2.3 Add unit test: place player in polygon 0 of a 3-polygon test map, tick once, verify polygon 0 is explored and its lines are explored. Verify adjacent polygon via non-solid line is explored. Verify polygon behind solid wall is NOT explored.
- [x] 2.4 Add unit test: move player from polygon 0 to polygon 1 (update PolygonIndex), tick, verify polygon 0 remains explored (persistence) and polygon 1 and its neighbors are now explored.

## 3. Line/Polygon Classification and Query API

- [x] 3.1 Define `LineCategory` enum: `SolidWall`, `ElevationChange`, `Transparent`, `Landscape`, `Platform`, `ControlPanel`. Define `ExploredLine` struct: `endpoints: (Vec2, Vec2)`, `category: LineCategory`, `explored: bool`. Define `ExploredPolygon` struct: `vertices: Vec<Vec2>`, `fill_color: [u8; 4]` (RGBA), `explored: bool`. Define `OverheadEntity` struct: `position: Vec2`, `facing: f32`, `kind: OverheadEntityKind` (Player, Monster, Item). Add these to `marathon-sim/src/tick.rs` or a new `marathon-sim/src/overhead.rs` module.
- [x] 3.2 Implement `classify_line(line_index, metadata) -> LineCategory` function that evaluates priority: control panel > platform > landscape > elevation > transparent > solid.
- [x] 3.3 Implement `polygon_fill_color(poly_index, metadata) -> [u8; 4]` function that checks media first, then polygon type, returning RGBA color bytes.
- [x] 3.4 Implement `explored_lines(&self) -> Vec<ExploredLine>` on `SimWorld` that iterates all lines, classifies each, reads explored status, and returns the list.
- [x] 3.5 Implement `explored_polygons(&self) -> Vec<ExploredPolygon>` on `SimWorld` that iterates all polygons, computes fill color, reads explored status, and returns the list.
- [x] 3.6 Implement `overhead_entities(&self) -> Vec<OverheadEntity>` on `SimWorld` that returns player position/facing and all monster/item positions with their kind.
- [x] 3.7 Implement `toggle_overhead_map(&mut self)` and `zoom_overhead_map(&mut self, delta: f32)` methods on `SimWorld` that modify `OverheadMapState`.
- [x] 3.8 Implement `overhead_map_visible(&self) -> bool` and `overhead_map_zoom(&self) -> f32` query methods on `SimWorld`.
- [x] 3.9 Add unit test: classify a line with IS_CONTROL_PANEL side as ControlPanel regardless of SOLID flag.
- [x] 3.10 Add unit test: classify a solid line adjacent to a Platform polygon as Platform.
- [x] 3.11 Add unit test: classify a line with ELEVATION flag (no higher priority) as ElevationChange.
- [x] 3.12 Add unit test: polygon_fill_color for a polygon with water media returns dark blue RGBA.
- [x] 3.13 Add unit test: polygon_fill_color for a Platform polygon with no media returns dark red RGBA.
- [x] 3.14 Add unit test: toggle_overhead_map toggles visible between false and true.

## 4. Web Automap Renderer Upgrade

- [ ] 4.1 Update `draw_automap()` in `marathon-web/src/render.rs` to accept explored polygons, explored lines, overhead entities, zoom level, and player position/facing instead of raw `map_lines`.
- [ ] 4.2 Implement polygon fill pass: iterate explored polygons, for each with `explored=true`, draw filled polygon path using Canvas 2D `beginPath/moveTo/lineTo/closePath/fill` with the polygon's fill color.
- [ ] 4.3 Implement line drawing pass: iterate explored lines, for each with `explored=true`, set stroke color based on `LineCategory`, draw line with `moveTo/lineTo/stroke`. Set line width to scale with zoom (1px at zoom<8, 1.5px at zoom 8-24, 2px at zoom>24).
- [ ] 4.4 Implement entity blip pass: if tick_count % 30 < 15, draw monster rectangles (red, 4px) and item dots (white, 3px) at transformed positions for entities in explored polygons.
- [ ] 4.5 Update player arrow drawing to use zoom-scaled size.
- [ ] 4.6 Wire zoom controls: in `setup_input_handlers()`, add +/= and - key handlers that call `sim.zoom_overhead_map(delta)` when the map is visible.
- [ ] 4.7 Update the render loop to call the sim's `explored_lines()`, `explored_polygons()`, and `overhead_entities()` query methods and pass results to `draw_automap()`.
- [ ] 4.8 Update the Tab toggle handler to call `sim.toggle_overhead_map()` and read `sim.overhead_map_visible()` for the canvas display style.

## 5. Desktop Automap Renderer

- [ ] 5.1 Create a flat-color wgpu shader (`automap.wgsl`) for the overhead map: vertex shader takes position (vec2) and color (vec4), outputs color to fragment; fragment shader outputs the interpolated color. No textures, no depth.
- [ ] 5.2 Create `OverheadMapRenderer` struct in `marathon-game/src/render.rs` (or a new `marathon-game/src/overhead.rs`) that holds the wgpu render pipeline, vertex buffer, and orthographic projection uniform.
- [ ] 5.3 Implement `OverheadMapRenderer::new(device, surface_format)` that creates the pipeline with alpha blending enabled, depth stencil disabled, and the flat-color shader.
- [ ] 5.4 Implement `OverheadMapRenderer::render()` that: builds vertex data from explored polygons (fan-triangulated) and lines (thin quads), writes to the vertex buffer, sets up orthographic projection centered on player, and submits a render pass to the provided `CommandEncoder` targeting the swapchain texture.
- [ ] 5.5 Implement entity blip rendering in the desktop overhead map: player arrow as a triangle, monster/item blips as small quads, using the same vertex buffer and flat-color shader.
- [ ] 5.6 Wire `OverheadMapRenderer` into the main frame loop in `marathon-game/src/render.rs` or `main.rs`: after 3D scene and HUD passes, if `overhead_map_visible()`, call `overhead_renderer.render()`.
- [ ] 5.7 Add Tab key handling in the desktop input loop to call `sim.toggle_overhead_map()`.
- [ ] 5.8 Add +/- key handling in the desktop input loop to call `sim.zoom_overhead_map(delta)` when the map is visible.

## 6. Testing

- [ ] 6.1 Run full cargo test suite in Docker and verify all existing + new tests pass.
- [ ] 6.2 Run e2e test suite and verify all existing tests pass (automap changes should not break existing behavior).
- [ ] 6.3 Add e2e test (web): open game, press Tab, verify automap canvas becomes visible (check `display` style).
- [ ] 6.4 Add e2e test (web): with automap visible, verify at least one colored line is drawn on the canvas (pixel sampling or canvas content check).
- [ ] 6.5 Deploy to marathon.llambit.io and verify: Tab toggles automap, explored areas show colored lines and polygon fills, entity blips appear, +/- zoom works.
- [ ] 6.6 Build and run marathon-game desktop binary, verify: Tab toggles automap overlay, explored geometry renders with correct colors, zoom controls work.
