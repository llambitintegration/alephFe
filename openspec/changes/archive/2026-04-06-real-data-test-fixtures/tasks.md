## 1. Committed GPL Fixtures

- [x] 1.1 Copy Aleph One engine MML files into `tests/fixtures/alephone/`: `Carnage_Messages.mml`, `Transparent_Liquids.mml`, `Transparent_Sprites.mml`, `Marathon_2.mml` (from Scripts/)
- [x] 1.2 Copy Aleph One Plugin.xml files into `tests/fixtures/alephone/`: `default_theme_Plugin.xml`, `BasicHUD_Plugin.xml`, `EnhancedHUD_Plugin.xml`, `Stats_Plugin.xml`, `TransparentLiquids_Plugin.xml`
- [x] 1.3 Update `tests/fixtures/.gitignore` to ensure `alephone/` directory is tracked (not ignored)

## 2. GPL Fixture Tests

- [x] 2.1 Add MML tests for GPL fixtures: parse each committed MML file and assert expected sections are present (console, opengl, interface, scenario)
- [x] 2.2 Add Plugin.xml tests for GPL fixtures: parse each committed Plugin.xml and assert expected attributes (hud_lua, stats_lua, theme_dir, multi-scenario)
- [x] 2.3 Add MML layering test using a real Aleph One MML file as overlay on synthetic base

## 3. CI Data Fetch Infrastructure

- [x] 3.1 Identify the current commit hash of the Marathon 2 data repo to pin in CI
- [x] 3.2 Add `fetch-data` stage to Dockerfile that clones Marathon 2 data repo at pinned commit and copies Map.sceA, Shapes.shpA, Sounds.sndA, and Standard.phyA into test fixtures
- [x] 3.3 Rebase the `test` and `coverage` Docker stages onto `fetch-data` so real data is available during test runs

## 4. Snapshot Assertions

- [x] 4.1 Run parsers against fetched Marathon 2 data locally (via Docker) to capture known-good values: level 0 endpoint/line/polygon counts, collection header counts, physics constant count
- [x] 4.2 Add snapshot assertions to `test_map_geometry_parsing`: assert specific endpoint, line, and polygon counts for level 0
- [x] 4.3 Add snapshot assertions to `test_shapes_parsing`: assert collection count with data and bitmap dimensions for a known collection
- [x] 4.4 Add snapshot assertions to `test_physics_parsing`: assert physics constant entry count and a known velocity value

## 5. Verification

- [x] 5.1 Run full test suite via Docker (`docker build --target test .`) and verify all real-data tests pass (no SKIPs in CI output)
- [x] 5.2 Verify tests still pass locally without fetched data (graceful skip behavior)
- [x] 5.3 Run coverage stage and confirm coverage increased from the new test paths
