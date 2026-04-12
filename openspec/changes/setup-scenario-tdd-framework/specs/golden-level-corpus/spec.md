## ADDED Requirements

### Requirement: Golden level corpus covers all three Marathon trilogy datasets
The project SHALL maintain a curated set of golden test levels spanning Marathon 1, Marathon 2, and Marathon Infinity, selected for maximum engine feature coverage.

#### Scenario: Marathon 1 levels are included
- **WHEN** the golden level corpus is enumerated from `tests/scenarios.toml`
- **THEN** at least 3 Marathon 1 levels SHALL be declared
- **AND** at least one level SHALL exercise WAD v0/v1 format parsing
- **AND** at least one level SHALL exercise vacuum mechanics
- **AND** at least one level SHALL exercise platform puzzles

#### Scenario: Marathon 2 levels are included
- **WHEN** the golden level corpus is enumerated from `tests/scenarios.toml`
- **THEN** at least 10 Marathon 2 levels SHALL be declared
- **AND** the levels SHALL collectively cover: water/liquid rendering, combat AI, terminal sequences, platform mechanics, complex geometry, 5D space, elevators, projectile paths, and net level structure

#### Scenario: Marathon Infinity levels are included
- **WHEN** the golden level corpus is enumerated from `tests/scenarios.toml`
- **THEN** at least 4 Marathon Infinity levels SHALL be declared
- **AND** at least one level SHALL exercise WAD v4 format parsing
- **AND** at least one level SHALL exercise dream-sequence architecture
- **AND** at least one level SHALL exercise complex overlapping geometry

#### Scenario: Each golden level has a unique identifier
- **WHEN** `tests/scenarios.toml` is parsed
- **THEN** each `[[levels]]` entry SHALL have a unique `id` field
- **AND** the `id` SHALL follow the pattern `{source}-{kebab-case-name}` (e.g., `m2-waterloo-waterpark`)

#### Scenario: Each golden level declares its feature coverage
- **WHEN** a `[[levels]]` entry is read from `tests/scenarios.toml`
- **THEN** the entry SHALL have a `features` array listing the engine features the level exercises
- **AND** the union of all levels' `features` arrays SHALL cover at minimum: `water`, `platforms`, `terminals`, `combat`, `vacuum`, `5d-geometry`, `elevators`, `dream-architecture`, `liquid`, `complex-geometry`

### Requirement: Golden level data sources are pinned and reproducible
Each data source in the manifest SHALL reference a specific Git commit hash, ensuring reproducible test results.

#### Scenario: Data sources use pinned commits
- **WHEN** `tests/scenarios.toml` is parsed
- **THEN** each `[sources.*]` entry SHALL have a `commit` field containing a full 40-character Git SHA
- **AND** the `repo` field SHALL be a valid HTTPS GitHub URL

#### Scenario: Marathon 2 source uses the existing pinned commit
- **WHEN** the `[sources.marathon-2]` entry is read
- **THEN** the `commit` field SHALL be `eaf21a7e9f72706c4c2ff9a2960c4367f739f04d` (matching the existing Dockerfile pin)

### Requirement: Total conversion levels are reserved but deferred
The manifest SHALL include placeholder entries for total conversion levels that require MML/Lua testing, but these SHALL be marked as deferred.

#### Scenario: Total conversion placeholders exist
- **WHEN** `tests/scenarios.toml` is parsed
- **THEN** there SHALL be at least 5 `[[levels]]` entries with `source = "tc-*"` or a `deferred = true` flag
- **AND** each deferred entry SHALL have a comment explaining the blocking dependency (MML engine, Lua engine, or licensing)
