## MODIFIED Requirements

### Requirement: CI fetches all three Marathon trilogy datasets
The CI pipeline SHALL fetch Marathon 1, Marathon 2, and Marathon Infinity scenario data from their respective public GitHub repositories before running tests. The fetched data MUST NOT be committed to the repository.

#### Scenario: Marathon 1 data available during CI test run
- **WHEN** the CI test stage executes
- **THEN** the Marathon 1 files (Map, Shapes, Sounds, Physics Model) SHALL exist in `tests/fixtures/marathon-1/` and be accessible to the test binary

#### Scenario: Marathon 2 data available during CI test run
- **WHEN** the CI test stage executes
- **THEN** the Marathon 2 files (Map, Shapes, Sounds, Physics Model) SHALL exist in `tests/fixtures/marathon-2/` and be accessible to the test binary

#### Scenario: Marathon Infinity data available during CI test run
- **WHEN** the CI test stage executes
- **THEN** the Marathon Infinity files (Map, Shapes, Sounds, Physics Model) SHALL exist in `tests/fixtures/marathon-infinity/` and be accessible to the test binary

#### Scenario: All data fetches use pinned commits from scenarios.toml
- **WHEN** the CI pipeline clones each Marathon data repository
- **THEN** it SHALL use the specific pinned commit hash declared in `tests/scenarios.toml` for each source
- **AND** the Marathon 2 commit SHALL remain `eaf21a7e9f72706c4c2ff9a2960c4367f739f04d` for backward compatibility

### Requirement: Fixture directory structure is reorganized
Marathon 2 fixture files SHALL be relocated from `tests/fixtures/` (root) to `tests/fixtures/marathon-2/` to accommodate multi-scenario data alongside Marathon 1 and Infinity directories.

#### Scenario: Marathon 2 files move to subdirectory
- **WHEN** the CI fetches Marathon 2 data
- **THEN** the files SHALL be placed in `tests/fixtures/marathon-2/Map`, `tests/fixtures/marathon-2/Shapes`, `tests/fixtures/marathon-2/Sounds`, and `tests/fixtures/marathon-2/Physics Model`

#### Scenario: Backward compatibility during transition
- **WHEN** tests reference Marathon 2 fixture files
- **THEN** the test code SHALL first check `tests/fixtures/marathon-2/<file>` and fall back to `tests/fixtures/<file>` if the subdirectory path does not exist

### Requirement: Real-data tests are parameterized over the golden level corpus
Tests in `real_data_tests.rs` SHALL iterate over golden levels declared in `tests/scenarios.toml` rather than hardcoding file paths and expected values for a single scenario.

#### Scenario: Tier 1 tests cover all trilogy datasets
- **WHEN** Marathon 1, 2, and Infinity data are all present
- **THEN** Tier 1 format parsing tests SHALL execute against levels from all three datasets
- **AND** each level's geometry counts SHALL be asserted against its `tier1` golden values from the manifest

#### Scenario: Marathon 1 WAD parsing test runs
- **WHEN** Marathon 1 Map file is present in `tests/fixtures/marathon-1/`
- **THEN** the test SHALL parse the WAD file, verify it has WAD version 0 or 1, and assert golden endpoint/line/polygon counts for declared Marathon 1 levels

#### Scenario: Marathon Infinity WAD parsing test runs
- **WHEN** Marathon Infinity Map file is present in `tests/fixtures/marathon-infinity/`
- **THEN** the test SHALL parse the WAD file, verify it has WAD version 4, and assert golden endpoint/line/polygon counts for declared Marathon Infinity levels

#### Scenario: Existing Marathon 2 snapshot assertions are preserved
- **WHEN** Marathon 2 `Map` is parsed from `tests/fixtures/marathon-2/`
- **THEN** the test SHALL assert: 41 total levels, level 0 has 716 endpoints, 1106 lines, 369 polygons (matching existing hardcoded values now sourced from the manifest)

### Requirement: Dockerfile fetch-data stage fetches all three repos
The `Dockerfile` `fetch-data` stage SHALL clone all three Marathon trilogy data repositories.

#### Scenario: Dockerfile clones Marathon 1 data
- **WHEN** the `fetch-data` Docker stage runs
- **THEN** it SHALL clone `data-marathon` at the pinned commit and copy Map, Shapes, Sounds, and Physics Model to `tests/fixtures/marathon-1/`

#### Scenario: Dockerfile clones Marathon Infinity data
- **WHEN** the `fetch-data` Docker stage runs
- **THEN** it SHALL clone `data-marathon-infinity` at the pinned commit and copy Map, Shapes, Sounds, and Physics Model to `tests/fixtures/marathon-infinity/`

#### Scenario: Dockerfile preserves existing Marathon 2 fetch
- **WHEN** the `fetch-data` Docker stage runs
- **THEN** it SHALL continue to clone `data-marathon-2` at commit `eaf21a7e9f72706c4c2ff9a2960c4367f739f04d`
- **AND** it SHALL place Marathon 2 files in both `tests/fixtures/marathon-2/` and `tests/fixtures/` (for backward compatibility during transition)
