## Context

The project has ~110 tests across 7 test files spanning 5 crates. Browser e2e tests verify the boot path via Playwright + Docker Compose but stop short of testing gameplay interaction. The `marathon-web` WASM crate has no Rust-side tests at all — it's tested only indirectly through the Playwright suite. Real-data Rust tests silently skip when fixtures are absent, which can mask zero coverage in CI.

Current test infrastructure:
- **Playwright** runs in `Dockerfile.e2e` against `docker-compose.e2e.yml` (data, web, tests services)
- **Rust real-data tests** use `require_fixtures!` macro that returns early if files missing
- **marathon-web** has `cdylib` crate type and `#[cfg(target_arch = "wasm32")]` guards on render/start_game

## Goals / Non-Goals

**Goals:**
- Verify the game is playable in-browser (keyboard input dispatches, HUD updates, pointer lock)
- Test marathon-web's non-GPU logic (level loading, mesh generation, texture pipeline) without Docker
- Detect rendering regressions via lightweight pixel distribution checks (not pixel-perfect screenshots)
- Make fixture-dependent test skips visible in test output (`--ignored` vs silent pass)
- Complete the error-handling test matrix (add Physics 404)

**Non-Goals:**
- Pixel-perfect visual regression (too brittle across GPU/driver combos, especially with SwiftShader)
- Firefox/Safari browser matrix (valuable but separate change — different Docker image, different WebGL behavior)
- Corrupted/malformed data file fuzzing (separate security-focused change)
- Performance benchmarks or long-duration stability tests
- Testing the actual wgpu render pipeline in WASM (requires GPU context)

## Decisions

### 1. `wasm-bindgen-test` for non-GPU marathon-web logic

**Decision**: Add `wasm-bindgen-test` dev-dependency and create `tests/wasm.rs` that tests `level::*`, `texture::pad_layer_count_for_webgl`, and `mesh::build_level_mesh` using synthetic MapData (same pattern as marathon-game/tests/integration.rs).

**Why not test `start_game` directly**: It requires a wgpu device + canvas, which `wasm-bindgen-test` can't provide headlessly. The render path is already covered by the Playwright suite.

**Why not native tests**: The `level`, `mesh`, and `texture` modules import `wgpu` types (e.g. `Vertex` uses `wgpu::vertex_attr_array!`). While some functions are pure computation, the modules won't compile for native target without conditional compilation changes. Using `wasm-bindgen-test` with `--headless --chrome` avoids this entirely.

**Alternative considered**: Refactoring modules to split GPU-dependent and pure-logic code. Too invasive for a testing change — better done separately if needed.

### 2. Playwright interaction tests via console event bridge

**Decision**: Test keyboard/mouse interaction by dispatching events to the canvas and observing side effects through console logs. The WASM game loop already logs state changes; tests will listen for specific log patterns after sending input.

**Approach**: 
- Send `KeyboardEvent` for WASD keys via `page.keyboard.press()`
- Verify the game processes input by checking that player position changes (logged or queryable via injected JS bridge)
- Test pointer lock by clicking canvas and checking `document.pointerLockElement`
- Test HUD by reading DOM element text content (`#health-val`, `#shield-val`) after game runs for a few seconds

**Why not a formal test bridge/API**: Adding WASM exports purely for testing is coupling test infrastructure to production code. Console logs and DOM state are already-existing observable outputs.

### 3. Visual regression via pixel distribution, not screenshot diff

**Decision**: After game renders for a few seconds, capture canvas pixels and check:
1. Non-zero pixel count > 20% of total (scene isn't mostly black)
2. Unique color count exceeds threshold (not a solid fill)
3. Pixels exist in multiple screen quadrants (geometry covers the viewport)

**Why not screenshot comparison**: SwiftShader (software renderer in Docker) produces different output from real GPUs. Exact pixel diffs would fail across environments. Distribution checks are stable across renderers while still catching "blank screen" and "completely wrong scene" regressions.

### 4. Convert fixture skips from `return` to `#[ignore]`

**Decision**: Replace the `require_fixtures!` macro's `return` with `panic!("SKIP: ...")` guarded by `#[ignore]`. Tests get the `#[ignore]` attribute; CI runs `cargo test -- --include-ignored`.

**Why**: `cargo test` currently shows "test ... ok" for tests that checked nothing. With `#[ignore]`, `cargo test` shows "test ... ignored" (honest reporting), and CI explicitly opts in with `--include-ignored` after fetching data.

**Migration**: Each test function gets `#[ignore]` attribute with a reason string. The macro keeps its early-return for environments where `--include-ignored` is used but fixtures genuinely aren't present (graceful degradation stays).

**Revised approach**: Actually, `#[ignore]` can't be applied conditionally at runtime. Instead: keep the `require_fixtures!` macro but add a `#[cfg(feature = "real-data")]` feature gate. Tests behind this feature compile only when opted in. For the default `cargo test`, they simply don't exist. CI enables the feature.

**Final decision**: Use the simplest approach — keep `require_fixtures!` as-is but ensure CI scripts (and Docker test Dockerfiles) always provide the fixtures. Document this clearly. For local runs, the existing skip-with-message behavior is actually fine. The real fix is ensuring CI never runs without fixtures, which is already handled by `docker-compose.e2e.yml` fetching data.

On reflection, the real gap here is that `cargo test` (Rust-side, not Playwright) in CI may not have fixtures. The Docker compose only fetches data for the Playwright tests. The fix: add a CI step that fetches Marathon 2 data before `cargo test` in Docker, or ensure the Rust test Docker image also pulls data. This is a CI configuration concern, not a code change.

**Revised decision**: Leave the `require_fixtures!` macro as-is. Instead, ensure the test Docker image (used for `cargo test`) also fetches Marathon 2 data into `marathon-formats/tests/fixtures/`. Document the CI requirement. This keeps the test code simple and honest.

## Risks / Trade-offs

**[Flaky interaction tests]** → Mitigation: Use generous timeouts, `toPass()` retries for console log assertions, and `waitForTimeout` for rendering settle. Tag interaction tests as `@slow` so they can be skipped during rapid iteration.

**[wasm-bindgen-test browser dependency]** → Mitigation: Tests run with `--headless --chrome`, which the Playwright Docker image already provides. Can also run locally if Chrome is installed.

**[Visual regression thresholds]** → Mitigation: Start with very loose thresholds (>20% non-black pixels, >50 unique colors). Tighten over time as we understand the variance. False negatives (missed regression) are better than false positives (flaky failures) at this stage.

**[SwiftShader rendering differences]** → Mitigation: The pixel distribution approach is specifically designed for SwiftShader tolerance. Thresholds are based on structural properties (scene covers viewport, has color variety) not exact pixel values.
