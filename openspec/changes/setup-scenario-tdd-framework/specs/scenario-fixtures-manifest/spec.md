## ADDED Requirements

### Requirement: Scenario manifest exists at `tests/scenarios.toml`
The project SHALL have a declarative TOML manifest at `tests/scenarios.toml` that defines all golden test levels, their data sources, and per-tier expected values.

#### Scenario: Manifest file is parseable
- **WHEN** `tests/scenarios.toml` is read and parsed as TOML
- **THEN** it SHALL parse without errors
- **AND** it SHALL contain a `[sources]` table and at least one `[[levels]]` entry

#### Scenario: Manifest is committed to the repository
- **WHEN** the repository is cloned
- **THEN** `tests/scenarios.toml` SHALL be present (not gitignored)
- **AND** it SHALL be the single source of truth for golden level definitions

### Requirement: Data sources are declared with pinned commits
Each data source SHALL declare a Git repository URL, a pinned commit hash, and a local directory path for CI-fetched data.

#### Scenario: Source entry has required fields
- **WHEN** a `[sources.<name>]` entry is parsed
- **THEN** it SHALL have `repo` (string, HTTPS GitHub URL), `commit` (string, 40-char hex SHA), and `local_dir` (string, relative path from repo root)

#### Scenario: At least three trilogy sources are declared
- **WHEN** `tests/scenarios.toml` is parsed
- **THEN** it SHALL contain entries for `sources.marathon-1`, `sources.marathon-2`, and `sources.marathon-infinity`

### Requirement: Level entries declare identity and metadata
Each `[[levels]]` entry SHALL uniquely identify a level and declare its source, file paths, and feature coverage.

#### Scenario: Level entry has required identity fields
- **WHEN** a `[[levels]]` entry is parsed
- **THEN** it SHALL have: `id` (unique string), `source` (string matching a `[sources.*]` key), `wad_path` (string, file name within source directory), `level_index` (integer >= 0), and `name` (human-readable string)

#### Scenario: Level entry declares features
- **WHEN** a `[[levels]]` entry is parsed
- **THEN** it SHALL have a `features` array of strings describing the engine features the level exercises

#### Scenario: Level entry may declare MacBinary stripping
- **WHEN** a `[[levels]]` entry is parsed and the WAD file has a MacBinary header
- **THEN** the entry SHALL have `wad_strip_macbinary = true`
- **AND** the test harness SHALL strip the first 128 bytes before parsing

### Requirement: Per-tier golden values are optional and extensible
Each level entry MAY have `[levels.tier1]`, `[levels.tier2]`, and `[levels.tier3]` sub-tables with tier-specific expected values.

#### Scenario: Tier 1 golden values
- **WHEN** a level entry has a `[levels.tier1]` sub-table
- **THEN** it SHALL contain `endpoints` (integer), `lines` (integer), and `polygons` (integer) fields representing expected geometry counts

#### Scenario: Tier 2 golden values
- **WHEN** a level entry has a `[levels.tier2]` sub-table
- **THEN** it SHALL contain: `tick_count` (integer), `input_script` (string), `player_x` (float), `player_y` (float), and `player_polygon` (integer)

#### Scenario: Tier 3 golden values
- **WHEN** a level entry has a `[levels.tier3]` sub-table
- **THEN** it SHALL contain: `camera_yaw` (float), `camera_pitch` (float), `min_coverage` (float, 0.0-1.0), `min_unique_colors` (integer), and `min_quadrants` (integer, 1-4)

#### Scenario: Levels without tier values are skipped for that tier
- **WHEN** a test harness iterates golden levels for a specific tier
- **THEN** levels without the corresponding `[levels.tierN]` sub-table SHALL be skipped without error

### Requirement: Manifest supports deferred levels
Levels that cannot yet be tested (e.g., total conversion levels blocked on MML/Lua engines) SHALL be declarable as deferred.

#### Scenario: Deferred level is skipped by all tiers
- **WHEN** a `[[levels]]` entry has `deferred = true`
- **THEN** all tier test harnesses SHALL skip the level
- **AND** the skip message SHALL include the level's `id` and the reason for deferral

### Requirement: Community contributors can add levels
Adding a new golden level SHALL require only appending a `[[levels]]` entry to `tests/scenarios.toml` with the appropriate golden values.

#### Scenario: New level addition workflow
- **WHEN** a contributor appends a new `[[levels]]` entry to `tests/scenarios.toml`
- **THEN** the next CI run SHALL automatically include the new level in all tiers for which it has golden values
- **AND** no Rust or TypeScript test code modifications SHALL be required
