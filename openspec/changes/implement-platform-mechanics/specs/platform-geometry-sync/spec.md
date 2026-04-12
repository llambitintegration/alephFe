## ADDED Requirements

### Requirement: Sync platform heights to MapGeometry each tick
The system SHALL write each platform's `current_floor` and `current_ceiling` back into `MapGeometry.floor_heights[polygon_index]` and `MapGeometry.ceiling_heights[polygon_index]` after ticking all platforms each simulation tick. This ensures that collision detection (which reads MapGeometry) and mesh generation (which reads MapGeometry) reflect the platform's actual position.

#### Scenario: Elevator raises floor height
- **WHEN** a FromFloor platform ticks and its `current_floor` changes from 0.0 to 0.5
- **THEN** `MapGeometry.floor_heights[polygon_index]` SHALL equal 0.5

#### Scenario: Door lowers ceiling
- **WHEN** an ExtendsCeilingToFloor platform ticks and its `current_ceiling` changes from 3.0 to 1.5
- **THEN** `MapGeometry.ceiling_heights[polygon_index]` SHALL equal 1.5

#### Scenario: Platform at rest does not write
- **WHEN** a platform is in AtRest state and its heights have not changed
- **THEN** MapGeometry values SHALL remain unchanged (no unnecessary writes)

### Requirement: Track changed polygons for renderer notification
The system SHALL maintain a `changed_polygons: Vec<bool>` field on `MapGeometry`, one entry per polygon, initialized to `false`. When platform ticking updates a polygon's floor or ceiling height, the corresponding entry SHALL be set to `true`. A convenience field `has_changes: bool` SHALL be set to `true` if any polygon was modified during the current tick, enabling renderers to skip the check entirely when no platforms moved.

#### Scenario: Moving platform marks polygon dirty
- **WHEN** platform ticking changes polygon 7's floor height
- **THEN** `MapGeometry.changed_polygons[7]` SHALL be `true` and `MapGeometry.has_changes` SHALL be `true`

#### Scenario: No platforms moving
- **WHEN** all platforms are at rest during a tick
- **THEN** `MapGeometry.has_changes` SHALL be `false`

#### Scenario: Renderer clears dirty flags
- **WHEN** the renderer reads `has_changes == true`, processes the changed polygons, and clears the flags
- **THEN** `has_changes` SHALL be `false` and all `changed_polygons` entries SHALL be `false`

### Requirement: Platform type determines height range calculation
The system SHALL compute `floor_rest`, `floor_extended`, `ceiling_rest`, and `ceiling_extended` for each platform based on its `platform_type` field and the polygon's initial floor and ceiling heights from the map data. The six platform types SHALL be computed as follows:

- **ExtendsFloorToCeiling (type 0):** `floor_rest = polygon_floor`, `floor_extended = polygon_ceiling`. Ceiling remains at `polygon_ceiling`. Used for doors that close by raising the floor.
- **ExtendsCeilingToFloor (type 1):** `ceiling_rest = polygon_ceiling`, `ceiling_extended = polygon_floor`. Floor remains at `polygon_floor`. Used for doors that open by raising the ceiling.
- **ExtendsFloorAndCeiling (type 2):** Both floor and ceiling move toward each other. `floor_extended` and `ceiling_extended` are computed from `minimum_height` and `maximum_height`.
- **FromFloor (type 3):** `floor_rest = minimum_height`, `floor_extended = maximum_height`. Ceiling remains at `polygon_ceiling`. Used for elevators.
- **FromCeiling (type 4):** `ceiling_rest = maximum_height`, `ceiling_extended = minimum_height`. Floor remains at `polygon_floor`. Used for crushers.
- **Teleporter (type 5):** No height movement. Heights remain at polygon defaults.

#### Scenario: Door platform (type 1) computes ceiling range
- **WHEN** a platform with `platform_type = 1` is spawned on a polygon with `floor_height = 0.0` and `ceiling_height = 3.0`
- **THEN** `ceiling_rest` SHALL be 3.0, `ceiling_extended` SHALL be 0.0, `floor_rest` SHALL be 0.0, and `floor_extended` SHALL be 0.0

#### Scenario: Elevator platform (type 3) computes floor range
- **WHEN** a platform with `platform_type = 3` is spawned with `minimum_height = 0.0` and `maximum_height = 2.0` on a polygon with `ceiling_height = 4.0`
- **THEN** `floor_rest` SHALL be 0.0, `floor_extended` SHALL be 2.0, `ceiling_rest` SHALL be 4.0, and `ceiling_extended` SHALL be 4.0

#### Scenario: Crusher platform (type 4) computes ceiling range
- **WHEN** a platform with `platform_type = 4` is spawned with `minimum_height = 0.5` and `maximum_height = 3.0` on a polygon with `floor_height = 0.0`
- **THEN** `ceiling_rest` SHALL be 3.0, `ceiling_extended` SHALL be 0.5, `floor_rest` SHALL be 0.0, and `floor_extended` SHALL be 0.0

#### Scenario: Teleporter platform (type 5) has no height movement
- **WHEN** a platform with `platform_type = 5` is spawned on a polygon with `floor_height = 1.0` and `ceiling_height = 4.0`
- **THEN** all rest and extended heights SHALL equal the polygon's static heights, and no height changes SHALL occur when ticked
