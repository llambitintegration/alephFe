## 1. Manifest Setup

- [ ] 1.1 Create `tests/scenarios.toml` with `[sources]` table declaring Marathon 1 (`data-marathon`), Marathon 2 (`data-marathon-2` at `eaf21a7`), and Marathon Infinity (`data-marathon-infinity`) repos with pinned commit hashes
- [ ] 1.2 Add `[[levels]]` entries for 3 Marathon 1 golden levels: Arrival (level 0), Bob-B-Q, Colony Ship -- with `id`, `source`, `wad_path`, `level_index`, `name`, and `features` fields
- [ ] 1.3 Add `[[levels]]` entries for 10 Marathon 2 golden levels: Waterloo Waterpark (level 0) through Fatum Iustum Stultorum -- with `id`, `source`, `wad_path`, `wad_strip_macbinary`, `level_index`, `name`, and `features` fields
- [ ] 1.4 Add `[[levels]]` entries for 4 Marathon Infinity golden levels: Ne Cede Malis (level 0), A Converted Church, Aye Mak Sicur, Hang Brain -- with `id`, `source`, `wad_path`, `level_index`, `name`, and `features` fields
- [ ] 1.5 Add 5 deferred `[[levels]]` entries for total conversion placeholders with `deferred = true` and comments explaining blocking dependencies
- [ ] 1.6 Add `[levels.tier1]` sub-tables with golden geometry counts (endpoints, lines, polygons) for Marathon 2 Waterloo Waterpark (716, 1106, 369 from existing tests)
- [ ] 1.7 Add a Rust module `tests/scenario_manifest.rs` (or in-crate helper) that parses `tests/scenarios.toml` into typed structs using `toml` + `serde`, exposing `load_manifest()`, `levels_for_tier()`, and `source_path()` functions
- [ ] 1.8 Add `toml` and `serde` dev-dependencies to `marathon-formats/Cargo.toml` and `marathon-sim/Cargo.toml` for manifest parsing in tests

## 2. Data Acquisition

- [ ] 2.1 Pin Marathon 1 commit: clone `data-marathon` repo, identify a stable commit SHA, and record it in `tests/scenarios.toml` under `[sources.marathon-1]`
- [ ] 2.2 Pin Marathon Infinity commit: clone `data-marathon-infinity` repo, identify a stable commit SHA, and record it in `tests/scenarios.toml` under `[sources.marathon-infinity]`
- [ ] 2.3 Update `Dockerfile` `fetch-data` stage to clone `data-marathon` at pinned commit, copy Map/Shapes/Sounds/Physics to `tests/fixtures/marathon-1/`
- [ ] 2.4 Update `Dockerfile` `fetch-data` stage to clone `data-marathon-infinity` at pinned commit, copy Map/Shapes/Sounds/Physics to `tests/fixtures/marathon-infinity/`
- [ ] 2.5 Update `Dockerfile` `fetch-data` stage to also copy Marathon 2 files to `tests/fixtures/marathon-2/` (in addition to existing `tests/fixtures/` paths for backward compat)
- [ ] 2.6 Update `Dockerfile.e2e` `fetch-data` stage to clone Marathon 1 and Infinity repos alongside Marathon 2, making data available at `/data/marathon-1/` and `/data/marathon-infinity/`
- [ ] 2.7 Add `tests/fixtures/marathon-1/`, `tests/fixtures/marathon-2/`, and `tests/fixtures/marathon-infinity/` to `.gitignore`
- [ ] 2.8 Create `tests/fixtures/marathon-2/` as a migration alias: update existing test code path references from `tests/fixtures/Map` to `tests/fixtures/marathon-2/Map` with fallback to old path

## 3. Tier 1 -- Format Parsing Expansion

- [ ] 3.1 Refactor `marathon-formats/tests/real_data_tests.rs`: extract a `fixture_for_source(source_name, file_name)` helper that resolves paths via `tests/scenarios.toml` source entries with fallback to legacy paths
- [ ] 3.2 Add Marathon 1 WAD parsing test: load `tests/fixtures/marathon-1/Map`, assert WAD version 0 or 1, assert entry count > 0, assert endpoint/line/polygon data is present
- [ ] 3.3 Add Marathon Infinity WAD parsing test: load `tests/fixtures/marathon-infinity/Map`, assert WAD version 4, assert entry count > 0
- [ ] 3.4 Add parameterized Tier 1 golden value test: iterate all levels in manifest with `tier1` values, parse map data, assert endpoints/lines/polygons match golden counts
- [ ] 3.5 Populate `[levels.tier1]` golden values for Marathon 1 and Infinity levels by running parsing tests against fetched data and recording actual counts
- [ ] 3.6 Add Marathon 1 Shapes/Sounds/Physics parsing tests (using same patterns as existing Marathon 2 tests)
- [ ] 3.7 Add Marathon Infinity Shapes/Sounds/Physics parsing tests
- [ ] 3.8 Verify all Tier 1 tests pass in Docker: `docker build --target test .`

## 4. Tier 2 -- Simulation Determinism

- [ ] 4.1 Create `marathon-sim/tests/determinism.rs` with test harness that reads `tests/scenarios.toml` and iterates levels with `tier2` golden values
- [ ] 4.2 Implement input script functions: `idle_script(n)` returns `Vec<ActionFlags>` with no bits set, `walk_forward_script(n)` with forward bit set, `strafe_left_script(n)` with left-strafe bit set
- [ ] 4.3 Implement `load_level_sim(level_entry)` helper: parses WAD via `marathon-formats`, builds `MapData`, constructs `SimWorld` with `SimConfig`
- [ ] 4.4 Implement determinism double-run test: for each tier2 level, run simulation twice with identical inputs and assert bitwise-identical player state
- [ ] 4.5 Implement golden value assertion test: for each tier2 level, run simulation for `tick_count` ticks with `input_script`, assert player position/polygon match within epsilon
- [ ] 4.6 Add `[levels.tier2]` golden values for Marathon 2 Waterloo Waterpark: run simulation, record player spawn position after idle ticks, record as baseline
- [ ] 4.7 Verify Tier 2 tests pass in Docker: `docker build --target test .`

## 5. Tier 3 -- Render Regression Expansion

- [ ] 5.1 Create `e2e/tests/scenario-visual-regression.spec.ts` that reads golden level tier3 values and generates per-level Playwright test cases
- [ ] 5.2 Implement level-loading mechanism: define URL parameter or WASM API call that tells the game to load a specific scenario/level index instead of the default
- [ ] 5.3 Implement per-level camera positioning: set camera yaw/pitch per tier3 golden values before screenshot capture
- [ ] 5.4 Implement per-level threshold assertions: use manifest `min_coverage`, `min_unique_colors`, `min_quadrants` instead of hardcoded values
- [ ] 5.5 Add `[levels.tier3]` golden values for Marathon 2 Waterloo Waterpark: camera_yaw=0, camera_pitch=0, thresholds from existing visual regression baseline
- [ ] 5.6 Verify existing `visual-regression.spec.ts` tests continue to pass unchanged alongside new per-level tests
- [ ] 5.7 Verify Tier 3 tests pass in Docker: `docker compose -f docker-compose.e2e.yml up --abort-on-container-exit`

## 6. Tier 4/5 -- MML and Lua Stubs

- [ ] 6.1 Create `marathon-integration/tests/mml_compat.rs` with stub test functions that print `SKIP: MML override engine not yet implemented` and return success
- [ ] 6.2 Create `marathon-integration/tests/lua_compat.rs` with stub test functions that print `SKIP: Lua scripting engine not yet implemented` and return success
- [ ] 6.3 Add structural skeleton to `mml_compat.rs`: function signatures for loading a scenario, applying MML overrides, and asserting modified physics/weapon/monster values
- [ ] 6.4 Add structural skeleton to `lua_compat.rs`: function signatures for loading a scenario, initializing Lua VM, executing a script, and asserting state changes
- [ ] 6.5 Verify stub tests pass in Docker: `docker build --target test .`

## 7. CI Integration

- [ ] 7.1 Verify `Dockerfile` builds successfully with expanded fetch-data stage (all three repos)
- [ ] 7.2 Verify `docker build --target test .` runs all Tier 0, Tier 1, and Tier 2 tests green
- [ ] 7.3 Verify `docker build --target clippy .` passes with new test files
- [ ] 7.4 Verify `docker build --target fmt .` passes with new test files
- [ ] 7.5 Verify `docker compose -f docker-compose.e2e.yml up --abort-on-container-exit` runs Tier 3 tests green
- [ ] 7.6 Confirm that Tier 4/5 stub tests appear in test output as SKIP (not as failures)
- [ ] 7.7 Confirm that running `cargo test` locally without fetched data produces graceful SKIPs for all Tier 1-5 tests and green results for Tier 0
