---
tags: [tier-1, game-loop, testing, tdd, e2e, ci, framework]
status: research-complete
created: 2026-04-12
---

# E2E TDD Framework: Scenarios as the Standard

## Philosophy

The Rust engine's correctness is defined by compatibility with real Marathon content. Instead of writing isolated unit tests that guess at behavior, we use actual scenario data as the ground truth. If the engine can load, parse, simulate, and render every level from the Marathon trilogy (and eventually major total conversions), it is correct.

This is Test-Driven Development at the system level: the scenarios are the specification.

## Architecture Overview

```
                    +---------------------+
                    |  Scenario Data      |
                    |  (downloaded at     |
                    |   test time)        |
                    +---------------------+
                             |
              +--------------+--------------+
              |              |              |
       +------v------+ +----v----+ +-------v-------+
       | Format Tests | | Sim     | | Render Tests  |
       | (Rust #[test])| Tests   | | (Playwright + |
       |              | | (Rust   | |  screenshot)  |
       |              | | #[test])| |               |
       +--------------+ +---------+ +---------------+
              |              |              |
              v              v              v
       Parse every     Tick N frames   Compare frames
       WAD entry       without crash   to golden images
       without error   & verify state
```

## Test Tiers

### Tier 0: Synthetic Tests (No Data Files Required)

Already implemented. These test the engine against hand-built test data.

**Current coverage:**
- `marathon-formats/tests/integration_tests.rs` -- WAD roundtrip with synthetic geometry
- `marathon-sim/tests/integration.rs` -- Sim tick with synthetic 2-polygon map
- `marathon-game/tests/integration.rs` -- Full pipeline with synthetic map + physics

**Value:** Fast, always-run, catch regressions in core logic. But they cannot verify real-world compatibility.

### Tier 1: Format Parsing Tests (Trilogy Data Required)

Test that every data format from the trilogy can be parsed without errors.

**Current coverage:**
- `marathon-formats/tests/real_data_tests.rs` -- WAD, Shapes, Sounds, Physics parsing with snapshot assertions
- `marathon-viewer/tests/e2e_tests.rs` -- Level enumeration, mesh generation, texture pipeline, light evaluation, platform data, full coherence check
- `marathon-game/tests/e2e_tests.rs` -- Full pipeline: load, mesh, sim init, tick, entity query

**Data source:** Marathon 2 data files in `marathon-formats/tests/fixtures/` (Map, Shapes, Sounds, Physics Model). Tests skip gracefully when files are absent.

**Proposed extensions:**

#### 1a. All-Level Parse Test
```rust
/// Parse EVERY level in the WAD, not just the first 5.
/// Verify zero panics and collect statistics.
#[test]
fn parse_all_levels_comprehensive() {
    let wad = WadFile::open(&fixture("Map")).unwrap();
    let mut stats = LevelStats::default();
    for i in 0..wad.entry_count() {
        let entry = wad.entry(i).unwrap();
        let map = MapData::from_entry(&entry)
            .unwrap_or_else(|e| panic!("level {i} parse failed: {e}"));
        stats.record(&map);
    }
    stats.assert_within_known_bounds();
}
```

#### 1b. Cross-Scenario Parse Test
```rust
/// Parse Map files from multiple scenarios.
/// Each scenario's data lives in a named subdirectory.
#[test]
fn parse_multi_scenario() {
    for scenario in &["marathon-2", "marathon-infinity", "marathon-1"] {
        let map_path = scenario_fixture(scenario, "Map");
        if map_path.is_none() { continue; }
        let wad = WadFile::open(&map_path.unwrap()).unwrap();
        for i in 0..wad.entry_count() {
            let entry = wad.entry(i).unwrap();
            let _ = MapData::from_entry(&entry)
                .unwrap_or_else(|e| panic!("{scenario} level {i}: {e}"));
        }
    }
}
```

#### 1c. Sound Reference Integrity Test
```rust
/// Verify every sound index referenced by physics data
/// exists in the Sounds file.
#[test]
fn sound_reference_integrity() {
    let sounds = SoundsFile::open(&fixture("Sounds")).unwrap();
    let physics = load_physics(&fixture("Physics Model"));
    for (i, monster) in physics.monsters.iter().enumerate() {
        for sound_idx in monster.sound_indices() {
            assert!(sounds.sound(sound_idx).is_ok(),
                "monster {i}: sound {sound_idx} not in Sounds file");
        }
    }
}
```

#### 1d. Texture Reference Integrity Test
```rust
/// Verify every ShapeDescriptor in map data resolves
/// to a valid bitmap in the Shapes file.
#[test]
fn texture_reference_integrity() {
    let wad = WadFile::open(&fixture("Map")).unwrap();
    let shapes = ShapesFile::open(&fixture("Shapes")).unwrap();
    for level_idx in 0..wad.entry_count() {
        let entry = wad.entry(level_idx).unwrap();
        let map = match MapData::from_entry(&entry) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let descriptors = collect_all_texture_descriptors(&map);
        for desc in &descriptors {
            if desc.is_none() { continue; }
            let coll = desc.collection() as usize;
            let bmp = desc.bitmap() as usize;
            let collection = shapes.collection(coll)
                .unwrap_or_else(|e| panic!("level {level_idx}: collection {coll}: {e}"));
            assert!(bmp < collection.bitmaps.len(),
                "level {level_idx}: bitmap {bmp} >= collection {coll} count {}",
                collection.bitmaps.len());
        }
    }
}
```

### Tier 2: Simulation Tests (Trilogy Data + Physics Required)

Test that the simulation engine can process real levels.

**Current coverage:**
- `marathon-game/tests/e2e_tests.rs` already tests sim init and 60-tick forward movement on level 0.

**Proposed extensions:**

#### 2a. All-Level Sim Stability Test
```rust
/// Initialize sim on EVERY level and tick 60 frames.
/// Zero panics = pass.
#[test]
fn sim_stability_all_levels() {
    let wad = WadFile::open(&fixture("Map")).unwrap();
    let physics = load_physics(&fixture("Physics Model"));
    let empty = ActionFlags::new(0);

    for i in 0..wad.entry_count() {
        let entry = wad.entry(i).unwrap();
        let map = match MapData::from_entry(&entry) {
            Ok(m) => m,
            Err(_) => continue,
        };
        // Skip levels without player starts
        if !map.objects.iter().any(|o| o.object_type == 3) { continue; }

        let config = SimConfig { random_seed: 42, difficulty: 2 };
        let mut world = SimWorld::new(&map, &physics, &config)
            .unwrap_or_else(|e| panic!("level {i} sim init: {e}"));

        for tick in 0..60 {
            world.tick(empty.into());
        }
        assert_eq!(world.tick_count(), 60,
            "level {i}: expected 60 ticks");
    }
}
```

#### 2b. Deterministic Replay Test
```rust
/// Run the same input sequence twice on the same level.
/// Verify the final state is bit-identical.
#[test]
fn sim_determinism() {
    let wad = WadFile::open(&fixture("Map")).unwrap();
    let physics = load_physics(&fixture("Physics Model"));

    let inputs = generate_test_input_sequence(120); // 4 seconds

    let run = |seed: u64| -> SimSnapshot {
        let entry = wad.entry(0).unwrap();
        let map = MapData::from_entry(&entry).unwrap();
        let config = SimConfig { random_seed: seed, difficulty: 2 };
        let mut world = SimWorld::new(&map, &physics, &config).unwrap();
        for input in &inputs {
            world.tick(*input);
        }
        world.snapshot()
    };

    let snap1 = run(42);
    let snap2 = run(42);
    assert_eq!(snap1, snap2, "sim must be deterministic");
}
```

#### 2c. Known-Position Physics Test
```rust
/// On Waterloo Waterpark, walk forward for 30 ticks.
/// Verify player position is within expected range.
/// Values calibrated against Aleph One reference.
#[test]
fn physics_position_golden() {
    // ... load level 0, tick 30 with MOVE_FORWARD ...
    let pos = world.player_position().unwrap();
    assert!((pos.x - EXPECTED_X).abs() < TOLERANCE);
    assert!((pos.y - EXPECTED_Y).abs() < TOLERANCE);
}
```

### Tier 3: Render Regression Tests (Browser/WASM Required)

Screenshot comparison against golden images at known camera positions.

**Current coverage:**
- `e2e/tests/visual-regression.spec.ts` -- Tests pixel coverage (>20% non-black), color variety (>50 unique colors), and multi-quadrant content.

**Proposed extensions:**

#### 3a. Per-Level Golden Screenshot Test
```typescript
// For each golden test level, navigate to that level,
// set camera to known position, capture screenshot,
// compare against stored golden image.
test.describe('Golden level screenshots', () => {
  const goldenLevels = [
    { scenario: 'm2', level: 0, name: 'waterloo-waterpark',
      camera: { x: 512, y: 512, z: 0, yaw: 0 } },
    { scenario: 'm2', level: 12, name: 'six-thousand-feet-under',
      camera: { x: 100, y: 200, z: -512, yaw: 90 } },
    // ... more from golden-test-levels.md
  ];

  for (const gl of goldenLevels) {
    test(`${gl.name} matches golden`, async ({ page }) => {
      await page.goto(`/?scenario=${gl.scenario}&level=${gl.level}`);
      await page.evaluate((cam) => {
        window.__engine.setCamera(cam.x, cam.y, cam.z, cam.yaw);
      }, gl.camera);
      await page.waitForTimeout(500); // let render settle
      const screenshot = await page.locator('#marathon-canvas').screenshot();
      expect(screenshot).toMatchSnapshot(`${gl.name}.png`, {
        maxDiffPixelRatio: 0.02,
      });
    });
  }
});
```

#### 3b. Texture Atlas Verification
```typescript
// Verify the texture atlas loads correctly by checking
// that rendered surfaces have the expected dominant colors.
test('waterloo waterpark has expected texture palette', async ({ page }) => {
  // Sample floor and wall pixels, verify they're in the
  // expected color range for Marathon 2's water textures.
});
```

### Tier 4: MML/Plugin Tests (Extended Data Required)

Test MML parsing and application with real scenario MML files.

**Current coverage:**
- `marathon-formats/tests/real_data_tests.rs` -- Tests Aleph One GPL MML files (Carnage_Messages, Transparent_Liquids, Transparent_Sprites, Marathon_2 scenario MML).

**Proposed extensions:**

#### 4a. Community Scenario MML Parsing
```rust
/// Download and parse MML from major scenarios.
/// Verify no parse errors on real-world MML.
#[test]
fn parse_community_mml() {
    for scenario_dir in discover_scenario_dirs() {
        for mml_path in glob(&scenario_dir, "**/*.mml") {
            MmlDocument::from_file(&mml_path)
                .unwrap_or_else(|e| panic!("{}: {e}", mml_path.display()));
        }
    }
}
```

#### 4b. Plugin.xml Parsing Across Scenarios
```rust
#[test]
fn parse_community_plugins() {
    for scenario_dir in discover_scenario_dirs() {
        for plugin_xml in glob(&scenario_dir, "**/Plugin.xml") {
            PluginMetadata::from_file(&plugin_xml)
                .unwrap_or_else(|e| panic!("{}: {e}", plugin_xml.display()));
        }
    }
}
```

### Tier 5: Lua Script Tests (Future)

Test Lua script execution against real scenario scripts.

```rust
/// Load Istoria's Lua scripts and verify they parse
/// without syntax errors. Execute init() functions.
#[test]
fn lua_script_parse_istoria() {
    let vm = LuaVm::new();
    for lua_path in glob(&scenario_dir("istoria"), "**/*.lua") {
        vm.load_file(&lua_path)
            .unwrap_or_else(|e| panic!("{}: {e}", lua_path.display()));
    }
    vm.call("init").unwrap();
}
```

## CI Pipeline Design

### Pipeline Architecture

```
CI Workflow: scenario-e2e
  |
  +-- Job: download-data
  |     Download trilogy data from GitHub releases
  |     Cache in CI artifact store
  |
  +-- Job: tier0-synthetic (parallel, no data)
  |     cargo test -p marathon-formats
  |     cargo test -p marathon-sim
  |     cargo test -p marathon-game
  |
  +-- Job: tier1-format (needs download-data)
  |     cargo test -p marathon-formats -- --ignored
  |     cargo test -p marathon-viewer -- --ignored
  |
  +-- Job: tier2-simulation (needs download-data)
  |     cargo test -p marathon-game -- --ignored
  |
  +-- Job: tier3-render (needs download-data + Docker)
  |     Build WASM, start web server
  |     npx playwright test e2e/
  |
  +-- Job: tier4-extended (manual trigger, community data)
        Download community scenarios
        Run cross-scenario parse tests
```

### Data Acquisition Script

```bash
#!/bin/bash
# scripts/download-test-data.sh
# Downloads Marathon trilogy data for testing.

FIXTURE_DIR="marathon-formats/tests/fixtures"
M2_URL="https://github.com/Aleph-One-Marathon/alephone/releases/download/release-20250829/Marathon2-20250829-Data.zip"
MINF_URL="https://github.com/Aleph-One-Marathon/alephone/releases/download/release-20250829/MarathonInfinity-20250829-Data.zip"

mkdir -p "$FIXTURE_DIR"

if [ ! -f "$FIXTURE_DIR/Map" ]; then
    echo "Downloading Marathon 2 data..."
    curl -L "$M2_URL" -o /tmp/m2-data.zip
    unzip -o /tmp/m2-data.zip -d /tmp/m2-data/
    # Copy data files to fixtures
    cp /tmp/m2-data/*/Map "$FIXTURE_DIR/Map" 2>/dev/null || \
    cp /tmp/m2-data/*/*/Map "$FIXTURE_DIR/Map" 2>/dev/null || true
    cp /tmp/m2-data/*/Shapes "$FIXTURE_DIR/Shapes" 2>/dev/null || \
    cp /tmp/m2-data/*/*/Shapes "$FIXTURE_DIR/Shapes" 2>/dev/null || true
    cp /tmp/m2-data/*/Sounds "$FIXTURE_DIR/Sounds" 2>/dev/null || \
    cp /tmp/m2-data/*/*/Sounds "$FIXTURE_DIR/Sounds" 2>/dev/null || true
    cp "/tmp/m2-data/*/Physics Model" "$FIXTURE_DIR/Physics Model" 2>/dev/null || \
    cp "/tmp/m2-data/*/*/Physics Model" "$FIXTURE_DIR/Physics Model" 2>/dev/null || true
    rm -rf /tmp/m2-data /tmp/m2-data.zip
fi

echo "Fixture files:"
ls -la "$FIXTURE_DIR/"
```

### Docker Build Integration

The existing Docker build setup (see [[project_build_setup]]) uses `rust:1.82-slim`. The CI pipeline extends this:

```dockerfile
# Dockerfile.test
FROM rust:1.82-slim AS builder
RUN apt-get update && apt-get install -y curl unzip
COPY scripts/download-test-data.sh /tmp/
RUN /tmp/download-test-data.sh
COPY . /app
WORKDIR /app
RUN cargo test --workspace
```

## Test Fixture Organization

### Current Structure
```
marathon-formats/tests/fixtures/
  Map              # Marathon 2 map (binary, .gitignored)
  Shapes           # Marathon 2 shapes (binary, .gitignored)
  Sounds           # Marathon 2 sounds (binary, .gitignored)
  Physics Model    # Marathon 2 physics (binary, .gitignored)
  sample.mml       # Synthetic MML (committed)
  sample_plugin/   # Synthetic Plugin.xml (committed)
  alephone/        # GPL Aleph One MML/Plugin.xml files (committed)
  .gitignore
  README.md
```

### Proposed Extended Structure
```
marathon-formats/tests/fixtures/
  marathon-2/        # Marathon 2 data (.gitignored)
    Map
    Shapes
    Sounds
    Physics Model
  marathon-infinity/  # Marathon Infinity data (.gitignored)
    Map.sceA
    Shapes.shpA
    Sounds.sndA
  marathon-1/         # Marathon 1 data (.gitignored)
    Map.scen
    Shapes.shps
    Sounds.sndz
    Physics.phys
  community/          # Community scenarios (.gitignored)
    rubicon-x/
    eternal-x/
    phoenix/
  synthetic/          # Committed test data
    sample.mml
    sample_plugin/
  alephone/           # GPL fixtures (committed)
    *.mml
    *_Plugin.xml
  Map                 # Symlink to marathon-2/Map (backward compat)
  Shapes              # Symlink to marathon-2/Shapes
```

## Snapshot / Golden Value Management

### Rust Snapshot Assertions

The existing tests use hard-coded snapshot values:
```rust
assert_eq!(wad.entry_count(), 41, "Marathon 2 should have 41 levels");
assert_eq!(map.endpoints.len(), 716, "level 0 endpoint count");
assert_eq!(map.lines.len(), 1106, "level 0 line count");
assert_eq!(map.polygons.len(), 369, "level 0 polygon count");
```

**Recommendation:** Maintain a `golden_values.json` file with known-good values for each scenario/level combination:

```json
{
  "marathon-2": {
    "entry_count": 41,
    "levels": {
      "0": {
        "name": "Waterloo Waterpark",
        "endpoints": 716,
        "lines": 1106,
        "polygons": 369,
        "sides": 642,
        "objects": 87,
        "platforms": 12
      }
    }
  }
}
```

### Playwright Screenshot Snapshots

Playwright's `toMatchSnapshot` stores golden images in a `__snapshots__` directory. These are committed to the repo and updated with `--update-snapshots` when rendering changes intentionally.

## Extending for Community Scenarios

### Phase 1: Trilogy Only (Current)
- Marathon 2 data in fixtures
- All existing tests pass

### Phase 2: All Three Trilogy Games
- Add Marathon 1 and Marathon Infinity data
- Cross-scenario parse tests
- Per-scenario snapshot values

### Phase 3: Major Total Conversions
- Download Rubicon X, Eternal X, Phoenix on demand
- Parse-only tests (no rendering required)
- MML and Plugin.xml parsing tests

### Phase 4: Lua Scenarios
- Download Istoria, Apotheosis X
- Lua syntax validation
- Lua API coverage tests

### Phase 5: Full Render Regression
- Golden screenshots for every golden test level
- Per-scenario render regression suite
- A/B comparison against Aleph One reference renders

## Existing Test Cross-Reference

| Test File | Tier | What It Tests | Data Required |
|-----------|------|---------------|---------------|
| `marathon-formats/tests/integration_tests.rs` | 0 | WAD roundtrip with synthetic data | None |
| `marathon-formats/tests/real_data_tests.rs` | 1 | WAD, Shapes, Sounds, Physics, MML, Plugin.xml | M2 data + GPL MML |
| `marathon-viewer/tests/e2e_tests.rs` | 1 | Level enum, mesh gen, texture pipeline, lights, platforms, coherence | M2 Map + Shapes |
| `marathon-game/tests/e2e_tests.rs` | 1+2 | Full pipeline: load, mesh, sim, tick, entities, snapshot | M2 Map + Shapes + Physics |
| `marathon-game/tests/integration.rs` | 0 | Sim init, tick, action flags, entities, events (synthetic) | None |
| `marathon-sim/tests/integration.rs` | 0 | Sim with synthetic 2-polygon map | None |
| `marathon-web/tests/wasm.rs` | 0 | WASM compilation and basic API | None |
| `e2e/tests/wasm-init.spec.ts` | 3 | WASM init in browser | Web server + M2 data |
| `e2e/tests/data-fetch.spec.ts` | 3 | Data file fetching | Web server + M2 data |
| `e2e/tests/game-start.spec.ts` | 3 | Game startup flow | Web server + M2 data |
| `e2e/tests/ui-elements.spec.ts` | 3 | UI element rendering | Web server + M2 data |
| `e2e/tests/webgl-compat.spec.ts` | 3 | WebGL2 compatibility | Web server |
| `e2e/tests/visual-regression.spec.ts` | 3 | Pixel coverage, color variety, quadrant coverage | Web server + M2 data |
| `e2e/tests/interaction.spec.ts` | 3 | Browser interaction (keyboard/mouse) | Web server + M2 data |
| `e2e/tests/error-handling.spec.ts` | 3 | Error handling in browser | Web server |

## Related Notes

- [[scenario-feature-matrix]] -- Which scenarios test which features
- [[golden-test-levels]] -- The specific ~20 levels chosen as the compatibility suite
- [[community-content-ecosystem]] -- Where scenarios come from
- [[project_build_setup]] -- Docker build configuration
- [[project_deployment]] -- Docker deployment and Playwright setup
