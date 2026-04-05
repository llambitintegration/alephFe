## ADDED Requirements

### Requirement: GPL fixture files committed to repository
The test suite SHALL include real Aleph One engine MML and Plugin.xml files in `tests/fixtures/alephone/`, committed to the repository under GPL-3.0 license.

#### Scenario: MML files covering untested sections
- **WHEN** tests run against the committed Aleph One MML fixtures
- **THEN** the MML parser SHALL successfully parse files containing `<console>`, `<opengl>`, `<interface>`, and `<scenario>` sections

#### Scenario: Plugin.xml files covering real-world attributes
- **WHEN** tests run against the committed Aleph One Plugin.xml fixtures
- **THEN** the plugin parser SHALL successfully parse files using `hud_lua`, `stats_lua`, `theme_dir`, and multi-`<scenario>` attributes

### Requirement: CI fetches Marathon 2 scenario data
The CI pipeline SHALL fetch Marathon 2 scenario data from the public `github.com/Aleph-One-Marathon/data-marathon-2.git` repository before running tests. The fetched data MUST NOT be committed to the repository.

#### Scenario: Marathon 2 data available during CI test run
- **WHEN** the CI test stage executes
- **THEN** the files `Map.sceA`, `Shapes`, `Sounds`, and `Physics Model` SHALL exist in `tests/fixtures/` and be accessible to the test binary

#### Scenario: Data fetch uses pinned commit
- **WHEN** the CI pipeline clones the Marathon 2 data repository
- **THEN** it SHALL use a specific pinned commit hash (not HEAD) to ensure reproducible test results

### Requirement: Real-data tests run in CI
All tests in `real_data_tests.rs` that are gated on fixture file existence SHALL execute successfully in CI when the Marathon 2 data has been fetched.

#### Scenario: WAD map parsing test runs
- **WHEN** `Map.sceA` is present in fixtures
- **THEN** `test_wad_m2_map_parsing` SHALL parse the file and assert a valid WAD header with version >= 0 and at least one entry

#### Scenario: Shapes parsing test runs
- **WHEN** `Shapes` is present in fixtures
- **THEN** `test_shapes_parsing` SHALL parse the file, find 32 collection headers, and successfully parse at least one collection with CLUTs and bitmaps

#### Scenario: Sounds parsing test runs
- **WHEN** `Sounds` is present in fixtures
- **THEN** `test_sounds_parsing` SHALL parse the file and validate the `snd2` header tag and sound definitions

#### Scenario: Physics parsing test runs
- **WHEN** `Physics Model` is present in fixtures
- **THEN** `test_physics_parsing` SHALL parse the physics WAD and extract physics constants with positive forward velocity values

#### Scenario: Map geometry parsing test runs
- **WHEN** `Map.sceA` is present in fixtures
- **THEN** `test_map_geometry_parsing` SHALL parse level 0 and find non-empty endpoints, lines, and polygons

#### Scenario: Cross-format test runs
- **WHEN** all fixture files (Map, Shapes, Sounds) are present
- **THEN** `test_community_scenario_cross_format` SHALL parse all formats without error

### Requirement: Snapshot assertions on known Marathon 2 data
Tests against fetched Marathon 2 data SHALL include snapshot-style assertions that verify specific known-good values, catching silent parsing regressions.

#### Scenario: Level 0 geometry counts are stable
- **WHEN** Marathon 2 `Map.sceA` level 0 is parsed
- **THEN** the endpoint count, line count, and polygon count SHALL match hardcoded expected values derived from the pinned data commit

#### Scenario: Physics constants are stable
- **WHEN** Marathon 2 `Standard.phyA` is parsed
- **THEN** the number of physics constant entries and at least one known velocity value SHALL match hardcoded expected values

### Requirement: Local tests degrade gracefully without fetched data
Tests that depend on fetched Marathon 2 data SHALL skip gracefully when the fixture files are absent, printing a `SKIP:` message to stderr.

#### Scenario: Running locally without game data
- **WHEN** a developer runs `cargo test` without Marathon 2 data in fixtures
- **THEN** real-data tests SHALL print `SKIP: <reason>` and return success (not fail or panic)

#### Scenario: GPL fixture tests always run
- **WHEN** a developer runs `cargo test` in any environment
- **THEN** tests against committed GPL fixtures SHALL always execute (never skip)
