## MODIFIED Requirements

### Requirement: Advance simulation by one tick
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `TickInput`. All eight systems SHALL execute in the defined canonical order: (1) update lights, (2) update media, (3) update platforms, (4) player physics, (5) update monsters, (6) update projectiles, (7) update effects, (8) update items. The tick counter SHALL be incremented after all systems complete. Each system SHALL be implemented as a private method on `SimWorld` called sequentially from `tick()`.

#### Scenario: All systems execute in order
- **WHEN** `tick()` is called with any `TickInput`
- **THEN** the eight systems SHALL execute in the defined order: lights, media, platforms, player physics, monsters, projectiles, effects, items, and the tick counter SHALL increment by 1

#### Scenario: Single tick with forward movement
- **WHEN** `tick()` is called with `ActionFlags::MOVE_FORWARD`
- **THEN** the player's position SHALL change according to movement physics, lights SHALL have their intensities recomputed, platforms SHALL have advanced their state machines, and all other systems SHALL have executed

#### Scenario: Empty action flags still advances world
- **WHEN** `tick()` is called with empty `ActionFlags`
- **THEN** lights SHALL still animate, platforms SHALL still move, monster AI SHALL still evaluate targets, projectiles SHALL still advance, effects SHALL still count down, and items SHALL still check for pickups

#### Scenario: Light update precedes media update
- **WHEN** a media entity's associated light has the Smooth function with a 60-tick period
- **THEN** the media's `current_height` after the tick SHALL reflect the light intensity computed during the same tick's light update phase (not a stale value from the previous tick)

#### Scenario: Platform update precedes player physics
- **WHEN** a platform is actively extending and the player is on the platform's polygon
- **THEN** the player physics system SHALL use the updated floor height from the platform's tick (not the previous tick's floor height) for collision and grounding

#### Scenario: Monster update follows player physics
- **WHEN** the player moves during player physics and a monster checks line-of-sight
- **THEN** the monster's vision check SHALL use the player's updated position from the current tick's player physics pass
