## Why

The alephone-rust project is parsing undocumented binary formats where correctness is everything — a single byte misread corrupts all downstream data. We currently have 10 unit tests covering happy paths, but no way to construct realistic test data, no integration tests, no CI pipeline, and no coverage tracking. As we build out map geometry, shapes, sounds, physics, and four more crates on top of this foundation, testing debt will compound. Establishing testing infrastructure now — before the codebase grows — ensures every future module starts with a testable pattern, not an afterthought.

## What Changes

- Create a `test-helpers` module within `marathon-formats` that provides a **WAD builder** — a fluent API for constructing synthetic WAD files with arbitrary headers, directory entries, and tagged data payloads, eliminating hand-crafted byte arrays in tests
- Create a **binary builder** utility for constructing arbitrary big-endian binary payloads (structs, arrays, padding) used across all format test modules
- Expand WAD parser unit tests to cover: multi-entry WADs, tag chain walking, CRC-32 validation, overlay WAD detection, version 0-1 old-format parsing, directory data parsing, and all error paths
- Establish an `integration_tests` module structure at `marathon-formats/tests/` for cross-module tests that parse complete synthetic scenarios (WAD → map geometry → validation)
- Add a **GitHub Actions CI workflow** that builds and tests in Docker on every push and PR, with test result reporting
- Add `cargo-tarpaulin` coverage measurement to CI with a minimum coverage threshold that ratchets upward as modules are implemented
- Document testing conventions and patterns in a `TESTING.md` guide so future contributors (and future crates) follow consistent patterns

## Capabilities

### New Capabilities

- `test-builders`: Fluent builder APIs for constructing synthetic Marathon binary data — WAD files, map geometry entries, shape collections, and arbitrary big-endian struct payloads — enabling focused, readable tests without hand-crafted byte arrays
- `ci-pipeline`: GitHub Actions workflow for automated build, test, lint (clippy), format check (rustfmt), and coverage measurement on every push and pull request, running in a Docker container matching the project's build environment
- `test-conventions`: Project-wide testing standards — module test organization, naming conventions, coverage expectations, integration test patterns, and fixture management — documented and enforced through CI

### Modified Capabilities

(none)

## Impact

- **New files**: `marathon-formats/src/test_helpers.rs`, `marathon-formats/tests/integration_tests.rs`, `.github/workflows/ci.yml`, `TESTING.md`
- **New dev-dependencies**: `cargo-tarpaulin` (coverage, CI only)
- **CI**: New GitHub Actions workflow; all PRs must pass build + test + clippy + fmt + coverage threshold
- **Developer workflow**: Every future module (map, shapes, sounds, physics, mml, plugin) will use the test builders, follow the conventions doc, and be required to meet coverage thresholds before merge
- **Existing code**: WAD module gains ~15-20 additional unit tests; no changes to public API
