## ADDED Requirements

### Requirement: Explored-area tracking
The simulation SHALL maintain a per-polygon and per-line exploration bitfield that persists for the lifetime of the current level. Each simulation tick, after player physics completes, the system SHALL mark the player's current polygon and all of its lines as explored. The system SHALL additionally mark each polygon adjacent to the player's current polygon (reachable through a non-solid line) and the shared border lines as explored. Exploration state SHALL be initialized to all-unexplored when a level loads.

#### Scenario: Player enters a new polygon
- **WHEN** the player moves into a polygon that has not been explored
- **THEN** that polygon and all of its boundary lines SHALL be marked as explored

#### Scenario: Adjacent polygons revealed
- **WHEN** the player is in polygon A which has non-solid lines connecting to polygons B and C
- **THEN** polygons B and C and their shared border lines with A SHALL be marked as explored

#### Scenario: Solid wall blocks exploration
- **WHEN** the player is in polygon A which connects to polygon D only through a solid wall (no non-solid connecting line)
- **THEN** polygon D SHALL NOT be marked as explored by adjacency from A

#### Scenario: Exploration persists
- **WHEN** the player has explored polygons A and B, then moves to polygon C far away
- **THEN** polygons A and B SHALL remain marked as explored

#### Scenario: Level load resets exploration
- **WHEN** a new level is loaded
- **THEN** all polygons and lines SHALL be marked as unexplored

### Requirement: Line-type classification
The system SHALL classify each map line into one of six visual categories for automap rendering, evaluated in priority order: (1) ControlPanel if any side on the line has the IS_CONTROL_PANEL flag, (2) Platform if an adjacent polygon is of type Platform, (3) Landscape if the line has the LANDSCAPE flag, (4) ElevationChange if the line has the ELEVATION or VARIABLE_ELEVATION flag, (5) Transparent if the line has the HAS_TRANSPARENT_SIDE flag, (6) SolidWall for all other lines. The classification SHALL be computed from MapData line flags, side flags, and polygon types.

#### Scenario: Control panel line
- **WHEN** a line has a side with IS_CONTROL_PANEL flag set
- **THEN** the line SHALL be classified as ControlPanel regardless of other flags

#### Scenario: Platform border line
- **WHEN** a line borders a polygon of type Platform and has no control panel side
- **THEN** the line SHALL be classified as Platform

#### Scenario: Elevation change line
- **WHEN** a line has the ELEVATION flag set and is not a control panel, platform border, or landscape line
- **THEN** the line SHALL be classified as ElevationChange

#### Scenario: Solid wall default
- **WHEN** a line has the SOLID flag and no higher-priority classification applies
- **THEN** the line SHALL be classified as SolidWall

### Requirement: Polygon-type coloring
The system SHALL assign a fill color to each explored polygon based on its type and media content. If the polygon has a valid media_index and the media height is above the polygon's floor height, the fill color SHALL be determined by media type: Water (dark blue), Lava (dark orange-red), Goo (dark green), Sewage (dark yellow-brown), Jjaro (dark purple). If no media is active, the fill color SHALL be determined by polygon type: Platform (dark red), Teleporter (dark cyan), MinorOuch (dark yellow), MajorOuch (dark red), all others (dark green).

#### Scenario: Water-filled polygon
- **WHEN** an explored polygon has media_index referencing Water media with height above the floor
- **THEN** the polygon SHALL be filled with a dark blue color

#### Scenario: Platform polygon without media
- **WHEN** an explored polygon is of type Platform with no active media
- **THEN** the polygon SHALL be filled with a dark red color

#### Scenario: Normal polygon
- **WHEN** an explored polygon is of type Normal with no media
- **THEN** the polygon SHALL be filled with a dark green color

### Requirement: Line rendering with type-based colors
The system SHALL render explored lines on the overhead map with colors determined by their classification: SolidWall (bright green), ElevationChange (medium green), Transparent (dim green), Landscape (brown), Platform (red), ControlPanel (bright red). Unexplored lines SHALL NOT be rendered. Line width SHALL scale with zoom level.

#### Scenario: Solid wall drawn green
- **WHEN** an explored line is classified as SolidWall
- **THEN** the line SHALL be rendered in bright green on the overhead map

#### Scenario: Unexplored line hidden
- **WHEN** a line has not been explored
- **THEN** the line SHALL NOT be drawn on the overhead map

#### Scenario: Control panel drawn red
- **WHEN** an explored line is classified as ControlPanel
- **THEN** the line SHALL be rendered in bright red on the overhead map

### Requirement: Entity blips
The system SHALL render entity markers on the overhead map for entities in explored polygons. The player SHALL be rendered as a directional arrow at the map center colored yellow. Monsters SHALL be rendered as small red rectangles at their world positions. Items SHALL be rendered as small white dots at their world positions. Entity blips SHALL blink at approximately 0.5 Hz (alternating visible/hidden based on tick count) to distinguish them from static geometry.

#### Scenario: Player arrow
- **WHEN** the overhead map is visible
- **THEN** a yellow directional arrow SHALL be drawn at the map center pointing in the player's facing direction

#### Scenario: Monster in explored area
- **WHEN** a monster is in an explored polygon and the blip is in its visible phase
- **THEN** a red rectangle SHALL be drawn at the monster's map position

#### Scenario: Monster in unexplored area
- **WHEN** a monster is in a polygon that has not been explored
- **THEN** no blip SHALL be drawn for that monster

#### Scenario: Blip blinking
- **WHEN** the simulation tick count modulo 30 is less than 15
- **THEN** entity blips (monsters, items) SHALL be visible; otherwise they SHALL be hidden

### Requirement: Coordinate transform with zoom
The system SHALL transform world coordinates to screen coordinates using a linear projection centered on the player's position: screen_x = center_x + (world_x - player_x) * zoom, screen_y = center_y + (world_y - player_y) * zoom. The zoom factor SHALL be adjustable between a minimum of 4.0 and a maximum of 48.0 pixels per world unit, with a default of 12.0. Zoom in SHALL be triggered by the + key (or = key) and zoom out by the - key.

#### Scenario: Default zoom
- **WHEN** the overhead map opens at default zoom (12.0)
- **THEN** a line 1 world unit long SHALL appear as 12 pixels on screen

#### Scenario: Zoom in
- **WHEN** the player presses + while the overhead map is visible
- **THEN** the zoom factor SHALL increase (up to 48.0), making nearby geometry appear larger

#### Scenario: Zoom out
- **WHEN** the player presses - while the overhead map is visible
- **THEN** the zoom factor SHALL decrease (down to 4.0), showing more of the level

#### Scenario: Zoom limits
- **WHEN** the zoom factor is at 48.0 and the player presses +
- **THEN** the zoom factor SHALL remain at 48.0

### Requirement: Overhead map toggle
The system SHALL toggle the overhead map overlay visibility when the player presses the Tab key. When visible, the overlay SHALL render as a semi-transparent layer on top of the 3D viewport with approximately 70% background opacity. When hidden, no overhead map rendering SHALL occur.

#### Scenario: Tab shows map
- **WHEN** the overhead map is hidden and the player presses Tab
- **THEN** the overhead map overlay SHALL become visible

#### Scenario: Tab hides map
- **WHEN** the overhead map is visible and the player presses Tab
- **THEN** the overhead map overlay SHALL be hidden

### Requirement: Desktop wgpu overhead map renderer
The desktop build (marathon-game) SHALL render the overhead map as a wgpu render pass using an orthographic projection. Polygons SHALL be rendered as flat colored triangles using fan triangulation. Lines SHALL be rendered as thin colored quads. The render pass SHALL disable depth testing and use alpha blending to composite over the 3D scene. A dedicated shader with flat vertex coloring (no textures) SHALL be used.

#### Scenario: Desktop automap renders over 3D scene
- **WHEN** the overhead map is visible in the desktop build
- **THEN** the automap SHALL render as a translucent 2D overlay on top of the 3D scene with correct polygon fills, colored lines, and entity blips

#### Scenario: Desktop automap does not corrupt depth buffer
- **WHEN** the overhead map render pass executes
- **THEN** the 3D scene depth buffer SHALL NOT be modified by the automap pass

### Requirement: Web Canvas 2D overhead map renderer
The web build (marathon-web) SHALL render the overhead map using the Canvas 2D API on a dedicated overlay canvas element. The renderer SHALL draw explored polygon fills first, then explored lines with type-based colors, then entity blips, then the player arrow. The canvas SHALL be styled as a fixed-position overlay with semi-transparent background.

#### Scenario: Web automap shows colored lines
- **WHEN** the overhead map is visible in the web build
- **THEN** explored lines SHALL be drawn with colors matching their type classification (green for walls, red for control panels, etc.)

#### Scenario: Web automap shows polygon fills
- **WHEN** the overhead map is visible and polygons have been explored
- **THEN** explored polygons SHALL be filled with colors based on their type (dark green for normal, dark red for platforms, etc.)

#### Scenario: Web automap shows entity blips
- **WHEN** the overhead map is visible and entities are in explored areas
- **THEN** monster rectangles (red) and item dots (white) SHALL appear at their map positions

### Requirement: Overhead map query API on SimWorld
SimWorld SHALL expose methods for renderers to query overhead map data: explored lines with classification and endpoint coordinates, explored polygons with classification and vertex coordinates, entity positions with type and facing, current zoom level, and visibility state. These methods SHALL return platform-agnostic data structures that both the desktop and web renderers consume.

#### Scenario: Query explored lines
- **WHEN** a renderer calls the explored lines query
- **THEN** it SHALL receive a list of lines with their world-space endpoints, visual classification, and explored status

#### Scenario: Query overhead entities
- **WHEN** a renderer calls the overhead entities query
- **THEN** it SHALL receive a list of entities with world position, facing angle, and entity kind (Player, Monster, Item)

#### Scenario: Query returns only explored data when fog enabled
- **WHEN** a renderer queries explored polygons
- **THEN** only polygons marked as explored in the ExploredMap SHALL have their explored flag set to true
