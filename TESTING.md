# Testing Guide

## Running Tests

**Via Docker (canonical, no local Rust required):**
```bash
# Build and run all tests
docker build --target test .

# Run clippy lint check
docker build --target clippy .

# Run format check
docker build --target fmt .

# Run coverage measurement
docker build --target coverage .
```

**Via local Rust toolchain (optional, if installed):**
```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## Test Naming Convention

All test functions follow the pattern: `test_<unit>_<scenario>`

- `<unit>` identifies what is being tested (a function, type, or module)
- `<scenario>` describes the condition or input being tested

Examples from the codebase:
- `test_wad_header_too_short` — WAD header parsing with truncated input
- `test_shape_descriptor_roundtrip` — ShapeDescriptor encode/decode round-trip
- `test_binary_writer_i16_big_endian` — BinaryWriter i16 output byte order
- `test_wad_builder_multi_entry_multi_tag` — WadBuilder with complex structure

## Test Structure

Use the Arrange-Act-Assert pattern:

```rust
#[test]
fn test_wad_builder_single_entry() {
    // Arrange
    let payload = vec![0xAA; 88];
    let data = WadBuilder::new()
        .version(4)
        .add_entry(0, vec![TagData::new(WadTag::MapInfo, payload.clone())])
        .build();

    // Act
    let wad = WadFile::from_bytes(&data).unwrap();

    // Assert
    assert_eq!(wad.entry_count(), 1);
    let tag_data = wad.entry(0).unwrap().get_tag_data(WadTag::MapInfo).unwrap();
    assert_eq!(tag_data, &payload[..]);
}
```

## Unit vs Integration Tests

**Unit tests** (`#[cfg(test)] mod tests` in each source file):
- Test a single module's internal logic
- Test error paths and edge cases
- Have access to `pub(crate)` items

**Integration tests** (`marathon-formats/tests/` directory):
- Test cross-module workflows
- Build a WAD → parse it → extract typed data → validate
- Use only the public API plus test helpers

## Test Builders

Always prefer builders over raw byte arrays.

**BinaryWriter** — for constructing arbitrary big-endian binary payloads:
```rust
let payload = BinaryWriter::new()
    .write_i16(100)     // x coordinate
    .write_i16(200)     // y coordinate
    .write_padding(12)  // reserved fields
    .build();
```

**WadBuilder** — for constructing complete WAD files:
```rust
let data = WadBuilder::new()
    .version(4)
    .add_entry(0, vec![
        TagData::new(WadTag::Endpoints, endpoint_payload),
        TagData::new(WadTag::Lines, line_payload),
    ])
    .build();

let wad = WadFile::from_bytes(&data).unwrap();
```

**MapDataBuilder** — for constructing map geometry tag payloads:
```rust
let endpoints = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (0, 1024)]);
let lines = MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 0, 0, -1)]);
let polygon = MapDataBuilder::polygon(3, &[0, 1, 2], &[0, 1, 2]);
```

If a builder is missing a feature you need, extend the builder rather than falling back to manual byte construction.

## Coverage

- The CI enforces a minimum coverage threshold stored in `.coverage-threshold`
- The threshold ratchets upward: when coverage improves, bump the threshold
- Every new module must include tests for: one happy-path parse, one malformed input, and one edge case
- The threshold is a floor, not a goal — aim higher

## Synthetic Data Only

All test data is constructed programmatically via builders. No binary fixture files. No copyrighted Marathon game data in the repository.
