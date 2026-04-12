---
tags: [tier-2, rendering, automap, overhead-map, hud, ui]
status: research-complete
---

# Overhead Map / Automap

How Marathon/Alephone renders the automap overlay, and what the Rust rebuild needs for a complete implementation.

## Original Alephone Implementation

### Architecture

The overhead map system in Alephone is split across several files:
- `overhead_map.h` / `overhead_map.cpp` - Configuration, color definitions, dispatch
- `OverheadMapRenderer.cpp` - Core rendering logic (polygon fill, line drawing, entity blips)
- `OverheadMap_SDL_Class` - Software (SDL) rendering backend
- `OverheadMap_OGL_Class` - OpenGL rendering backend

The main entry point `_render_overhead_map()` selects between SDL and OGL renderers based on the `OGL_MapActive` flag.

### Rendering Modes

Three rendering modes (from `overhead_map.h`):
- `_rendering_saved_game_preview` - Static preview for save game screen
- `_rendering_checkpoint_map` - Map shown at level start (reveals explored areas)
- `_rendering_game_map` - Live in-game automap (toggled with M key)

### Data Structure

```c
struct overhead_map_data {
    short mode;              // Rendering mode
    short scale;             // Zoom level (1-4, default 3)
    world_point2d origin;    // Center of view (usually player position)
    short origin_polygon_index;
    short half_width, half_height;
    short width, height;
    short top, left;
    bool draw_everything;    // Ignore fog of war
};
```

Scale constants:
- `OVERHEAD_MAP_MINIMUM_SCALE` = 1 (most zoomed out)
- `OVERHEAD_MAP_MAXIMUM_SCALE` = 4 (most zoomed in)
- `DEFAULT_OVERHEAD_MAP_SCALE` = 3

### Coordinate Transform

World coordinates are transformed to screen space:
```c
#define WORLD_TO_SCREEN(x, x0, scale) \
    (((x) - (x0)) >> (WORLD_TO_SCREEN_SCALE_ONE - (scale)))
```

This centers the view on `(x0, y0)` (player position) and scales by right-shifting. Higher scale = more zoomed in.

### Polygon Rendering

The renderer iterates through all polygons (`dynamic_world->polygon_count`). For each polygon marked as explored (`_polygon_on_automap` flag), it determines a fill color based on type:

| Polygon Type | Color (approximate RGB) |
|-------------|------------------------|
| Plain/Normal | Dark green `(0, 12000, 0)` |
| Platform | Red `(30000, 0, 0)` |
| Secret Platform | Red (same as platform) |
| Water | Blue |
| Lava | Orange-red |
| Goo | Bright green |
| Sewage | Yellow-green |
| Jjaro | Purple |
| Minor Ouch | Yellow |
| Major Ouch | Red |
| Teleporter | Cyan |
| Hill | Special color |

Media-filled polygons use colors based on the media type rather than the polygon type, but only when the media height is above the floor.

Each polygon is drawn as a filled shape using `draw_polygon(vertex_count, endpoint_indexes, color, scale)`.

### Line Rendering

Lines (walls) are drawn after polygons. Three line categories with distinct colors and widths:

| Line Type | Color | Width (per scale) |
|-----------|-------|-------------------|
| Solid wall | Green | 4 width values for zoom levels |
| Elevation change | Lighter green | Thinner than solid |
| Control panel | Red | Same as solid |

Lines are only drawn if:
- The line is in the automap bitfield (`LINE_IS_IN_AUTOMAP(i)`)
- The line is solid, has variable elevation, or is landscape-flagged
- Line drawing respects elevation differences between adjacent polygons

### Entity Blips (Things)

Objects on the map are rendered as small shapes:

| Entity Type | Shape | Color |
|------------|-------|-------|
| Player | Arrow (team-colored) | Team color |
| Civilian/VacBob | Rectangle | Blue |
| Monster | Rectangle | Red |
| Item | Rectangle | White |
| Projectile | Rectangle | Yellow |
| Checkpoint | Circle | Red |

Entity rendering uses `draw_thing(location, facing, thing_type, scale)`. Blips **blink** based on tick count for visibility.

Players are special: `draw_player(location, facing, team, scale)` renders an arrow pointing in their facing direction with team coloring.

### Fog of War

Exploration tracking uses two bitfield arrays:
- `automap_lines[]` - one bit per line, set when the player has seen it
- `automap_polygons[]` - one bit per polygon, set when the player has visited/seen it

Macros for checking:
- `LINE_IS_IN_AUTOMAP(i)` - test if line `i` has been explored
- `POLYGON_IS_IN_AUTOMAP(i)` - test if polygon `i` has been explored
- `SET_LINE_IN_AUTOMAP(i)` / `SET_POLYGON_IN_AUTOMAP(i)` - mark as explored

The checkpoint map mode uses `flood_map()` to expand visible areas from the entry polygon, respecting secret platform boundaries.

`ResetOverheadMap()` clears both bitfield arrays with `memset`.

Two visibility modes:
- `OverheadMap_CurrentlyVisible` - clears previous data, shows only currently visible
- `OverheadMap_All` - reveals everything (cheat/debug mode)

### MML Customization

Colors, line widths, entity shapes, and display parameters can be overridden via MML (Marathon Markup Language) XML configuration through `parse_mml_overhead_map()`.

## Current State in Rust Rebuild

### What Exists

**marathon-formats** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/tags.rs`):
- `WadTag::AutomapLines` and `WadTag::AutomapPolygons` tag identifiers are defined
- Tags are parsed correctly but data is not extracted or used

**marathon-formats/mml.rs** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/mml.rs`):
- `overhead_map` MML section is recognized and parsed

**marathon-web** (`/home/llambit/0_repos/alephone-rust/marathon-web/src/render.rs` lines 267-379):
- **Basic automap exists** in the web crate using a 2D Canvas overlay
- Toggle with a key, renders as a `<canvas>` element over the WebGL canvas
- `draw_automap()` function renders all map lines in a single color (`#4a9`)
- Player arrow rendered as a yellow triangle at center
- Simple coordinate transform: `cx + (point - player) * scale`
- `map_lines` extracted from level data as `Vec<([f32; 2], [f32; 2])>`

**Level loading** (`marathon-web/src/render.rs` lines 581-587):
- Map lines extracted from endpoints during level load
- All lines included (no fog of war filtering)

### Gaps

1. **No fog of war** - all lines shown regardless of exploration state
2. **No polygon filling** - only wireframe lines, no colored polygon fill
3. **No entity blips** - no monsters, items, or other player markers
4. **No line type differentiation** - all lines same color, no distinction between solid/elevation/control panel
5. **No zoom controls** - fixed scale, no zoom in/out
6. **No automap data from WAD** - `AutomapLines`/`AutomapPolygons` tags not loaded or used
7. **Only in web crate** - marathon-viewer and marathon-game have no automap
8. **No exploration tracking** - sim does not track which polygons/lines have been visited
9. **No media-colored polygons** - water/lava areas not distinguished
10. **No MML color overrides** - custom map colors not supported

## Implementation Recommendations

### Phase 1: Enhanced Line Rendering (Web)

The existing web automap is a good foundation. Enhance it:

1. **Line type colors**: Classify lines by type and color them:
   ```rust
   fn line_color(line: &LineData, map: &MapData) -> &str {
       if line.flags.contains(LineFlags::SOLID) { "#0c0" }       // solid green
       else if line.flags.contains(LineFlags::ELEVATION) { "#090" } // dim green
       else { "#060" }                                              // subtle
   }
   ```

2. **Polygon fill**: Before drawing lines, fill explored polygons using Canvas 2D `fill()`:
   ```rust
   fn polygon_color(poly: &PolygonData, map: &MapData) -> &str {
       if poly.media_index >= 0 {
           match map.media[poly.media_index as usize].media_type {
               0 => "#002244", // water
               1 => "#440800", // lava
               2 => "#084408", // goo
               3 => "#2a3008", // sewage
               4 => "#200840", // jjaro
               _ => "#001800",
           }
       } else {
           match poly.polygon_type() {
               PolygonType::Platform => "#300000",
               PolygonType::Teleporter => "#003030",
               _ => "#001800",
           }
       }
   }
   ```

3. **Zoom controls**: Add keyboard bindings for zoom in/out (e.g., +/- keys), clamping scale between 4.0 and 48.0 pixels per world unit.

### Phase 2: Entity Blips and Fog of War

4. **Entity rendering**: Query the sim for entity positions and draw:
   - Monsters as small red rectangles
   - Items as white dots
   - Player as a larger yellow/green arrow

5. **Exploration tracking**: Add bitfield arrays to the sim state:
   ```rust
   pub struct AutomapState {
       pub explored_lines: Vec<bool>,    // indexed by line index
       pub explored_polygons: Vec<bool>, // indexed by polygon index
   }
   ```
   Mark polygons/lines as explored when the player enters a polygon (BFS from player polygon through visible lines).

6. **Fog of war rendering**: Only render explored lines/polygons. Unexplored areas are dark.

### Phase 3: Native Renderer (wgpu)

For marathon-viewer and marathon-game, implement the automap as a wgpu render pass:

7. **2D orthographic projection**: Create a separate render pass with an orthographic camera centered on the player.

8. **Polygon mesh**: Generate a flat 2D mesh of polygon outlines and fills.

9. **Line rendering**: Use thick line rendering (triangle strips or geometry instancing).

10. **Overlay composition**: Render the automap to a texture, then composite it over the 3D view with alpha blending.

### WebGL2 Considerations

The current Canvas 2D approach in marathon-web is actually ideal for the automap since:
- 2D Canvas is well-suited for line/polygon rendering
- No additional wgpu draw calls needed
- Independent resolution from the 3D viewport
- Easy to add text labels for annotations

Keep the Canvas 2D approach for web, but add the wgpu approach for native.

## Related Notes

- [[liquid-surface-rendering]] - Media types affect polygon colors on the automap
- [[dynamic-lighting]] - Light states could optionally affect automap brightness
- [[infravision-mode]] - Infravision could reveal more on the automap
