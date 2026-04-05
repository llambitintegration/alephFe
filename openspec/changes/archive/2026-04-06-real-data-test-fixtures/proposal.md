## Why

Our test suite has 182 unit tests that parse hand-crafted binary data, but zero tests against real Marathon game files. The `real_data_tests.rs` file already has 12 tests written for real data — they silently skip in CI because the fixtures don't exist. Meanwhile, the Aleph One project hosts Marathon 2 (and Infinity) scenario data as public GitHub submodules under a Bungie distribution license, and the Aleph One engine ships GPL-3.0 MML and Plugin.xml files that exercise sections our synthetic fixtures never touch (`<console>`, `<opengl>`, `<interface>`, `<scenario>`). We should use both sources to build real-data coverage incrementally.

## What Changes

- **Copy GPL-3.0 Aleph One engine MML and Plugin.xml files** into `tests/fixtures/alephone/` as committed fixtures. These provide real-world coverage for MML sections and Plugin.xml attributes that our synthetic `sample.mml` and `sample_plugin/Plugin.xml` don't cover.
- **Add a CI stage that fetches Marathon 2 scenario data** from the public `github.com/Aleph-One-Marathon/data-marathon-2.git` repo before running tests. This provides real Map (WAD), Shapes, Sounds, and Physics files so `real_data_tests.rs` stops skipping.
- **Write new tests against the committed GPL fixtures** to cover previously untested MML sections and Plugin.xml attribute variants.
- **Add snapshot-style assertions to real-data tests** with known-good values from Marathon 2 level data (endpoint counts, polygon counts, etc.) to catch parsing regressions.

## Capabilities

### New Capabilities
- `real-data-testing`: Infrastructure and tests for running the parser suite against real Marathon game data files, both committed GPL fixtures and CI-fetched scenario data.

### Modified Capabilities
_(none — no existing spec-level requirements change)_

## Impact

- **Tests**: New test functions in `real_data_tests.rs` and a new test file for GPL fixture tests. Existing skipped tests become active in CI.
- **CI / Dockerfile**: New Docker stage to fetch Marathon 2 data submodule (~50 MB) before the test stage. CI build time increases by the clone duration.
- **Fixtures**: New committed files in `tests/fixtures/alephone/` (~15 KB of MML/XML). Large binary game data is fetched at CI time, never committed.
- **Dependencies**: No new Rust crate dependencies.
