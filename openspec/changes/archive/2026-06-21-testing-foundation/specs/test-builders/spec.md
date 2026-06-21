# Spec: test-builders

Fluent builder APIs for constructing synthetic Marathon binary data in tests. These builders eliminate hand-crafted byte arrays by providing ergonomic helpers for constructing big-endian binary payloads, complete WAD files, and map geometry tag data. All builders live in a `test_helpers` module gated with `#[cfg(test)]` within the `marathon-formats` crate.

**Source reference**: Test patterns observed in `marathon-formats/src/wad.rs`, `marathon-formats/src/types.rs`, and `marathon-formats/src/tags.rs`.

---

## ADDED Requirements

### Requirement: BinaryWriter for constructing big-endian binary payloads

The `BinaryWriter` struct SHALL provide a low-level utility for constructing arbitrary big-endian binary payloads used across all format test modules. It MUST accumulate bytes into an internal buffer and produce the final byte vector on demand.

The `BinaryWriter` MUST support the following methods:

| Method | Description |
|--------|-------------|
| `write_i16(value: i16)` | Append a big-endian signed 16-bit integer |
| `write_i32(value: i32)` | Append a big-endian signed 32-bit integer |
| `write_u16(value: u16)` | Append a big-endian unsigned 16-bit integer |
| `write_u32(value: u32)` | Append a big-endian unsigned 32-bit integer |
| `write_bytes(data: &[u8])` | Append raw bytes verbatim |
| `write_padding(n: usize)` | Append `n` zero bytes |
| `write_fixed(value: f32)` | Convert an `f32` to a 16.16 fixed-point `i32` and append it as big-endian |
| `into_bytes(self) -> Vec<u8>` | Consume the writer and return the accumulated byte buffer |

#### Scenario: Construct a simple two-field struct

WHEN a test creates a `BinaryWriter`, writes an `i16` value of `100` followed by a `u32` value of `0xDEADBEEF`, and calls `into_bytes()`
THEN the resulting byte vector SHALL be exactly 6 bytes long
AND the first two bytes SHALL be `[0x00, 0x64]` (100 as big-endian i16)
AND the next four bytes SHALL be `[0xDE, 0xAD, 0xBE, 0xEF]`

#### Scenario: Verify big-endian byte order for all integer types

WHEN a test writes values using `write_i16`, `write_i32`, `write_u16`, and `write_u32`
THEN all multi-byte values SHALL be serialized in big-endian (most-significant byte first) order
AND the byte order SHALL match the Marathon binary format convention

#### Scenario: Build a multi-field payload with padding

WHEN a test writes an `i16`, calls `write_padding(4)`, then writes another `i16`
THEN the resulting byte vector SHALL contain the first `i16` (2 bytes), followed by 4 zero bytes, followed by the second `i16` (2 bytes), totaling 8 bytes

#### Scenario: Convert f32 to fixed-point via write_fixed

WHEN a test calls `write_fixed(1.0)`
THEN the writer SHALL append the big-endian representation of `0x00010000` (65536 as i32)
AND when a test calls `write_fixed(0.5)`, the writer SHALL append the big-endian representation of `0x00008000` (32768 as i32)
AND when a test calls `write_fixed(-1.0)`, the writer SHALL append the big-endian representation of `-65536` as a signed i32

#### Scenario: Write raw bytes verbatim

WHEN a test calls `write_bytes(&[0x41, 0x42, 0x43])`
THEN the writer SHALL append exactly `[0x41, 0x42, 0x43]` to the buffer without any transformation

---

### Requirement: WadBuilder for constructing complete WAD binary files

The `WadBuilder` SHALL provide a fluent builder API for constructing complete, valid WAD binary files as `Vec<u8>`. The builder MUST handle all binary layout details including header construction, directory offset computation, entry header sizing, and directory entry layout based on the configured WAD version.

The `WadBuilder` MUST support the following configuration methods, each returning `&mut Self` or `Self` for fluent chaining:

| Method | Description |
|--------|-------------|
| `version(v: i16)` | Set the WAD version (0-4); defaults to 4 |
| `data_version(v: i16)` | Set the data version; defaults to 0 |
| `file_name(name: &str)` | Set the 64-byte file name field |
| `checksum(c: u32)` | Set the header checksum field; defaults to 0 |
| `parent_checksum(c: u32)` | Set the parent checksum for overlay WADs; defaults to 0 |
| `application_specific_directory_data_size(s: i16)` | Set app-specific directory data size; defaults to 0 |
| `add_entry(index: i16, tags: Vec<(WadTag, Vec<u8>)>)` | Add an entry with the given logical index and tagged data payloads |
| `build() -> Vec<u8>` | Consume the builder and produce the complete WAD binary |

#### Scenario: Build an empty WAD file

WHEN a test creates a `WadBuilder` with default settings and calls `build()` without adding any entries
THEN the resulting byte vector SHALL be at least 128 bytes (the WAD header)
AND parsing the result with `WadFile::from_bytes()` SHALL succeed
AND the parsed WAD SHALL have `entry_count()` equal to 0
AND the parsed header SHALL have `version` equal to 4 (the default)

#### Scenario: Build a WAD with one entry and one tag

WHEN a test creates a `WadBuilder`, adds one entry with index 0 containing a single `WadTag::MapInfo` tag with 88 bytes of payload data, and calls `build()`
THEN parsing the result with `WadFile::from_bytes()` SHALL succeed
AND the parsed WAD SHALL have `entry_count()` equal to 1
AND the first entry SHALL have `index` equal to 0
AND calling `get_tag_data(WadTag::MapInfo)` on the first entry SHALL return the original 88 bytes

#### Scenario: Build a WAD with multiple entries and multiple tags

WHEN a test creates a `WadBuilder`, adds two entries each containing two different tags (e.g., `Endpoints` and `Lines`), and calls `build()`
THEN parsing the result with `WadFile::from_bytes()` SHALL succeed
AND the parsed WAD SHALL have `entry_count()` equal to 2
AND each entry SHALL contain exactly the tags that were added to it
AND the tag data for each tag SHALL match the original payload bytes

#### Scenario: Build an overlay WAD

WHEN a test creates a `WadBuilder`, sets `parent_checksum` to a non-zero value (e.g., `0x12345678`), adds at least one entry, and calls `build()`
THEN parsing the result with `WadFile::from_bytes()` SHALL succeed
AND `header.is_overlay()` SHALL return `true`
AND `header.parent_checksum` SHALL equal `0x12345678`

#### Scenario: Build an old-format (version 0) WAD

WHEN a test creates a `WadBuilder`, sets `version(0)`, adds one entry with one tag, and calls `build()`
THEN the builder SHALL use 8-byte old directory entries (offset_to_start: i32, length: i32) without an index field
AND the builder SHALL use 12-byte old entry headers (tag: u32, next_offset: i32, length: i32)
AND parsing the result with `WadFile::from_bytes()` SHALL succeed
AND the parsed WAD SHALL have `header.version` equal to 0

#### Scenario: Automatic directory offset computation

WHEN a test builds a WAD with any number of entries and tags
THEN the builder SHALL automatically compute the `directory_offset` header field to point to the byte position immediately after all entry data
AND the builder SHALL automatically set `wad_count` to the number of entries added
AND the builder SHALL automatically set `entry_header_size` and `directory_entry_base_size` to the correct values for the configured version

#### Scenario: Builder sets version-appropriate sizes

WHEN a test builds a WAD with version 2 or higher
THEN the builder SHALL set `entry_header_size` to 16 and `directory_entry_base_size` to 10
AND WHEN a test builds a WAD with version 0 or 1
THEN the builder SHALL use old entry header size of 12 and old directory entry size of 8 (these values are implicit and not stored in the header for old-format WADs)

---

### Requirement: MapDataBuilder for constructing map geometry tag data

The `MapDataBuilder` SHALL provide a convenience builder for constructing map geometry tag data (endpoints, lines, polygons) as byte payloads. These payloads are intended to be embedded into WAD entries via `WadBuilder.add_entry()`.

The `MapDataBuilder` MUST support at minimum the following methods:

| Method | Description |
|--------|-------------|
| `endpoint(x: i16, y: i16) -> Vec<u8>` | Build a single 16-byte EPNT endpoint record with the given coordinates and default values for other fields |
| `endpoints(points: &[(i16, i16)]) -> Vec<u8>` | Build a complete EPNT tag payload from a list of (x, y) coordinates |
| `line(endpoint_a: i16, endpoint_b: i16, cw_poly: i16, ccw_poly: i16) -> Vec<u8>` | Build a single 32-byte LINS line record |
| `lines(defs: &[(i16, i16, i16, i16)]) -> Vec<u8>` | Build a complete LINS tag payload from a list of line definitions |
| `polygon(vertex_count: u16, endpoint_indexes: &[i16], line_indexes: &[i16]) -> Vec<u8>` | Build a single 128-byte POLY polygon record |

#### Scenario: Build endpoint data for a triangle

WHEN a test calls `MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (512, 1024)])`
THEN the result SHALL be a 48-byte payload (3 endpoints * 16 bytes each)
AND parsing each 16-byte record as an EPNT structure SHALL yield the correct x, y coordinates
AND the payload SHALL be suitable for embedding as a `WadTag::Endpoints` tag in a `WadBuilder` entry

#### Scenario: Build line data connecting endpoints

WHEN a test calls `MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 0, 0, -1)])`
THEN the result SHALL be a 96-byte payload (3 lines * 32 bytes each)
AND each line record SHALL have the correct endpoint indexes and polygon owner references

#### Scenario: Build a minimal valid map entry

WHEN a test uses `MapDataBuilder` to create endpoint, line, and polygon payloads for a triangle, then embeds them as `Endpoints`, `Lines`, and `Polygons` tags in a `WadBuilder` entry, and builds the WAD
THEN parsing the resulting WAD with `WadFile::from_bytes()` SHALL succeed
AND the entry SHALL contain all three tags with correctly structured data
AND the endpoint, line, and polygon records SHALL be parseable by the map geometry parser
