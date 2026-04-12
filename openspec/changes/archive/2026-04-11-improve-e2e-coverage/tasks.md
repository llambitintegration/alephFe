## 1. Browser Interaction Tests (Playwright)

- [x] 1.1 Create `e2e/tests/interaction.spec.ts` with test for WASD keyboard input: start game, press W/A/S/D keys, verify no console errors and game continues running
- [x] 1.2 Add Space key action test to `interaction.spec.ts`: press Space after game start, verify no crash
- [x] 1.3 Add pointer lock test to `interaction.spec.ts`: click canvas, verify `document.pointerLockElement` equals the canvas (or `pointerlockchange` event fires)
- [x] 1.4 Add HUD visibility test to `interaction.spec.ts`: after game runs for 2+ seconds, verify `#hud` is visible and `#health-val` / `#shield-val` contain numeric values

## 2. Missing Physics Error Test

- [x] 2.1 Add Physics 404 test to `e2e/tests/error-handling.spec.ts`: intercept `/data/Physics.phyA` with 404, verify `#error` element becomes visible and contains "Physics"

## 3. WASM API Tests (marathon-web)

- [x] 3.1 Add `wasm-bindgen-test` dev-dependency to `marathon-web/Cargo.toml`
- [x] 3.2 Create `marathon-web/tests/wasm.rs` with `wasm_bindgen_test` harness setup
- [x] 3.3 Add `level::enumerate_levels` test: construct a WAD from bytes (using `WadFile::from_bytes` with test helpers), verify returned `LevelInfo` vec has entries with non-empty names
- [x] 3.4 Add `level::load_level` success test: load index 0 from test WAD, verify `LoadedLevel` has non-empty `map.polygons`
- [x] 3.5 Add `level::load_level` error test: load index 9999, verify `Err` contains "out of range"
- [x] 3.6 Add `texture::pad_layer_count_for_webgl` tests: verify returns >= 2 for input 1, returns 7 for input 6, returns 13 for input 12, passes 5 through unchanged
- [x] 3.7 Add `mesh::build_level_mesh` test: construct synthetic MapData with one polygon, verify non-empty vertices/indices, index count multiple of 3, all indices in bounds

## 4. Visual Regression Baseline

- [x] 4.1 Create `e2e/tests/visual-regression.spec.ts` with pixel coverage test: after game renders 2+ seconds, sample canvas pixels, verify >20% non-black
- [x] 4.2 Add color variety test: quantize sampled pixels to 6-bit per channel, verify >50 unique colors
- [x] 4.3 Add quadrant coverage test: divide canvas into 4 quadrants, verify at least 3 contain non-black pixels

## 5. CI Data Availability

- [x] 5.1 Update `Dockerfile.e2e` (or create a shared data-fetch stage) so that the Rust test Docker image also places Marathon 2 data into `marathon-formats/tests/fixtures/` using the same pinned commit (`eaf21a7`) — already handled by main Dockerfile's `fetch-data` stage
- [x] 5.2 Verify that `cargo test -p marathon-formats -p marathon-viewer -p marathon-game` passes with real data present in Docker — confirmed: all 12 e2e, 22 real-data, 12 viewer tests pass in Docker `test` target
