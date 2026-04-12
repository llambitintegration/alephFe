## Context

The engine currently has 14 Rust test files across 7 crates and 8 Playwright e2e specs, but all real-data testing targets a single scenario (Marathon 2) with no structured tier progression. There is no simulation determinism checking, no per-level golden values, no multi-scenario visual regression, and no harness ready for MML/Lua plugin compatibility testing. The Marathon trilogy data is freely redistributable under Bungie's 2005/2021 license from stable GitHub URLs (`Aleph-One-Marathon/data-marathon`, `data-marathon-2`, `data-marathon-infinity`).

This change establishes a 6-tier TDD framework that treats community scenarios as the definitive test corpus, with a declarative manifest, CI data acquisition, and per-tier test harnesses.

## Goals / Non-Goals

**Goals:**
- A 6-tier test taxonomy with clear promotion criteria between tiers
- A declarative TOML manifest (`tests/scenarios.toml`) for golden level corpus management
- CI pipeline that fetches all three trilogy datasets at pinned commits
- Tier 1 (format parsing) expanded to Marathon 1 and Infinity alongside Marathon 2
- Tier 2 (simulation determinism) harness that loads real levels, ticks N frames, and asserts golden physics state
- Tier 3 (render regression) expansion to per-level screenshots at known camera positions
- Tier 4/5 stub harnesses structurally ready for MML and Lua compatibility
- Community-extensible: add levels by appending to `scenarios.toml`, not modifying test code

**Non-Goals:**
- Implementing MML override engine (Tier 4 blocked on that subsystem)
- Implementing Lua scripting engine (Tier 5 blocked on that subsystem)
- Total conversion data sourcing (requires separate community licensing strategy)
- Performance benchmarking or load testing
- Gameplay balance testing

## Decisions

### 1. Six-Tier Test Taxonomy

**Tier 0 -- Synthetic Unit Tests**: Existing unit tests with synthetic/handcrafted data. No external dependencies. Already implemented across all crates.

**Tier 1 -- Format Parsing Against Real Data**: Parse real scenario files (WAD maps, Shapes, Sounds, Physics) and assert golden values (endpoint counts, polygon counts, physics constants). Tests skip gracefully when data is absent locally.

**Tier 2 -- Simulation Determinism**: Load a real level into `marathon-sim`, tick N frames with scripted inputs (e.g., no-op standing, walk-forward), and assert that resulting physics state (player position, velocity, polygon index) matches golden values. Catches floating-point divergence and non-determinism.

**Tier 3 -- Render Regression**: Extend Playwright visual regression suite to capture screenshots at known camera positions in golden levels, with per-level thresholds (quadrant coverage, color distribution, dominant color ranges) rather than generic "non-blank" checks.

**Tier 4 -- MML Compatibility** (stub): Test harness entry points that will load a scenario with MML overrides and assert that weapon/monster/physics parameters match expected modified values. Blocked on MML override application in the engine.

**Tier 5 -- Lua Compatibility** (stub): Test harness entry points that will load a scenario with Lua scripts, tick frames, and assert script-driven state changes. Blocked on Lua scripting integration.

**Why this hierarchy**: Each tier depends on the capabilities of the tier below it. Format parsing must work before simulation can load levels. Simulation must work before render output can be validated. MML/Lua modify runtime behavior and thus sit at the top. This gives clear promotion criteria: a subsystem graduates to the next tier when the lower tier is green.

### 2. Declarative `scenarios.toml` Manifest

**Choice**: A single TOML file at `tests/scenarios.toml` that declares every golden level.

**Format**:
```toml
[sources.marathon-1]
repo = "https://github.com/Aleph-One-Marathon/data-marathon.git"
commit = "<pinned-sha>"
local_dir = "tests/fixtures/marathon-1"

[sources.marathon-2]
repo = "https://github.com/Aleph-One-Marathon/data-marathon-2.git"
commit = "eaf21a7e9f72706c4c2ff9a2960c4367f739f04d"
local_dir = "tests/fixtures/marathon-2"

[sources.marathon-infinity]
repo = "https://github.com/Aleph-One-Marathon/data-marathon-infinity.git"
commit = "<pinned-sha>"
local_dir = "tests/fixtures/marathon-infinity"

[[levels]]
id = "m2-waterloo-waterpark"
source = "marathon-2"
wad_path = "Map.sceA"
wad_strip_macbinary = true
level_index = 0
name = "Waterloo Waterpark"
features = ["water", "basic-geometry", "starting-level"]

[levels.tier1]
endpoints = 716
lines = 1106
polygons = 369

[levels.tier2]
tick_count = 60
input_script = "idle"
player_x = 0.0
player_y = 0.0
player_polygon = 0

[levels.tier3]
camera_yaw = 0.0
camera_pitch = 0.0
min_coverage = 0.20
min_unique_colors = 50
min_quadrants = 3
```

**Why TOML**: Rust ecosystem standard (Cargo.toml, config files). Serde support is trivial. Human-readable and diff-friendly. Community contributors can add levels without touching test code.

**Why a single file**: All golden levels in one place makes it easy to see coverage gaps and ensures consistent schema. Individual test files reference levels by `id`.

### 3. CI Data Acquisition Pipeline

**Choice**: Expand the existing `fetch-data` stage in `Dockerfile` and `Dockerfile.e2e` to clone all three trilogy repos at pinned commits.

**Pipeline**:
1. Clone `data-marathon` (Marathon 1, ~30 MB) at pinned commit
2. Clone `data-marathon-2` (Marathon 2, ~50 MB) at pinned commit (existing)
3. Clone `data-marathon-infinity` (Marathon Infinity, ~60 MB) at pinned commit
4. Copy relevant files to `tests/fixtures/{marathon-1,marathon-2,marathon-infinity}/`
5. Strip MacBinary headers where needed (Marathon 2 Map.sceA uses 128-byte header)

**Total CI data footprint**: ~140 MB fetched, ~100 MB after extraction (temporary clones deleted).

**Why pinned commits**: Reproducible builds. The Marathon data repos are occasionally updated with metadata fixes. Pinning ensures test golden values remain stable.

**Why not committed to repo**: Game data files are 30-60 MB each. Git LFS adds complexity. The data is freely available from canonical sources. CI-fetch is the established pattern (already used for Marathon 2).

### 4. Golden Level Selection Criteria

Levels are selected to maximize engine feature coverage across the minimum set. Selection criteria:

- **WAD version coverage**: At least one level per WAD version (v0/v1 Marathon 1, v2 Marathon 2, v4 Marathon Infinity)
- **Geometry complexity**: Simple (rectangular rooms), medium (multi-polygon corridors), complex (overlapping architecture, 5D space)
- **Feature-specific**: Each level exercises at least one distinct engine feature (water/liquid, platforms, elevators, vacuum, teleporters, terminals, monster closets, light effects, media)
- **Boundary conditions**: Levels known to stress-test parsers (maximum polygon counts, unusual line flags, degenerate geometry)
- **Progression**: Early levels (simple, guaranteed to parse) through late levels (complex, exercises advanced subsystems)

Initial corpus of 22 levels:
- Marathon 1 (3): Arrival (starting, simple geometry), Bob-B-Q (vacuum mechanics), Colony Ship (platform puzzles)
- Marathon 2 (10): Waterloo Waterpark (water, starting), What About Bob? (combat, monsters), The Slings & Arrows of Outrageous Fortune (complex geometry), Eat It Vid Boi (terminals), Come and Take Your Medicine (platforms), My Own Private Thermopylae (large open area), All Roads Lead to Sol (5D geometry), Rise Robot Rise (elevators), If I Had a Rocket Launcher I'd Make Somebody Pay (projectile paths), Fatum Iustum Stultorum (net level, different structure)
- Marathon Infinity (4): Ne Cede Malis (starting, WAD v4), A Converted Church (dream architecture), Aye Mak Sicur (complex geometry), Hang Brain (liquid heavy)
- Total Conversions (5 stubs): Reserved for MML/Lua testing once sourcing is resolved

### 5. Test Harness Architecture Per Tier

**Tier 1 -- `marathon-formats/tests/real_data_tests.rs` (expanded)**:
- Reads `tests/scenarios.toml` to enumerate golden levels
- For each level with `tier1` golden values: parse WAD, extract map data, assert counts
- Uses the existing `fixture()` helper pattern with graceful skip
- New: parameterized over all three trilogy datasets, not just Marathon 2

**Tier 2 -- `marathon-sim/tests/determinism.rs` (new)**:
- Reads `tests/scenarios.toml` for levels with `tier2` golden values
- For each: loads WAD via `marathon-formats`, constructs `SimWorld` via `marathon-sim`
- Runs `tick_count` simulation ticks with the named `input_script` (idle, walk-forward, strafe-left, etc.)
- Asserts player position, velocity, and polygon index against golden values within epsilon
- Runs the same simulation twice and asserts bitwise-identical state (determinism check)
- Input scripts are defined as functions returning `Vec<ActionFlags>` sequences

**Tier 3 -- `e2e/tests/scenario-visual-*.spec.ts` (new Playwright specs)**:
- For each golden level with `tier3` values: navigates to the game, loads the specified level
- Captures screenshot after rendering settles at the specified camera position
- Asserts per-level thresholds: minimum pixel coverage, minimum unique colors, minimum quadrant coverage
- Extends existing `sampleCanvasPixels` helper with level-specific camera setup

**Tier 4 -- `marathon-integration/tests/mml_compat.rs` (stub)**:
- Reads `tests/scenarios.toml` for levels with `tier4` entries
- Stub function that logs "SKIP: MML override engine not yet implemented"
- Structural skeleton: load scenario, apply MML overrides, assert modified values

**Tier 5 -- `marathon-integration/tests/lua_compat.rs` (stub)**:
- Reads `tests/scenarios.toml` for levels with `tier5` entries
- Stub function that logs "SKIP: Lua scripting engine not yet implemented"
- Structural skeleton: load scenario, execute Lua script, assert state changes

### 6. Docker Integration

**Dockerfile changes**:
- `fetch-data` stage expanded to clone all three repos
- Marathon 1 data: `data-marathon` -> `tests/fixtures/marathon-1/`
- Marathon Infinity data: `data-marathon-infinity` -> `tests/fixtures/marathon-infinity/`
- Marathon 2 data: existing path unchanged (`tests/fixtures/marathon-2/` aliased from current `tests/fixtures/`)

**Dockerfile.e2e changes**:
- Same three-repo fetch for web e2e data
- Level selection for Playwright tests may differ (only levels with tier3 golden values)

**docker-compose.e2e.yml**:
- No structural changes, but data volume mounts expanded for multi-scenario data

### 7. Fixture Directory Structure

```
tests/
  scenarios.toml                    # Golden level manifest
  fixtures/
    README.md                       # Existing: instructions for obtaining data
    alephone/                       # Existing: GPL MML/Plugin.xml fixtures
    sample.mml                      # Existing
    sample_plugin/                  # Existing
    marathon-1/                     # NEW: CI-fetched, gitignored
      Map                           # Marathon 1 WAD (v0/v1)
      Shapes
      Sounds
      Physics Model
    marathon-2/                     # Refactored: currently files live at fixtures/ root
      Map                           # Marathon 2 WAD (v2), MacBinary stripped
      Shapes
      Sounds
      Physics Model
    marathon-infinity/              # NEW: CI-fetched, gitignored
      Map                           # Marathon Infinity WAD (v4)
      Shapes
      Sounds
      Physics Model
    input-scripts/                  # NEW: Tier 2 input sequence definitions
      idle.json                     # No input for N ticks
      walk-forward.json             # W key held for N ticks
      strafe-left.json              # A key held for N ticks
```

The `marathon-1/`, `marathon-2/`, and `marathon-infinity/` directories are gitignored and populated only during CI or manual developer setup. The `scenarios.toml` and `input-scripts/` are committed.

**Migration note**: Current Marathon 2 fixtures live directly in `tests/fixtures/` (Map, Shapes, Sounds, Physics Model). These will be relocated to `tests/fixtures/marathon-2/` with the existing test code updated to use the new paths via `scenarios.toml` lookups. A backward-compatible symlink or fallback path check will ensure existing tests continue to work during the transition.

## Risks / Trade-offs

- **[CI time increase]** Fetching 3 repos (~140 MB) instead of 1 (~50 MB) adds ~1-2 min to CI. Mitigate with Docker layer caching and parallel clones.
- **[Golden value brittleness]** Hardcoded expected values break if data repos change. Mitigate with pinned commit hashes.
- **[Tier 2 floating-point sensitivity]** Simulation determinism may fail across different CPU architectures or Rust compiler versions. Mitigate with epsilon-based comparisons and documenting the reference platform (x86-64, rust:1.82-slim).
- **[Total conversion data sourcing]** The 5 TC golden levels require data from community repos with varying licenses. Mitigate by deferring TC levels to a future change and using only trilogy data initially.
- **[Marathon 1 WAD format differences]** Marathon 1 uses WAD v0/v1 which may have different tag structures than v2/v4. The `marathon-formats` parser may need updates to handle these. Discovery will happen during Tier 1 implementation.
- **[Fixture directory migration]** Moving Marathon 2 files from `tests/fixtures/` to `tests/fixtures/marathon-2/` requires updating existing tests. Mitigate with a fallback path pattern during transition.
