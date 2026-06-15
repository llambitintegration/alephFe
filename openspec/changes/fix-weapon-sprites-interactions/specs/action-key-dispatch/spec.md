## ADDED Requirements

### Requirement: Action key ray-cast target finding
The system SHALL, when the ACTION flag is set in `TickInput`, cast a 2D ray from the player's position in the player's facing direction to find interaction targets. The ray SHALL traverse polygons using the map's adjacency data, checking each crossed line for platforms (doors) and control panels. The maximum ray distance SHALL be 3.0 world units (`MAXIMUM_ACTIVATION_RANGE`).

#### Scenario: Ray hits door platform
- **WHEN** the player presses the action key while facing a polygon that is a door platform within 3.0 world units
- **THEN** the system SHALL identify the platform as the target and dispatch a platform activation

#### Scenario: Ray hits control panel
- **WHEN** the player presses the action key while facing a wall side with a control panel within 1.5 world units (`MAXIMUM_CONTROL_ACTIVATION_RANGE`)
- **THEN** the system SHALL identify the control panel as the target and dispatch a panel activation

#### Scenario: Ray hits nothing
- **WHEN** the player presses the action key but no platform or control panel is within range along the facing direction
- **THEN** no activation SHALL occur

#### Scenario: Panel behind player
- **WHEN** a control panel is 1.0 world units behind the player and the player presses the action key
- **THEN** the panel SHALL NOT be activated because the ray only extends in the facing direction

### Requirement: Polygon traversal for ray-cast
The system SHALL traverse polygons along the ray by finding which polygon edge the ray crosses (using cross-product edge intersection tests), then moving to the adjacent polygon across that edge. The traversal SHALL stop when: a valid target is found, the ray reaches a solid line with no adjacent polygon, or no more edges are crossed.

#### Scenario: Ray crosses multiple polygons
- **WHEN** the player is in polygon A, facing through polygon B toward a door in polygon C
- **THEN** the ray SHALL traverse A → B → C and find the door platform in polygon C

#### Scenario: Ray blocked by solid wall
- **WHEN** the player faces a solid wall with no adjacent polygon behind it
- **THEN** the traversal SHALL stop at that wall and report no target (unless the wall has a control panel)

### Requirement: Action key dispatch to handlers
The system SHALL dispatch the found target to the appropriate handler: platforms dispatch to `player_touch_platform_state` logic, control panels dispatch to `change_panel_state` logic. The dispatch SHALL occur within the tick loop after player physics and before item updates.

#### Scenario: Dispatch platform activation
- **WHEN** the ray-cast finds a door platform target
- **THEN** the system SHALL call the platform activation handler which toggles the platform state

#### Scenario: Dispatch panel activation
- **WHEN** the ray-cast finds a control panel target
- **THEN** the system SHALL call the panel activation handler which triggers the panel's associated action (platform toggle, light toggle, or terminal event)
