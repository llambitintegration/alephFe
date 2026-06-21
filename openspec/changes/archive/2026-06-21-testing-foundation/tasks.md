# testing-foundation: Tasks

Implementation checklist for the testing foundation infrastructure. Tasks are ordered by dependency -- earlier sections must be completed before later ones can begin. Each task is scoped to be completable in a single session.

---

## 1. BinaryWriter

- [x] 1.1 Create `marathon-formats/src/test_helpers.rs` with `#[cfg(test)]` module declaration in `lib.rs`; implement the `BinaryWriter` struct with an internal `Vec<u8>` buffer and `new()` constructor
- [x] 1.2 Implement `write_i16(self, v: i16) -> Self` and `write_i32(self, v: i32) -> Self` methods that append big-endian bytes to the buffer
- [x] 1.3 Implement `write_u16(self, v: u16) -> Self` and `write_u32(self, v: u32) -> Self` methods that append big-endian bytes to the buffer
- [x] 1.4 Implement `write_bytes(self, data: &[u8]) -> Self` that appends raw bytes verbatim and `write_padding(self, n: usize) -> Self` that appends `n` zero bytes
- [x] 1.5 Implement `write_fixed(self, v: f32) -> Self` that converts an `f32` to a 16.16 fixed-point `i32` (multiply by 65536.0 and cast) and appends it as big-endian
- [x] 1.6 Implement `into_bytes(self) -> Vec<u8>` that consumes the writer and returns the accumulated buffer
- [x] 1.7 Write unit tests for `BinaryWriter`: verify big-endian byte order for all integer types, verify `write_fixed` for 1.0/0.5/-1.0, verify `write_padding` produces zero bytes, verify `write_bytes` copies verbatim, verify fluent chaining produces correct concatenated output

## 2. WadBuilder

- [x] 2.1 Implement `WadBuilder` struct with fields for `version` (default 4), `data_version` (default 0), `file_name` (default empty 64-byte array), `checksum` (default 0), `parent_checksum` (default 0), `application_specific_directory_data_size` (default 0), and an `entries: Vec<EntryBuilder>`
- [x] 2.2 Implement fluent setter methods: `version(v: i16)`, `data_version(v: i16)`, `file_name(name: &str)`, `checksum(c: u32)`, `parent_checksum(c: u32)`, `application_specific_directory_data_size(s: i16)` -- each returning `Self`
- [x] 2.3 Implement `EntryBuilder` and `TagData` helper structs; implement `add_entry(index: i16, tags: Vec<(WadTag, Vec<u8>)>) -> Self` that creates an `EntryBuilder` with the given index and tag payloads
- [x] 2.4 Implement `build() -> Vec<u8>` for version 2+ (new format): write the 128-byte header using `BinaryWriter`, then for each entry write tag chain data (16-byte entry headers with correct `next_offset` linking, followed by tag payloads), then write directory entries (10 bytes each: `offset_to_start`, `length`, `index`); set `entry_header_size=16`, `directory_entry_base_size=10`, compute `directory_offset` and `wad_count` automatically
- [x] 2.5 Implement `build()` support for version 0-1 (old format): use 12-byte entry headers (tag, next_offset, length without the offset field) and 8-byte directory entries (offset_to_start, length without index field); set implicit old-format sizes
- [x] 2.6 Write unit tests: build an empty WAD and verify `WadFile::from_bytes()` succeeds with `entry_count() == 0` and `version == 4`
- [x] 2.7 Write unit tests: build a WAD with one entry containing one `MapInfo` tag (88 bytes payload), parse with `WadFile::from_bytes()`, verify `entry_count() == 1`, verify `get_tag_data(WadTag::MapInfo)` returns the original payload
- [x] 2.8 Write unit tests: build a WAD with two entries each containing two tags (`Endpoints` and `Lines`), parse and verify all tag data round-trips correctly
- [x] 2.9 Write unit tests: build an overlay WAD with `parent_checksum(0x12345678)`, verify `header.is_overlay()` returns `true` and `header.parent_checksum == 0x12345678`
- [x] 2.10 Write unit tests: build a version 0 WAD with one entry and one tag, verify `WadFile::from_bytes()` succeeds and `header.version == 0`

## 3. MapDataBuilder

- [x] 3.1 Implement `MapDataBuilder` with static methods: `endpoint(x: i16, y: i16) -> Vec<u8>` that builds a single 16-byte EPNT record (flags=0, highest_adjacent_floor=0, lowest_adjacent_ceiling=0, vertex x/y, transformed x/y matching vertex, supporting_polygon_index=-1)
- [x] 3.2 Implement `endpoints(points: &[(i16, i16)]) -> Vec<u8>` that concatenates multiple `endpoint()` results into a complete EPNT tag payload
- [x] 3.3 Implement `line(endpoint_a: i16, endpoint_b: i16, cw_poly: i16, ccw_poly: i16) -> Vec<u8>` that builds a single 32-byte LINS record with the given endpoint and polygon references and zero/default values for remaining fields
- [x] 3.4 Implement `lines(defs: &[(i16, i16, i16, i16)]) -> Vec<u8>` that concatenates multiple `line()` results into a complete LINS tag payload
- [x] 3.5 Implement `polygon(vertex_count: u16, endpoint_indexes: &[i16], line_indexes: &[i16]) -> Vec<u8>` that builds a single 128-byte POLY record with the given vertex count, up to 8 endpoint/line indexes, and zero/default values for remaining fields
- [x] 3.6 Write unit tests: build triangle endpoint data (3 points), verify payload is 48 bytes and contains correct coordinates at expected offsets
- [x] 3.7 Write unit tests: build a minimal valid map entry (endpoints, lines, polygon for a triangle), embed into a `WadBuilder` entry, build the WAD, parse with `WadFile::from_bytes()`, verify all three tags are present and contain correctly sized data

## 4. Expand WAD Parser Tests

- [x] 4.1 Add test using `WadBuilder`: multi-entry WAD with 3+ entries, each with different tags; verify `entry_count()`, iterate entries and confirm each has the correct index and tag set
- [x] 4.2 Add test using `WadBuilder`: tag chain walking with multiple tags per entry; verify tags are returned in the correct order and with correct data
- [x] 4.3 Add test for CRC-32 validation: build a WAD, compute its CRC-32 using `compute_crc32`, set the checksum field in the built bytes, parse and verify `validate_checksum()` returns `true`; also verify a corrupted byte causes `validate_checksum()` to return `false`
- [x] 4.4 Add test for overlay WAD detection: build a WAD with `parent_checksum` set to a non-zero value, parse and verify `is_overlay()` returns `true`; build another with `parent_checksum(0)` and verify `is_overlay()` returns `false`
- [x] 4.5 Add test for version 0 old-format WAD: build with `version(0)`, add entries with tags, parse and verify correct entry count, tag data, and `header.version == 0`
- [x] 4.6 Add negative/error case tests: header too short (< 128 bytes), directory offset beyond file end (`DirectoryOutOfBounds`), entry data beyond file end (`EntryOutOfBounds`), negative `wad_count` (`NegativeWadCount`)
- [x] 4.7 Add test for directory data parsing: build a WAD with `application_specific_directory_data_size` set to a non-zero value, add entries, verify `application_data` is correctly read for each entry

## 5. Test Module Organization

- [x] 5.1 Verify `test_helpers.rs` uses `#[cfg(test)]` gating in `lib.rs` so it compiles away in release builds; verify `pub(crate)` visibility allows unit tests in `wad.rs`, `types.rs`, and `tags.rs` to import helpers via `use crate::test_helpers::*`
- [x] 5.2 Create integration test file at `marathon-formats/tests/integration_tests.rs`; add at least one test that uses `marathon_formats::test_helpers::*` to build a WAD, parse it via `WadFile::from_bytes()`, and verify entry contents -- confirming integration tests can access the test helpers
- [x] 5.3 Update `lib.rs` module declaration to use `pub mod test_helpers` (not `pub(crate)`) under `#[cfg(test)]` so integration tests can access the helpers
- [x] 5.4 Verify that `cargo build` (non-test) does not include any test helper code in the compiled output

## 6. CI Pipeline

- [x] 6.1 Update `Dockerfile` to use multi-stage builds: add a `base` stage (`FROM rust:1.82-slim AS base`), a `test` stage (build + test), a `clippy` stage (install clippy component, run `cargo clippy -- -D warnings`), a `fmt` stage (install rustfmt component, run `cargo fmt --check`), and a `coverage` stage (install `cargo-tarpaulin`, run with `--fail-under` reading from `.coverage-threshold`); ensure default `docker build` (no `--target`) still runs build + test
- [x] 6.2 Create `.coverage-threshold` file at the project root containing `60` (the initial coverage floor)
- [x] 6.3 Create `.github/workflows/ci.yml` with trigger on push to `main` and all pull requests; define a single `check` job on `ubuntu-latest` with steps: checkout, build+test (`docker build --target test .`), clippy (`docker build --target clippy .`), fmt (`docker build --target fmt .`), coverage (`docker build --target coverage .`)
- [x] 6.4 Add Docker layer caching to the CI workflow using `actions/cache` or equivalent to cache the cargo registry and build artifacts across runs
- [x] 6.5 Verify the workflow runs all stages by pushing to a test branch or running `act` locally; confirm each stage produces expected output and the coverage threshold is enforced

## 7. TESTING.md

- [x] 7.1 Create `TESTING.md` at the project root with sections: test naming conventions (`test_<unit>_<scenario>` with at least three examples from the codebase), test structure pattern (arrange/act/assert with code example), unit vs integration test guidance, test builder usage examples (`BinaryWriter` and `WadBuilder` with code snippets), coverage expectations and threshold ratchet explanation, and local test execution instructions (`docker build` and `cargo test` commands)
- [x] 7.2 Review all existing test function names across `wad.rs`, `types.rs`, and `tags.rs`; rename any that do not follow the `test_<unit>_<scenario>` convention (e.g., `test_header_too_short` -> `test_wad_header_too_short`, `test_invalid_version` -> `test_wad_invalid_version`, `test_empty_wad` -> `test_wad_empty`)
- [x] 7.3 Verify all tests pass after renaming; run `cargo test` (or `docker build`) to confirm no regressions
