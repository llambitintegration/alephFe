## Context

The overhead map (automap) is one of Marathon's core navigation tools. Original Alephone renders a 2D overlay showing explored geometry with color-coded lines (solid/elevation/transparent/landscape), filled polygons colored by type (platform, teleporter, media), entity blips (monsters, items, player arrow), and fog of war that only reveals visited areas.

The current Rust codebase has two problems:
1. **marathon-web** has a minimal Canvas 2D automap (`draw_automap()` in `render.rs`) that draws every line in a single color (`#4a9`) with no fog of war, no polygon fill, no entity blips, no line-type differentiation, and no zoom controls.
2. **marathon-game** (desktop) has no automap at all.

The simulation layer (`marathon-sim`) has no exploration tracking -- all map geometry is available but there is no per-polygon/per-line visited bitfield. The `MapGeometry` resource stores adjacency, floor/ceiling heights, line endpoints, and solid/transparent flags, but does not classify lines by visual type or track which areas the player has seen.

The `marathon-formats` crate already provides all the raw data needed: `PolygonType` (24 variants including Platform, Teleporter, MinorOuch, MajorOuch), `LineFlags` (SOLID, TRANSPARENT, LANDSCAPE, ELEVATION, VARIABLE_ELEVATION, HAS_TRANSPARENT_SIDE, DECORATIVE), `SideFlags` (IS_CONTROL_PANEL), `MediaData` with `media_type` (Water, Lava, Goo, Sewage, Jjaro), and `Polygon.media_index` linking polygons to their liquid fill.

## Goals / Non-Goals

**Goals:**
- Track explored polygons and lines in the simulation as the player moves through the level
- Render color-coded lines differentiated by type (solid wall, elevation change, transparent/window, landscape, control panel)
- Render filled polygons colored by type (platform=red, teleporter=cyan, media by liquid type)
- Render entity blips: player as a directional arrow, monsters as red rectangles, items as white dots
- Implement fog of war so only explored geometry is drawn
- Support zoom in/out (at least 3 zoom levels) and keep the map centered on the player
- Toggle the overlay with Tab on both desktop and web
- Desktop: wgpu-based 2D overlay render pass composited on the 3D scene
- Web: upgrade the existing Canvas 2D automap with the full feature set

**Non-Goals:**
- MML color override support (deferred -- the infrastructure in marathon-formats recognizes the section but we do not need runtime customization yet)
- Pan offset or north-up rotation mode (player-centered, player-up-facing is sufficient for now)
- Save/load of exploration state (resets on level load, same as original Marathon behavior within a session)
- Checkpoint map or saved-game-preview rendering modes (only the live in-game mode)
- Annotations / text labels on the map

## Decisions

### 1. Explored-area tracking as a bevy_ecs Resource in marathon-sim

**Decision**: Add an `ExploredMap` resource to `marathon-sim` containing two `Vec<bool>` bitfields, one indexed by polygon index and one by line index. Each tick after player physics runs, a BFS from the player's current polygon marks the polygon and all of its lines as explored, then extends one hop to each adjacent polygon (via non-solid lines) and marks those polygons and their border lines. This gives a simple "current room + visible neighbors" exploration radius that accumulates over the level's lifetime.

**Rationale**: Using a bevy_ecs `Resource` keeps explored state collocated with the sim world and accessible via `SimWorld` query methods. A `Vec<bool>` is simple and sufficient -- Marathon levels have at most ~2000 polygons and ~4000 lines, so memory is trivial. The BFS approach matches Alephone's exploration behavior where entering a polygon reveals it and adjacent visible areas. The adjacency data is already in `MapGeometry.polygon_adjacency`.

**Alternative considered**: Using `BitVec` from the `bitvec` crate for compactness. Rejected because it adds a dependency for negligible savings at these sizes. Can be swapped in later if needed.

### 2. Shared OverheadMapState struct with platform-agnostic query API

**Decision**: Add an `OverheadMapState` resource in `marathon-sim` holding the `ExploredMap`, current zoom level (f32), and a `visible: bool` toggle. Expose query methods on `SimWorld`:
- `overhead_map_state() -> &OverheadMapState` -- zoom, visibility
- `explored_lines() -> Vec<ExploredLine>` -- line endpoints, line type classification, explored status
- `explored_polygons() -> Vec<ExploredPolygon>` -- polygon vertices, polygon type classification, explored status
- `overhead_entities() -> Vec<OverheadEntity>` -- position, facing, entity kind (player/monster/item)
- `toggle_overhead_map()`, `zoom_overhead_map(delta: f32)`

Renderers (both desktop wgpu and web Canvas 2D) call these query methods and handle their own drawing. The sim owns the data model; renderers own the drawing.

**Rationale**: This keeps the sim as the single source of truth for exploration and map state. Both renderers consume the same API, preventing divergence. The sim does not need to know about Canvas 2D or wgpu. Classification of lines/polygons into visual types happens once in the sim query (reading `MapData` line flags, polygon types, side flags, media indices) rather than being duplicated in each renderer.

**Alternative considered**: Putting the OverheadMapState in each renderer crate. Rejected because exploration tracking must live in the sim (it depends on player polygon, which only the sim knows), and splitting state between sim and renderer would create synchronization issues.

### 3. Line-type classification scheme

**Decision**: Classify each line into one of six visual categories based on `LineFlags`, adjacent polygon types, and `SideFlags`:

| Priority | Condition | Category | Color (Alephone-matching) |
|----------|-----------|----------|--------------------------|
| 1 | Side has `IS_CONTROL_PANEL` | ControlPanel | Red `#c00` |
| 2 | Adjacent polygon is Platform type | Platform | Red `#800` |
| 3 | `LANDSCAPE` flag set | Landscape | Brown `#960` |
| 4 | `ELEVATION` or `VARIABLE_ELEVATION` flag set | ElevationChange | Light green `#090` |
| 5 | `HAS_TRANSPARENT_SIDE` flag set | Transparent | Dim green `#060` |
| 6 | `SOLID` flag set (or default) | SolidWall | Bright green `#0c0` |

Priority ordering means a line that is both solid and a control panel renders as a control panel. Lines not in the automap (unexplored) are not drawn at all.

**Rationale**: This matches Alephone's `OverheadMapRenderer` which checks control panels first, then platform membership, then line flags. The six categories cover all visually distinct line types in the original game. Colors are chosen to match the original Alephone automap palette.

### 4. Polygon fill coloring

**Decision**: Fill explored polygons before drawing lines. Color priority:
1. If polygon has `media_index >= 0` and the media height is above the floor, color by media type (Water=#002244, Lava=#440800, Goo=#084408, Sewage=#2a3008, Jjaro=#200840)
2. Else if polygon type is Platform or SecretPlatform: #300000 (dark red)
3. Else if polygon type is Teleporter: #003030 (dark cyan)
4. Else if polygon type is MinorOuch: #302000 (dark yellow)
5. Else if polygon type is MajorOuch: #300000 (dark red)
6. Default: #001800 (dark green)

**Rationale**: Media takes priority because Alephone renders the liquid color when media is present regardless of polygon type. The dark tones ensure lines remain visible on top. These colors match the vault research from Alephone's color tables.

### 5. Entity blips

**Decision**: After polygon fill and line drawing, render entity blips:
- **Player**: A directional arrow (triangle pointing in facing direction), yellow `#ff4`, size 8px at default zoom. Drawn at map center (map is always centered on player).
- **Monsters**: Small filled rectangles, red `#c44`, size 4px. Only drawn if their polygon is explored.
- **Items**: Small filled circles/dots, white `#ddd`, size 3px. Only drawn if their polygon is explored.

Blips blink at 0.5 Hz (visible 50% of the time) based on tick count modulo, matching Alephone's blinking behavior.

**Rationale**: Keeping blip shapes simple (rect/circle/arrow) works well in both Canvas 2D and wgpu line rendering. Restricting entity blips to explored polygons maintains fog of war consistency. Blinking makes blips distinguishable from static map geometry.

### 6. Coordinate transform

**Decision**: World coordinates transform to screen space as:
```
screen_x = viewport_center_x + (world_x - player_x) * zoom_scale
screen_y = viewport_center_y + (world_y - player_y) * zoom_scale
```

The map is always centered on the player's current 2D position (x, y in Marathon coordinates, which are the horizontal plane). No rotation is applied -- "east" in-world is right on screen. The player arrow rotates to show facing direction.

Zoom scale levels: minimum 4.0, default 12.0, maximum 48.0 pixels per world unit. Zoom in/out with +/- keys (or mousewheel on desktop).

**Rationale**: Player-centered rendering is simpler than player-facing-up rotation and matches the most common automap UX. The zoom range covers from zoomed-out full-level views to detailed room-level views. The coordinate transform is trivial (linear) and identical between web and desktop.

**Alternative considered**: Rotating the entire map so the player's facing is always "up." Deferred as an optional mode -- it adds complexity (rotating all line/polygon coordinates each frame) for debatable UX benefit.

### 7. Desktop renderer: wgpu 2D overlay pass

**Decision**: On desktop (`marathon-game`), render the automap as a separate wgpu render pass after the 3D scene and HUD passes. Use an orthographic projection matrix for 2D drawing. Render polygons as flat colored triangles (fan-triangulate each polygon) and lines as thin quads (2-triangle strips with configurable thickness). The render target is the same swapchain surface, composited with alpha blending (map background at ~70% opacity). A dedicated simple shader (flat vertex color, no textures) handles both polygon fills and lines.

**Rationale**: A wgpu render pass integrates naturally into the existing frame loop, shares the same device/queue, and avoids needing a separate 2D drawing library. Fan triangulation works because Marathon polygons are always convex. Lines as thin quads give consistent thickness at all zoom levels. The shader is trivially simple compared to the 3D pipeline.

**Alternative considered**: Rendering to an offscreen texture then compositing as a fullscreen quad. Rejected for the first pass -- direct rendering to the swapchain is simpler and avoids the extra texture allocation. Can be refactored later if needed for post-processing effects.

### 8. Web renderer: enhanced Canvas 2D

**Decision**: Keep the existing Canvas 2D approach for `marathon-web` but replace the `draw_automap()` function body with the full-featured version. The Canvas 2D API is well-suited for 2D line/polygon rendering and avoids adding complexity to the WebGL pipeline.

Changes to `draw_automap()`:
1. Accept `OverheadMapState` data (explored lines, explored polygons, entities) instead of raw `map_lines`
2. First pass: fill explored polygons with type-based colors
3. Second pass: draw explored lines with type-based colors and line widths
4. Third pass: draw entity blips (monsters, items)
5. Final: draw player arrow at center

**Rationale**: Canvas 2D is already working for the basic automap. The API directly supports `beginPath/fill/stroke` with per-element color changes, which is exactly what we need. No WebGL shader work needed. The Canvas overlay is independent of the 3D viewport resolution.

### 9. Exploration update integrated into tick()

**Decision**: After `run_player_physics()` in `SimWorld::tick()`, call a new `update_explored_map()` method. This method reads the player's `PolygonIndex`, marks that polygon and all its lines as explored, then for each adjacent polygon reachable through a non-solid line, marks that polygon and its border lines as explored too.

**Rationale**: Running after player physics ensures the exploration reflects the player's post-movement polygon. The one-hop BFS is cheap (each polygon has at most 8 neighbors) and runs once per tick (30 Hz). Over time this accumulates a complete picture of everywhere the player has been and the rooms they have looked into.

### 10. MapData reference stored in sim for classification queries

**Decision**: Store a subset of `MapData` (lines with flags, polygons with types and media indices, sides with flags, media with types) in a new `MapMetadata` resource so the sim's overhead map query methods can classify lines and polygons without needing the full `MapData` at query time.

**Rationale**: The `MapGeometry` resource already extracts geometric data but strips the type/flag metadata needed for automap coloring. Rather than bloating `MapGeometry`, a separate `MapMetadata` resource holds the classification data. This resource is built at level load alongside `MapGeometry` and is read-only.

## Risks / Trade-offs

- **[Risk] Desktop wgpu overlay may conflict with existing depth/blend state** - The automap render pass must disable depth testing and use alpha blending. If the existing frame loop does not cleanly separate passes, the automap could write to the depth buffer and corrupt subsequent rendering. Mitigate by explicitly setting `depth_stencil: None` and blend state on the automap pipeline.
- **[Risk] BFS exploration may over-reveal in open areas** - One-hop BFS from the player polygon reveals all visible neighbors, which in large open areas could reveal many polygons at once. This matches Alephone's behavior and is acceptable. If too aggressive, the hop count can be reduced to zero (only current polygon).
- **[Trade-off] No map rotation mode** - Players who prefer "facing = up" orientation will not have that option initially. The data model supports adding this later by rotating all coordinates before drawing.
- **[Trade-off] Vec<bool> instead of BitVec** - Uses 8x more memory than a bitfield. At ~6000 entries (polygons + lines), this is ~6 KB vs ~750 bytes. Negligible either way.
- **[Trade-off] No saved exploration state** - Exploration resets on level load. This matches Marathon's original behavior for in-level play. Save/load of explored state would require adding it to `SimSnapshot`, deferred to a future change.
