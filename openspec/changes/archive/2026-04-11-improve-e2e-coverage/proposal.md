## Why

The Playwright browser e2e suite currently covers the "does the game boot" path (WASM init, data serving, loading screen, WebGL2 compatibility) but never verifies that the game is actually playable. There are no tests for keyboard/mouse interaction, HUD updates, pointer lock, or error handling for all data files. Meanwhile, `marathon-web` — the only user-facing crate — has zero Rust-side tests. Additionally, the real-data Rust tests silently skip when fixtures are missing, meaning they can report "pass" with zero actual coverage.

## What Changes

- Add browser interaction e2e tests: keyboard input (WASD movement), pointer lock activation, and HUD element visibility/updates after game starts
- Add missing Physics file 404 error-handling test to complete data-fetch error coverage
- Add `wasm-bindgen-test` suite for `marathon-web` to test the WASM API surface without requiring Docker
- Add a visual regression baseline test that screenshots the running game and compares canvas pixel distribution against known thresholds (not pixel-perfect — just "scene rendered with expected color variety")
- Convert silent fixture skips to explicit `#[ignore]` so `cargo test` reports them accurately and CI can run them with `--include-ignored`

## Capabilities

### New Capabilities
- `browser-interaction-tests`: Playwright tests for keyboard input dispatch, pointer lock, and HUD DOM element updates during gameplay
- `wasm-api-tests`: `wasm-bindgen-test` suite for `marathon-web` covering the `start_game` entry point, data parsing on the WASM side, and error paths
- `visual-regression-baseline`: Playwright screenshot capture with pixel distribution analysis to detect rendering regressions (non-blank, color variety, no all-black/all-white)

### Modified Capabilities
- `real-data-testing`: Convert silent `return` skips to `#[ignore]` with descriptive messages; add Physics 404 error test to Playwright suite

## Impact

- **e2e/tests/**: New spec files for interaction, visual regression
- **e2e/tests/error-handling.spec.ts**: Additional test case for Physics 404
- **marathon-web/**: New `tests/` directory with `wasm-bindgen-test` tests; `Cargo.toml` gains `[dev-dependencies]` for `wasm-bindgen-test`
- **marathon-game/tests/e2e_tests.rs**: `require_fixtures!` macro changes from `return` to `#[ignore]`
- **marathon-viewer/tests/e2e_tests.rs**: Same fixture skip change
- **marathon-formats/tests/**: Same fixture skip change
- **marathon-sim/tests/**: No changes (uses synthetic data, no skips)
