## MODIFIED Requirements

### Requirement: Level loading and initialization
The system SHALL load a level by: (1) parsing the map entry from the WadFile via marathon-formats, (2) assembling the MML override cascade from global MML, local MML, scenario MML, plugin MML, and level-embedded MML, interpreting it into an `MmlOverrideSet`, (3) applying the `MmlOverrideSet`'s physics overrides to the parsed `PhysicsData`, (4) initializing marathon-sim with the overridden physics data, map data, dynamic limits from the `MmlOverrideSet`, and game mode, (5) initializing the rendering pipeline with map geometry, textures, and MML overrides for landscapes and texture loading, (6) optionally initializing marathon-audio with the map data and sound definitions, (7) transitioning to the `Playing` state. The scenario+plugin MML base SHALL be cached so that level transitions only re-apply level-embedded MML. All parsing, override resolution, and initialization SHALL complete before gameplay begins. Audio initialization failure SHALL be non-fatal.

#### Scenario: Load level with scenario MML overrides
- **WHEN** the player starts a campaign whose scenario MML overrides monster 3's vitality to 500
- **THEN** the system SHALL assemble the MML cascade, apply monster 3's vitality override to physics data, and initialize the simulation with the overridden value

#### Scenario: Load level with plugin MML stacking
- **WHEN** two enabled plugins both modify monsters (plugin A changes monster 0, plugin B changes monster 5)
- **THEN** the system SHALL layer plugin A's MML then plugin B's MML (alphabetical by plugin name), preserving both plugins' changes in the resolved override set

#### Scenario: Level transition re-applies level-embedded MML
- **WHEN** the player transitions from level 2 (which has embedded MML setting player energy to 100) to level 3 (which has no embedded MML)
- **THEN** the system SHALL use the cached scenario+plugin MML base for level 3, without level 2's player energy override carrying over

#### Scenario: Level load with no MML sources
- **WHEN** a level is loaded from a scenario that has no MML files, no plugins, and no embedded MML
- **THEN** the system SHALL use unmodified physics data and default configuration values

#### Scenario: Level load failure
- **WHEN** a level's map data fails to parse
- **THEN** the system SHALL display an error message and transition to `MainMenu`

#### Scenario: Audio unavailable during level load
- **WHEN** the audio subsystem is not available during level loading
- **THEN** the system SHALL skip audio initialization and proceed with visual-only gameplay

### Requirement: Scenario and plugin discovery feeds MML cascade
The system SHALL, during scenario loading, discover all enabled plugins via the existing `discover_plugins()` function and collect their `mml_files` lists. For each plugin (in alphabetical order by name), the system SHALL parse each MML file and layer it into the cascade. Global MML files SHALL be discovered from `MML/` subdirectories and local MML files from `Scripts/` subdirectories, both in alphabetical order. The assembled scenario+plugin MML base SHALL be stored for reuse across level transitions.

#### Scenario: Plugins discovered and MML files loaded
- **WHEN** the scenario directory contains two plugins, "Alpha" with `a.mml` and "Beta" with `b.mml`
- **THEN** the system SHALL parse `a.mml` first (Alpha is alphabetically first), then `b.mml`, layering each on top of the previous cascade state

#### Scenario: Plugin with multiple MML files
- **WHEN** plugin "Gamma" lists `["effects.mml", "monsters.mml"]` in its MML files
- **THEN** the system SHALL parse `effects.mml` then `monsters.mml` in that order, layering each

#### Scenario: No plugins present
- **WHEN** the scenario has no plugin directory or no enabled plugins
- **THEN** the MML cascade SHALL proceed with global, local, and scenario MML only
