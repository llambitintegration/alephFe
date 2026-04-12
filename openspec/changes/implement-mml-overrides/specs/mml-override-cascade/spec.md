## ADDED Requirements

### Requirement: Assemble override cascade from all MML sources
The system SHALL assemble MML overrides by layering documents from multiple sources in this order (later sources override earlier ones): (1) engine defaults (empty document), (2) global MML files from `MML/` subdirectories of data directories in alphabetical order, (3) local MML files from `Scripts/` subdirectory in alphabetical order, (4) scenario MML embedded in the scenario WAD's global entry, (5) plugin MML files for each enabled plugin in alphabetical plugin name order (within each plugin, MML files in their listed order), (6) level-embedded MML from the current level's MMLS WAD tag. The final merged `MmlDocument` SHALL then be interpreted into a typed `MmlOverrideSet`.

#### Scenario: Plugin MML overrides scenario MML
- **WHEN** the scenario MML sets monster 5 vitality to 200 and a plugin MML sets monster 5 vitality to 500
- **THEN** the resolved `MmlOverrideSet` SHALL contain monster 5 vitality as 500

#### Scenario: Level-embedded MML overrides plugin MML
- **WHEN** a plugin sets the player energy to 300 and the current level's embedded MML sets player energy to 100
- **THEN** the resolved `MmlOverrideSet` SHALL contain player energy as 100

#### Scenario: Earlier plugin preserves values not overridden by later plugin
- **WHEN** plugin A (alphabetically first) sets monster 3 vitality to 100 and speed to 5, and plugin B sets monster 3 vitality to 200 (but does not set speed)
- **THEN** the resolved override for monster 3 SHALL have vitality 200 (from B) and speed 5 (from A)

#### Scenario: Global MML provides baseline
- **WHEN** a global MML file sets dynamic_limits monsters to 1024 and no later source overrides it
- **THEN** the resolved `MmlOverrideSet` SHALL contain dynamic_limits monsters as 1024

### Requirement: Produce typed MmlOverrideSet from merged document
The system SHALL interpret the final merged `MmlDocument` into an `MmlOverrideSet` struct containing typed override data for each section. The `MmlOverrideSet` SHALL expose typed accessors for at minimum: monster overrides, weapon overrides, projectile overrides, effect overrides, player overrides, dynamic limits overrides, item overrides, landscape overrides, texture loading overrides, string set overrides, and scenario identification. Sections that are absent from the merged document SHALL produce empty/default override data (no overrides applied).

#### Scenario: MmlOverrideSet from document with monsters and weapons
- **WHEN** the merged `MmlDocument` has populated `monsters` and `weapons` sections
- **THEN** the `MmlOverrideSet` SHALL have non-empty monster override list and weapon override data, with all other sections at their defaults

#### Scenario: MmlOverrideSet from empty document
- **WHEN** the merged `MmlDocument` has no populated sections
- **THEN** the `MmlOverrideSet` SHALL have all sections at their defaults (empty lists, no overrides)

### Requirement: Cache scenario+plugin base across level transitions
The system SHALL cache the merged `MmlDocument` produced by steps 1-5 of the cascade (everything except level-embedded MML) so that level transitions only need to re-apply step 6 (level-embedded MML) on top of the cached base. The cache SHALL be invalidated when the scenario or plugin set changes.

#### Scenario: Level transition re-applies only level MML
- **WHEN** the player transitions from level 2 to level 3 within the same scenario
- **THEN** the system SHALL use the cached scenario+plugin MML base, layer the new level's embedded MML on top, and produce a new `MmlOverrideSet` without re-parsing global, local, scenario, or plugin MML files

#### Scenario: Cache invalidated on scenario change
- **WHEN** the player switches to a different scenario
- **THEN** the cached MML base SHALL be discarded and rebuilt from the new scenario's sources

### Requirement: Apply physics overrides to PhysicsData before SimWorld construction
The system SHALL apply the resolved `MmlOverrideSet`'s physics-related overrides (monster, weapon, projectile, effect, physics constants) to the `PhysicsData` loaded from the WAD before constructing `SimWorld`. Each override with a matching index SHALL update only the fields that have `Some` values, leaving other fields at their WAD-parsed values. Override indices that exceed the physics data array bounds SHALL be silently ignored.

#### Scenario: Monster override applied to PhysicsData
- **WHEN** the WAD contains 47 monster definitions and the `MmlOverrideSet` contains a `MonsterOverride` for index 5 with `vitality=Some(300)`
- **THEN** `PhysicsData.monsters[5].vitality` SHALL be 300 after override application, and all other monster definitions SHALL be unchanged

#### Scenario: Override index out of bounds ignored
- **WHEN** the WAD contains 10 monster definitions and the `MmlOverrideSet` contains a `MonsterOverride` for index 50
- **THEN** the override SHALL be silently ignored and no panic or error SHALL occur

#### Scenario: Multiple fields overridden on one definition
- **WHEN** a `MonsterOverride` for index 3 has `vitality=Some(500)`, `speed=Some(15)`, and `radius=None`
- **THEN** monster 3's vitality SHALL be 500, speed SHALL be 15, and radius SHALL retain its WAD-parsed value

### Requirement: Pass MmlOverrideSet to non-physics subsystems
The system SHALL pass the resolved `MmlOverrideSet` to the rendering pipeline, HUD system, and other non-physics subsystems that need MML-configured data. The `MmlOverrideSet` SHALL be available as a shared reference during level initialization and gameplay. Subsystems SHALL read their relevant sections (landscape overrides, texture loading overrides, string tables, dynamic limits, interface layout) from the `MmlOverrideSet`.

#### Scenario: Renderer receives landscape overrides
- **WHEN** the `MmlOverrideSet` contains landscape overrides for collection 27
- **THEN** the rendering pipeline SHALL receive the `MmlOverrideSet` and be able to query landscape configuration for collection 27

#### Scenario: Dynamic limits available to SimWorld
- **WHEN** the `MmlOverrideSet` contains dynamic_limits with `monsters=Some(1024)`
- **THEN** the simulation world initialization SHALL have access to the overridden monster limit
