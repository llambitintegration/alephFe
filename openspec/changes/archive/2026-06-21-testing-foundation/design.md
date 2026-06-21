# testing-foundation: Design

## Context

The `marathon-formats` crate parses Marathon's undocumented big-endian binary formats -- WAD containers, map geometry, shapes, sounds, physics, MML, and plugins. Byte-level correctness is critical: a single off-by-one in a field offset corrupts all downstream data.

**Current state:**

- **10 unit tests** across three modules (`wad.rs`: 3 tests, `types.rs`: 5 tests, `tags.rs`: 2 tests). The remaining 8 modules (`map.rs`, `shapes.rs`, `sounds.rs`, `physics.rs`, `mml.rs`, `plugin.rs`, `error.rs`, `lib.rs`) have zero tests.
- **Manual byte array construction** -- existing WAD tests build raw `[u8; 128]` arrays and poke individual byte offsets by hand (e.g., `data[78] = 0; data[79] = 16;` for `entry_header_size`). This is fragile, unreadable, and does not scale.
- **No CI** -- there is no `.github/workflows/` directory. Quality checks (build, test, clippy, fmt) are manual or not run at all.
- **No coverage tracking** -- no measurement of what code paths are exercised.
- **Docker-based build** -- the project builds inside `rust:1.82-slim` via a Dockerfile. There is no local Rust toolchain requirement. CI must respect this.
- **Open source** -- contributors need clear, documented testing patterns. The project will grow to 5+ crates (marathon-viewer, marathon-sim, marathon-audio, marathon-integration, and more). Testing conventions established now propagate across the entire workspace.

## Goals

- Make writing tests easy and readable via builder APIs that produce valid binary data
- Automate quality gates via CI: build, test, clippy, fmt, and coverage on every push and PR
- Establish testing patterns that scale across 5+ future crates without per-crate reinvention
- Implement a ratcheting coverage threshold that starts at the current level and only goes up

## Non-Goals

- **Fuzzing** -- valuable for binary parsers but out of scope; future work after core test coverage is solid
- **Property-based testing** (e.g., proptest/quickcheck) -- future work once builders are mature enough to generate arbitrary valid inputs
- **Performance benchmarks** -- parsing is fast enough; optimize when profiling shows a need
- **Testing with real copyrighted Marathon data files** -- all test data must be synthetically constructed; no copyrighted game files in the repository

## Decisions

### 1. WAD Builder pattern

**Decision**: Provide a fluent builder API (`WadBuilder`) that constructs complete, valid WAD binary data as `Vec<u8>`. This is the testing cornerstone -- every format module needs to construct WADs containing their tag data to test round-trip parsing.

**Problem it solves**: The current test style requires developers to know the exact byte offsets of every WAD header field and manually poke values into a raw byte array. This is the `test_empty_wad` pattern today:

```rust
// Current: fragile, unreadable, error-prone
let mut data = [0u8; 128];
data[1] = 4;    // version -- which field is this?
data[74] = 0;   // wad_count high byte -- easy to get wrong
data[75] = 0;   // wad_count low byte
data[68] = 0;   // directory_offset byte 0 -- wait, isn't this checksum?
data[69] = 0;
data[70] = 0;
data[71] = 128;
data[78] = 0;   // entry_header_size
data[79] = 16;
data[80] = 0;   // directory_entry_base_size
data[81] = 10;
```

The builder replaces this with:

```rust
// New: self-documenting, correct by construction
let data = WadBuilder::new()
    .version(4)
    .data_version(1)
    .build();

let wad = WadFile::from_bytes(&data).unwrap();
assert_eq!(wad.entry_count(), 0);
assert_eq!(wad.header.version, 4);
```

Adding entries with tag data:

```rust
let points_payload = BinaryWriter::new()
    .write_i16(100)   // x
    .write_i16(200)   // y
    .build();

let data = WadBuilder::new()
    .version(4)
    .data_version(1)
    .add_entry(0, vec![
        TagData::new(WadTag::Points, points_payload),
    ])
    .build();

let wad = WadFile::from_bytes(&data).unwrap();
assert_eq!(wad.entry_count(), 1);
let entry = wad.entry(0).unwrap();
assert!(entry.get_tag_data(WadTag::Points).is_some());
```

**Implementation sketch**:

```rust
pub(crate) struct WadBuilder {
    version: i16,
    data_version: i16,
    file_name: [u8; 64],
    parent_checksum: u32,
    entries: Vec<EntryBuilder>,
}

pub(crate) struct EntryBuilder {
    index: i16,
    application_data: Vec<u8>,
    tags: Vec<TagData>,
}

pub(crate) struct TagData {
    tag: WadTag,
    payload: Vec<u8>,
}
```

The builder handles all layout concerns: computing `directory_offset`, writing the header with correct `wad_count`/`entry_header_size`/`directory_entry_base_size`, serializing entry headers with tag chains (setting `next_offset` pointers correctly), and appending the directory at the end. Callers only specify the logical content.

**Version support**: The builder defaults to version 4 (Marathon Infinity format, `entry_header_size=16`, `directory_entry_base_size=10`). A `.version(0)` call switches to old-format layout (`entry_header_size=12`, `directory_entry_base_size=8`, no index field in directory entries). This lets tests cover all version code paths.

### 2. BinaryWriter helper

**Decision**: Provide a small utility struct for writing big-endian binary payloads with named methods: `write_i16`, `write_i32`, `write_u16`, `write_u32`, `write_bytes`, `write_padding`.

**Problem it solves**: Even with the WAD builder, individual struct payloads (the data inside a tag) still need to be constructed as byte vectors. Without a helper, tests would still be doing:

```rust
// Without BinaryWriter: still manual and error-prone
let mut payload = Vec::new();
payload.extend_from_slice(&100_i16.to_be_bytes());
payload.extend_from_slice(&200_i16.to_be_bytes());
payload.extend_from_slice(&[0u8; 12]); // padding
```

With the helper:

```rust
let payload = BinaryWriter::new()
    .write_i16(100)    // x
    .write_i16(200)    // y
    .write_padding(12) // reserved fields
    .build();
```

**Scope**: This is intentionally minimal -- just a `Vec<u8>` wrapper with big-endian write methods. It is not a general-purpose serialization framework. Methods:

```rust
pub(crate) struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self;
    pub fn write_i16(self, v: i16) -> Self;
    pub fn write_i32(self, v: i32) -> Self;
    pub fn write_u16(self, v: u16) -> Self;
    pub fn write_u32(self, v: u32) -> Self;
    pub fn write_bytes(self, data: &[u8]) -> Self;
    pub fn write_padding(self, count: usize) -> Self;
    pub fn build(self) -> Vec<u8>;
}
```

All methods consume and return `self` for fluent chaining. The builder is used by both `WadBuilder` internally (to write headers and directory entries) and by test code directly (to construct tag payloads).

### 3. Test module organization

**Decision**: Three-tier test organization -- unit tests in-module, shared helpers in a `cfg(test)` gated file, and integration tests in the standard Cargo location.

**Unit tests**: Each source module contains `#[cfg(test)] mod tests { ... }` at the bottom, following Rust convention and the existing pattern in `wad.rs`, `types.rs`, and `tags.rs`. Unit tests exercise internal functions and error paths.

**Shared test helpers**: A new file `marathon-formats/src/test_helpers.rs` containing `WadBuilder`, `BinaryWriter`, and any future builder utilities. The module is gated with `#[cfg(test)]` so it compiles away in release builds and does not leak into the public API:

```rust
// In lib.rs:
#[cfg(test)]
pub(crate) mod test_helpers;
```

Using `pub(crate)` visibility means all modules within `marathon-formats` can import the helpers in their test blocks via `use crate::test_helpers::*;`. The `#[cfg(test)]` gate ensures helpers are only available during test compilation.

**Integration tests**: `marathon-formats/tests/` directory for cross-module tests that exercise complete parsing pipelines (e.g., build a WAD with map geometry tags, parse it via `WadFile::from_bytes`, extract endpoints and polygons, validate cross-references). Integration tests can access `pub(crate)` helpers through a re-export pattern:

```rust
// In lib.rs (cfg(test) gated):
#[cfg(test)]
pub mod test_helpers;  // pub for integration test access
```

Note: Since integration tests are external to the crate, they can only access `pub` items. The `test_helpers` module is made `pub` under `#[cfg(test)]` so integration tests can use it, but it still compiles away in non-test builds.

### 4. CI pipeline design

**Decision**: A single GitHub Actions workflow (`.github/workflows/ci.yml`) that runs all quality checks inside Docker, triggered on pushes to `main` and all pull requests.

**Why Docker**: The project deliberately has no local Rust toolchain requirement -- the Dockerfile (`rust:1.82-slim`) is the build environment. CI must match this. Running `docker build` in CI ensures the same environment as local development.

**Workflow structure**:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build and test
        run: docker build --target test .

      - name: Clippy
        run: docker build --target clippy .

      - name: Format check
        run: docker build --target fmt .

      - name: Coverage
        run: docker build --target coverage .
```

**Multi-stage Dockerfile**: Extend the existing Dockerfile with named build stages so CI can target specific checks:

```dockerfile
FROM rust:1.82-slim AS base
WORKDIR /app
COPY . .

FROM base AS test
RUN cargo build 2>&1
RUN cargo test 2>&1

FROM base AS clippy
RUN rustup component add clippy
RUN cargo clippy -- -D warnings 2>&1

FROM base AS fmt
RUN rustup component add rustfmt
RUN cargo fmt --check 2>&1

FROM base AS coverage
RUN cargo install cargo-tarpaulin
RUN cargo tarpaulin --out stdout --fail-under 60 2>&1
```

The default `docker build` (no `--target`) still runs build + test as before, preserving the existing workflow. CI targets individual stages for granular reporting.

**Layer caching**: GitHub Actions Docker layer caching (`docker/build-push-action` with `cache-from`/`cache-to` or `actions/cache` on Docker layers) mitigates rebuild times. The `rust:1.82-slim` base image and compiled dependencies are cached across runs. Only source code changes trigger recompilation.

### 5. Coverage with cargo-tarpaulin

**Decision**: Use `cargo-tarpaulin` for code coverage measurement, run inside Docker, with a ratcheting minimum threshold.

**Why tarpaulin**: It is the most widely used Rust coverage tool, works without nightly Rust, produces multiple output formats, and supports a `--fail-under` flag for CI enforcement. No external coverage service (Codecov, Coveralls) is needed -- results go to stdout and the GitHub Actions job summary.

**Threshold strategy**: Start with a realistic threshold based on current coverage. With 10 tests covering `wad.rs` (3 tests), `types.rs` (5 tests), and `tags.rs` (2 tests), and 8 untested modules, current coverage is likely in the 15-25% range. However, this change will add significant tests alongside the builders, so the initial threshold should be set at 60% -- achievable after the new WAD tests are added. The threshold ratchets upward: when a PR increases coverage, the threshold in the Dockerfile is bumped to the new floor (rounded down to the nearest 5%). This is a manual ratchet -- a developer updates the number when coverage improves.

**Output**: Tarpaulin outputs a per-file coverage summary to stdout, which appears in CI logs. For GitHub PR visibility, the coverage stage can write to `$GITHUB_STEP_SUMMARY`:

```dockerfile
RUN cargo tarpaulin --out stdout 2>&1 | tee /tmp/coverage.txt
```

The workflow step then appends the output to the job summary.

**No external services**: Coverage data stays in CI logs. No Codecov tokens, no third-party uploads, no badges to maintain. If external reporting is wanted later, tarpaulin supports `--out Lcov` for upload.

### 6. TESTING.md conventions

**Decision**: Create a `TESTING.md` at the repository root documenting testing standards for all crates in the workspace.

**Contents**:

1. **Test naming**: `test_<unit>_<scenario>` pattern. The `<unit>` identifies what is being tested (a function, method, or logical unit). The `<scenario>` describes the condition or input. Examples:
   - `test_header_too_short` -- unit: header parsing, scenario: input too short
   - `test_tag_roundtrip` -- unit: tag conversion, scenario: round-trip encode/decode
   - `test_parse_wad_multi_entry` -- unit: WAD parsing, scenario: multiple entries
   - `test_wad_overlay_detection` -- unit: overlay check, scenario: parent_checksum set

2. **Test structure**: Arrange-Act-Assert with blank line separators and comments for complex setups:
   ```rust
   #[test]
   fn test_parse_wad_single_entry_with_points() {
       // Arrange
       let points = BinaryWriter::new()
           .write_i16(100)
           .write_i16(200)
           .build();
       let data = WadBuilder::new()
           .version(4)
           .add_entry(0, vec![TagData::new(WadTag::Points, points)])
           .build();

       // Act
       let wad = WadFile::from_bytes(&data).unwrap();

       // Assert
       assert_eq!(wad.entry_count(), 1);
       let tag_data = wad.entry(0).unwrap().get_tag_data(WadTag::Points).unwrap();
       assert_eq!(tag_data.len(), 4);
   }
   ```

3. **When to use unit vs integration tests**:
   - **Unit tests** (`#[cfg(test)] mod tests`): Test a single module's internal logic, error paths, edge cases, and parsing of individual structs. These have access to `pub(crate)` and private items.
   - **Integration tests** (`tests/` directory): Test cross-module workflows -- e.g., build a WAD, parse it, extract map geometry, validate that endpoint indices referenced by lines actually exist. These use only the public API (plus `test_helpers`).

4. **Using builders**: Always prefer `WadBuilder` + `BinaryWriter` over raw byte arrays. If a builder is missing a feature you need, extend the builder rather than falling back to manual construction.

5. **Coverage expectations**: Every new module must include tests for at least: one happy-path parse, one truncated/malformed input, and one edge case (empty array, maximum valid value, boundary condition). The CI coverage threshold is the floor, not the goal.

6. **Synthetic data only**: All test data is constructed programmatically via builders. No binary fixture files checked into the repository. No copyrighted Marathon game data.

## Risks / Trade-offs

### Docker build times increase with CI stages

Adding clippy, fmt, and tarpaulin stages increases the total CI time. Each stage starts from the `base` layer (shared `COPY . .` and dependency compilation), but tarpaulin in particular is slow to install and run (it instruments every function).

**Mitigation**: Multi-stage builds share the base layer. Docker layer caching in GitHub Actions means dependencies are only recompiled when `Cargo.toml` or `Cargo.lock` change. The stages run as separate `docker build` commands, which could be parallelized across CI jobs if build time becomes a problem. Tarpaulin install can be cached by pinning a version and caching the cargo install directory.

### Coverage threshold too aggressive early

Setting the threshold too high before enough tests exist causes CI to fail on unrelated PRs, frustrating contributors. Setting it too low makes it meaningless.

**Mitigation**: Start at 60%, which is achievable after the WAD builder tests are added. Ratchet manually -- a human reviews coverage output and bumps the threshold when it is safe to do so. The threshold is stored as a single number in the Dockerfile, easy to find and update.

### Test helpers become their own maintenance burden

Builder APIs require maintenance as the formats they construct evolve. If builders become too complex or too numerous, they become a second codebase to debug.

**Mitigation**: Keep builders minimal. `WadBuilder` handles WAD-level structure only -- it does not know about map geometry structs, polygon layouts, or shape collections. Individual modules build their own tag payloads using `BinaryWriter`, which is a trivial wrapper. No deep builder hierarchies, no generics, no traits. If a builder method is only used by one test, inline the logic in the test instead.

### Tarpaulin accuracy limitations

`cargo-tarpaulin` occasionally reports inaccurate line-level coverage, particularly for match arms, closures, and code behind `cfg` attributes. It can also be slow on larger codebases.

**Mitigation**: Use tarpaulin as a directional metric, not an absolute measure. The threshold is conservative (starts at 60%, not 90%). If tarpaulin proves problematic, switching to `cargo llvm-cov` (requires nightly or specific LLVM tooling) is a future option that does not affect the rest of the testing infrastructure.

### No local test runner without Docker

Developers without a local Rust toolchain must run `docker build` to execute tests, which is slower than native `cargo test`. This is an existing trade-off of the project's Docker-only build strategy, not introduced by this change.

**Mitigation**: Document in TESTING.md that developers with a local Rust 1.82+ toolchain can run `cargo test` directly. The Docker path is the canonical CI path and the only one required to pass.
