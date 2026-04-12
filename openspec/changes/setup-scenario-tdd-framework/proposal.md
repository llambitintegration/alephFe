## Why

The engine's correctness standard is simple: if it can run real Marathon scenarios, it's compatible. We already have 14 Rust test files across 7 crates and 8 Playwright specs, but these tests are either synthetic unit tests or single-scenario smoke tests against Marathon 2 data. There is no structured framework that treats community scenarios as the definitive test corpus, no simulation determinism checks, no per-level golden values, and no way to test MML/Lua plugin compatibility. Meanwhile, the original Marathon trilogy data is freely redistributable under Bungie's 2005/2021 license with stable GitHub download URLs (Aleph-One-Marathon/data-marathon, data-marathon-2, data-marathon-infinity), giving us a legal, reproducible foundation for scenario-driven testing. Establishing a tiered TDD framework now — while the engine is still being built — means every new subsystem (platforms, liquids, combat, terminals, Lua scripting) ships with scenario-level acceptance criteria from day one, not retroactive test bolting.

## What Changes

- **Define a 6-tier test framework** (Tier 0-5) that progresses from synthetic unit tests through format parsing, simulation determinism, render regression, MML compatibility, and Lua script compatibility
- **Curate 22 golden test levels** across the Marathon trilogy and community total conversions, selected for maximum coverage of engine features: geometry edge cases, water/liquid rendering, platform mechanics, combat AI, terminal sequences, WAD version differences, dream-sequence architecture, MML overrides, Lua scripting, HUD Lua, and texture replacements
- **Add Tier 1 infrastructure**: Expand real-data parsing tests beyond Marathon 2 to include Marathon 1 (WAD v2) and Marathon Infinity (WAD v4) scenario data, with per-level snapshot assertions (endpoint counts, polygon counts, line counts, physics constants) for all 22 golden levels
- **Add Tier 2 infrastructure**: A simulation stability harness that loads each golden level, ticks N frames without crash, and compares physics state (player position, velocity, monster positions) against golden values for determinism verification
- **Add Tier 3 infrastructure**: Extend the existing Playwright visual regression suite to capture screenshots at known camera positions in golden levels, with per-pixel-region thresholds rather than just "non-blank" checks
- **Add Tier 4/5 stubs**: Test harness entry points for MML override verification and Lua script execution, blocked on those engine subsystems but structurally ready
- **Add a CI data-fetch stage** that pulls all three Marathon trilogy datasets (Marathon 1: ~30 MB, Marathon 2: ~50 MB, Infinity: ~60 MB) at pinned commits before running tier 1+ tests
- **Add a `scenario-fixtures` manifest** (`tests/scenarios.toml`) that declares each golden level's source repo, commit hash, WAD path, level index, and expected golden values — making the test corpus declarative, versionable, and extensible by community contributors

## Capabilities

### New Capabilities

- `scenario-test-tiers`: A 6-tier test taxonomy (Tier 0: synthetic unit, Tier 1: format parsing against real data, Tier 2: simulation determinism, Tier 3: render regression, Tier 4: MML compatibility, Tier 5: Lua compatibility) with clear promotion criteria between tiers and per-tier CI gates
- `golden-level-corpus`: A curated set of 22 levels across Marathon 1 (3 levels: WAD v2, platform puzzles, vacuum mechanics), Marathon 2 (10 levels: geometry, water, platforms, combat, terminals, net levels), Marathon Infinity (4 levels: WAD v4, dream levels, complex architecture), and total conversions (5 levels: MML overrides, Lua scripting, HUD Lua, texture replacements) — each with documented feature coverage and expected golden values
- `simulation-determinism-harness`: A Rust test harness that loads a level from real scenario data, runs N simulation ticks with scripted inputs, and asserts that the resulting physics state matches golden values — catching non-determinism, floating-point divergence, and simulation regressions
- `scenario-fixtures-manifest`: A declarative TOML manifest (`tests/scenarios.toml`) listing each golden level's data source, commit pin, file paths, level index, and per-tier expected values — enabling community contributors to add new golden levels without modifying test code

### Modified Capabilities

- `real-data-testing`: Expand from Marathon 2-only to all three trilogy datasets (Marathon 1, Marathon 2, Marathon Infinity) with per-level golden value assertions for all 22 selected levels; CI data-fetch stage pulls all three repos at pinned commits
- `visual-regression-baseline`: Extend from single-scenario "non-blank canvas" checks to per-level screenshot comparisons at known camera positions across the golden level corpus, with quadrant-level and color-distribution thresholds specific to each level's expected visual content

## Impact

- **New files**: `tests/scenarios.toml` (golden level manifest), `tests/scenario_harness.rs` or `marathon-sim/tests/determinism.rs` (Tier 2 harness), new Playwright specs per golden level group (Tier 3)
- **Modified files**: `Dockerfile` and `Dockerfile.e2e` (fetch Marathon 1 and Infinity data alongside Marathon 2), `marathon-formats/tests/real_data_tests.rs` (Tier 1 expansion for M1/Infinity levels), `e2e/tests/visual-regression.spec.ts` (Tier 3 per-level screenshots)
- **CI**: Data-fetch stage triples in scope (3 repos instead of 1, ~140 MB total); new CI jobs for Tier 2 determinism checks; Tier 4/5 jobs added as no-op placeholders
- **Data dependencies**: Marathon 1 data from `Aleph-One-Marathon/data-marathon` and Infinity data from `Aleph-One-Marathon/data-marathon-infinity` added as CI-fetched fixtures (never committed); total conversion test data requires separate sourcing strategy (community repos or manual fixtures)
- **Crates affected**: `marathon-formats` (Tier 1 test expansion), `marathon-sim` (Tier 2 determinism harness), `marathon-web`/`marathon-game` (Tier 3 render targets), `marathon-integration` (potential Tier 4/5 harness host)
- **Developer workflow**: New levels are added by appending entries to `tests/scenarios.toml` with expected golden values; CI enforces that all declared golden levels pass their tier-appropriate tests before merge
