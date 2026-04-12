## MODIFIED Requirements

### Requirement: Advance simulation by one tick
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `TickInput`. All systems SHALL execute in the defined order: input processing, player physics, monster AI, weapon/combat, projectile physics, damage resolution, world mechanics, cleanup. The world mechanics phase SHALL call `run_world_mechanics()` which iterates all Platform entities, advances their state machines via `tick_platform()`, syncs updated heights into `MapGeometry`, checks activation triggers against entity positions, processes crush checks for entities on platform polygons, dispatches linked platform/light events, and emits sound events.

#### Scenario: Single tick with platform movement
- **WHEN** `tick()` is called and a platform is in the Extending state
- **THEN** the platform's height SHALL advance by its speed, MapGeometry SHALL be updated, and any crush checks SHALL be performed

#### Scenario: Tick order ensures physics before platforms
- **WHEN** `tick()` is called with `MOVE_FORWARD` and the player is on a platform polygon
- **THEN** player physics SHALL execute first (updating player position/polygon), then world mechanics SHALL check the player's updated polygon against platform activation triggers

#### Scenario: Platform activation in world mechanics phase
- **WHEN** the player's polygon (after physics) matches a platform polygon with `ACTIVATE_ON_PLAYER_ENTRY`
- **THEN** the world mechanics phase SHALL activate that platform during the same tick

#### Scenario: Empty tick still runs platforms
- **WHEN** `tick()` is called with empty `ActionFlags` and platforms are moving
- **THEN** the world mechanics phase SHALL still advance all moving platforms and sync MapGeometry
