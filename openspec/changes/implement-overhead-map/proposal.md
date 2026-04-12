## Why

Marathon's levels are non-linear, multi-story labyrinths where players routinely get lost. The overhead map (automap) is essential for navigation — original Marathon shows it as a toggled 2D overlay of explored geometry with color-coded lines and entity blips. The current codebase has two problems: the web build (`marathon-web`) has a minimal Canvas 2D automap that draws every line in a single color with no fog of war, no entity markers, and no polygon type coloring; the desktop build (`marathon-game`) has no automap at all. Both need a proper overhead map that tracks exploration state and renders the map data the engine already fully parses.

## What Changes

- **Add explored-area tracking in `marathon-sim`**: Introduce a bitfield resource (`ExploredMap`) that marks polygons and lines as explored when the player enters or can see into adjacent polygons. The player's current polygon and its neighbors (via `adjacent_polygon_indexes`) get marked explored each tick. This state persists for the level's lifetime and is queryable by renderers.
- **Add a shared automap data model**: Define an `OverheadMapState` struct (in `marathon-sim` or a new shared module) that holds the explored bitfield, current zoom level, and pan offset. Expose query methods on `SimWorld` to retrieve explored lines with their type metadata (solid wall, elevation change, transparent/window, control panel side) and explored polygons with their type (platform, teleporter, media-containing, damage, etc.).
- **Implement line-type coloring**: Each line is colored based on its `LineFlags` and the polygon/side it borders — solid walls are one color, elevation changes another, transparent sides (windows) another, landscape lines another. Lines bordering polygons with media get colored by media type (water=blue, lava=orange, goo=green, sewage=brown). Platform polygons' lines are red. This matches Alephone's automap palette.
- **Implement entity blips**: Monsters are drawn as small rectangles at their world position (colored by hostility), items as diamonds or dots, and the player as a directional arrow. Entity positions come from `SimWorld::entities()` and `SimWorld::player_position()`/`player_facing()`.
- **Implement coordinate transform and zoom/pan**: World coordinates are projected to screen-space centered on the player (or pan offset). Zoom levels (at least 3) scale the pixels-per-world-unit factor. The map rotates so the player's facing is always "up" (north-up is an alternative mode).
- **Wire toggle keybinding**: Tab toggles the overlay on both desktop and web. The overlay composites on top of the 3D viewport as a translucent layer.
- **Desktop automap rendering**: Add automap drawing to `marathon-game` using either a wgpu 2D render pass (line primitives or a fullscreen quad with generated texture) or by rendering to an offscreen buffer and compositing.
- **Upgrade web automap rendering**: Replace the current single-color `draw_automap()` in `marathon-web/src/render.rs` with the full-featured version that uses the shared data model, fog of war, line coloring, and entity blips.

## Capabilities

### New Capabilities
- `overhead-map`: Explored-area tracking (bitfield per polygon/line), world-to-screen coordinate transform with zoom/pan, line-type color mapping (wall/elevation/transparent/landscape/media/platform), entity blips (player arrow, monster rectangles, item markers), fog of war (only explored geometry drawn), toggle keybinding, and rendering as a 2D overlay on both desktop and web builds

### Modified Capabilities
- `input-system`: Add Tab as the automap toggle key in both desktop (winit KeyCode) and web (JavaScript keydown) input handlers
- `level-rendering`: The frame loop must conditionally invoke the automap render pass after the 3D scene and HUD passes when the automap is visible
- `hud-rendering`: Automap overlay shares the 2D compositing approach — both render after the 3D pass and must not conflict in screen space or depth state

## Impact

- **marathon-sim/src/components.rs**: New `ExploredMap` resource (or component) holding `BitVec` for polygons and lines
- **marathon-sim/src/tick.rs**: Each tick, mark the player's polygon and visible neighbors as explored; new `explored_lines()` and `explored_polygons()` query methods on `SimWorld`
- **marathon-sim/src/world.rs**: `OverheadMapState` struct with zoom/pan state; initialization to all-unexplored on level load
- **marathon-game/src/render.rs**: New automap render pass or overlay drawing code; Tab key handling in the input event loop
- **marathon-web/src/render.rs**: Replace `draw_automap()` with the full implementation using explored-area data, line coloring, and entity blips; update `setup_input_handlers()` Tab handling to use new toggle
- **marathon-formats/src/map.rs**: No changes needed — `PolygonType`, `LineFlags`, `MediaData`, `Polygon.media_index`, and `Line` structs already provide all required data
- **Existing automap specs**: The `web-automap` spec from `fix-web-rendering` covers the basic toggle/render/player-marker; this change supersedes it with full fog-of-war, coloring, and entity blips
