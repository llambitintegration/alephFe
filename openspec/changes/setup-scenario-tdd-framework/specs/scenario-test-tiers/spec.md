## ADDED Requirements

### Requirement: Six-tier test taxonomy is defined and enforced
The project SHALL define a 6-tier test taxonomy (Tier 0 through Tier 5) with clear scope, data dependencies, and promotion criteria for each tier.

#### Scenario: Tier 0 tests run without external data
- **WHEN** `cargo test` is run without any scenario data in `tests/fixtures/`
- **THEN** all Tier 0 (synthetic unit) tests SHALL pass using only handcrafted/in-memory test data
- **AND** no Tier 0 test SHALL depend on files in `tests/fixtures/marathon-*`

#### Scenario: Tier 1 tests skip gracefully without data
- **WHEN** `cargo test` is run without Marathon trilogy data in `tests/fixtures/`
- **THEN** all Tier 1 (format parsing) tests SHALL print `SKIP: <reason>` to stderr and return success
- **AND** no Tier 1 test SHALL panic or fail due to missing fixture files

#### Scenario: Tier 1 tests run when data is present
- **WHEN** Marathon trilogy data has been fetched into `tests/fixtures/marathon-{1,2,infinity}/`
- **THEN** all Tier 1 tests SHALL execute, parsing WAD, Shapes, Sounds, and Physics files from each trilogy dataset
- **AND** each test SHALL assert golden values from `tests/scenarios.toml`

#### Scenario: Tier 2 tests exercise simulation determinism
- **WHEN** a golden level with `tier2` values is loaded into `marathon-sim`
- **THEN** the test harness SHALL run the specified number of simulation ticks with the named input script
- **AND** the resulting player position, velocity, and polygon index SHALL match golden values within epsilon

#### Scenario: Tier 2 determinism is verified by double-run
- **WHEN** the same golden level is simulated twice with identical inputs
- **THEN** the resulting physics state SHALL be bitwise-identical across both runs

#### Scenario: Tier 3 tests capture per-level screenshots
- **WHEN** a golden level with `tier3` values is loaded in the browser via Playwright
- **THEN** the test SHALL capture a screenshot after rendering settles
- **AND** the screenshot SHALL meet the level's specified coverage, color variety, and quadrant thresholds

#### Scenario: Tier 4 stubs are structurally present
- **WHEN** `cargo test` is run in the `marathon-integration` crate
- **THEN** Tier 4 (MML compatibility) test functions SHALL exist and print `SKIP: MML override engine not yet implemented`
- **AND** the test functions SHALL return success (not fail)

#### Scenario: Tier 5 stubs are structurally present
- **WHEN** `cargo test` is run in the `marathon-integration` crate
- **THEN** Tier 5 (Lua compatibility) test functions SHALL exist and print `SKIP: Lua scripting engine not yet implemented`
- **AND** the test functions SHALL return success (not fail)

### Requirement: Tier promotion criteria are documented
Each tier SHALL have documented criteria for when a subsystem is considered "graduated" to the next tier.

#### Scenario: Tier 0 to Tier 1 promotion
- **WHEN** a format parser passes all Tier 0 synthetic tests
- **THEN** it SHALL be eligible for Tier 1 testing against real scenario data
- **AND** Tier 1 golden values SHALL be added to `tests/scenarios.toml` for the relevant levels

#### Scenario: Tier 1 to Tier 2 promotion
- **WHEN** all Tier 1 format parsing tests pass for a scenario's data files
- **THEN** the level SHALL be eligible for Tier 2 simulation testing
- **AND** Tier 2 golden values (tick count, input script, expected state) SHALL be added to the level's manifest entry

#### Scenario: Tier 2 to Tier 3 promotion
- **WHEN** a level's simulation runs deterministically for the specified tick count
- **THEN** the level SHALL be eligible for Tier 3 render regression testing
- **AND** Tier 3 golden values (camera position, visual thresholds) SHALL be added to the level's manifest entry
