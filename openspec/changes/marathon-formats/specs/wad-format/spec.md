# Spec: wad-format

WAD container parsing for Marathon/Aleph One binary files. WAD is the universal container format used for maps, shapes, sounds, physics, saves, and films. This spec defines requirements for reading WAD file headers, versioned directory structures, and tagged chunk entries from WAD files across all format versions.

**Source reference**: `wad.h` and `wad.cpp` from the Aleph One C++ engine.

---

## ADDED Requirements

### Requirement: Parse WAD file header

The parser SHALL read and decode the 128-byte WAD file header from the start of any WAD file. All multi-byte integer fields are stored in big-endian byte order.

The header layout is:

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 2 | i16 | `version` |
| 2 | 2 | i16 | `data_version` |
| 4 | 64 | char[64] | `file_name` |
| 68 | 4 | u32 | `checksum` |
| 72 | 4 | i32 | `directory_offset` |
| 76 | 2 | i16 | `wad_count` |
| 78 | 2 | i16 | `application_specific_directory_data_size` |
| 80 | 2 | i16 | `entry_header_size` |
| 82 | 2 | i16 | `directory_entry_base_size` |
| 84 | 4 | u32 | `parent_checksum` |
| 88 | 40 | i16[20] | `unused` (reserved, must be zero-filled) |

Total: 128 bytes.

#### Scenario: Valid WAD header

WHEN a byte source of at least 128 bytes is provided
THEN the parser SHALL decode all header fields using big-endian byte order
AND the resulting header struct SHALL expose every field listed in the layout table above
AND the `file_name` field SHALL be treated as a null-terminated C string padded to 64 bytes

#### Scenario: Truncated header

WHEN a byte source contains fewer than 128 bytes
THEN the parser SHALL return an error indicating the file is truncated
AND the error SHALL include the actual size available and the expected minimum of 128 bytes

#### Scenario: Header field accessibility

WHEN the header has been successfully parsed
THEN the `version` field SHALL be accessible as a signed 16-bit integer
AND the `data_version` field SHALL be accessible as a signed 16-bit integer
AND the `checksum` field SHALL be accessible as an unsigned 32-bit integer
AND the `directory_offset` field SHALL be accessible as a signed 32-bit integer
AND the `wad_count` field SHALL be accessible as a signed 16-bit integer
AND the `application_specific_directory_data_size` field SHALL be accessible as a signed 16-bit integer
AND the `entry_header_size` field SHALL be accessible as a signed 16-bit integer
AND the `directory_entry_base_size` field SHALL be accessible as a signed 16-bit integer
AND the `parent_checksum` field SHALL be accessible as an unsigned 32-bit integer

---

### Requirement: Support WAD versions 0 through 4

The parser SHALL accept WAD files with version values 0, 1, 2, 3, and 4. Each version implies different directory entry and entry header formats.

Version semantics:

| Version | Constant | Meaning |
|---------|----------|---------|
| 0 | `PRE_ENTRY_POINT_WADFILE_VERSION` | Pre-entry-point; old directory entries (8 bytes), old entry headers (12 bytes) |
| 1 | `WADFILE_HAS_DIRECTORY_ENTRY` | Has directory entries; old directory entries (8 bytes), old entry headers (12 bytes) |
| 2 | `WADFILE_SUPPORTS_OVERLAYS` | Supports overlays; new directory entries (10 bytes), entry header size from header field |
| 3 | (intermediate) | New directory entries; entry header size from header field |
| 4 | `WADFILE_HAS_INFINITY_STUFF` | Marathon Infinity format; new directory entries (10 bytes), entry header size from header field |

#### Scenario: Version 0 format selection

WHEN a WAD header has `version` equal to 0
THEN the parser SHALL use old directory entries of 8 bytes (offset_to_start: i32, length: i32)
AND the parser SHALL use old entry headers of 12 bytes (tag: u32, next_offset: i32, length: i32)
AND the parser SHALL treat `application_specific_directory_data_size` as 0

#### Scenario: Version 1 format selection

WHEN a WAD header has `version` equal to 1
THEN the parser SHALL use old directory entries of 8 bytes (offset_to_start: i32, length: i32)
AND the parser SHALL use old entry headers of 12 bytes (tag: u32, next_offset: i32, length: i32)

#### Scenario: Version 2 and above format selection

WHEN a WAD header has `version` greater than or equal to 2
THEN the parser SHALL use the `directory_entry_base_size` from the header to determine directory entry size
AND the parser SHALL use the `entry_header_size` from the header to determine entry header size
AND the new directory entry format SHALL be at least 10 bytes (offset_to_start: i32, length: i32, index: i16)
AND the new entry header format SHALL be at least 16 bytes (tag: u32, next_offset: i32, length: i32, offset: i32)

#### Scenario: Unrecognized version

WHEN a WAD header has a `version` value outside the range 0 through 4
THEN the parser SHALL return an error indicating the version is unrecognized
AND the error SHALL include the actual version value encountered

---

### Requirement: Read WAD directory

The parser SHALL read the WAD directory, which is an array of directory entries located at the byte offset specified by `directory_offset` in the header. The number of entries is `wad_count`. Each directory entry is followed by optional application-specific data.

The total size of the directory is: `wad_count * (directory_entry_base_size + application_specific_directory_data_size)`.

#### Scenario: Reading directory with old entries (version 0 or 1)

WHEN a WAD file has version 0 or 1
THEN the parser SHALL read `wad_count` directory entries from `directory_offset`
AND each entry SHALL be 8 bytes: `offset_to_start` (i32) followed by `length` (i32)
AND both fields SHALL be decoded as big-endian
AND for version 0, `application_specific_directory_data_size` SHALL be treated as 0
AND the sequential index in the directory array SHALL serve as the entry index

#### Scenario: Reading directory with new entries (version 2+)

WHEN a WAD file has version 2 or higher
THEN the parser SHALL read `wad_count` directory entries from `directory_offset`
AND each entry SHALL have at minimum: `offset_to_start` (i32), `length` (i32), `index` (i16)
AND the `index` field SHALL identify the logical entry index for in-place modification support
AND application-specific directory data of `application_specific_directory_data_size` bytes SHALL follow each base directory entry

#### Scenario: Accessing application-specific directory data

WHEN a WAD file has `application_specific_directory_data_size` greater than 0
THEN the parser SHALL expose the application-specific data bytes for each directory entry
AND the application-specific data SHALL be located immediately after the base directory entry fields for each entry
AND the data SHALL be returned as raw bytes of exactly `application_specific_directory_data_size` length

#### Scenario: Directory offset beyond file bounds

WHEN the `directory_offset` value in the header points beyond the end of the file
THEN the parser SHALL return an error indicating the directory offset is out of bounds
AND the error SHALL include the directory offset and the actual file size

#### Scenario: Directory extends beyond file bounds

WHEN the directory region starting at `directory_offset` with total size `wad_count * (directory_entry_base_size + application_specific_directory_data_size)` extends beyond the end of the file
THEN the parser SHALL return an error indicating the directory is truncated

#### Scenario: Indexed entry lookup for version 2+

WHEN a caller requests a WAD entry by logical index from a version 2+ file
THEN the parser SHALL search the directory entries for an entry whose `index` field matches the requested logical index
AND the parser SHALL use a hint that the positional index equals the logical index to optimize the common case
AND the parser SHALL return an error if no directory entry has a matching `index` field

---

### Requirement: Extract tagged entries by tag code

Within each WAD entry (the data region referenced by a directory entry), data is organized as a linked list of tagged chunks. Each chunk has a header containing a 4-byte tag code, and chunks are chained via the `next_offset` field.

#### Scenario: Reading entry headers in old format (version 0 or 1)

WHEN parsing tagged entries from a WAD entry in a version 0 or 1 file
THEN the parser SHALL read 12-byte old entry headers with the layout:
- `tag` (u32): four-character tag code (e.g., `EPNT`, `LINS`, `POLY`)
- `next_offset` (i32): byte offset from the start of the WAD entry data to the next entry header; 0 indicates last entry
- `length` (i32): byte length of the data payload following this header

AND the data payload SHALL immediately follow the 12-byte header

#### Scenario: Reading entry headers in new format (version 2+)

WHEN parsing tagged entries from a WAD entry in a version 2+ file
THEN the parser SHALL read entry headers of `entry_header_size` bytes (typically 16) with the layout:
- `tag` (u32): four-character tag code
- `next_offset` (i32): byte offset from the start of the WAD entry data to the next entry header; 0 indicates last entry
- `length` (i32): byte length of the data payload following this header
- `offset` (i32): offset for in-place expansion of data (used by overlays)

AND the data payload SHALL immediately follow the entry header

#### Scenario: Walking the tag chain

WHEN a WAD entry contains multiple tagged chunks
THEN the parser SHALL start at the first byte of the WAD entry data and read the first entry header
AND the parser SHALL follow the `next_offset` chain to locate subsequent entry headers
AND the `next_offset` value SHALL be interpreted as an absolute offset from the start of the WAD entry data (i.e., `directory_entry.offset_to_start + next_offset` in file coordinates)
AND the chain SHALL terminate when `next_offset` is 0

#### Scenario: Extracting data by tag code

WHEN a caller requests data for a specific 4-byte tag code from a parsed WAD entry
THEN the parser SHALL iterate through all tagged chunks in the entry
AND the parser SHALL return the data payload of the first chunk whose tag matches the requested code
AND the parser SHALL also return the length of the matched payload
AND the parser SHALL return an indication of "not found" if no chunk matches the requested tag code

#### Scenario: Enumerating all tags in an entry

WHEN a caller requests all tagged chunks from a WAD entry
THEN the parser SHALL return a collection of (tag, data, length) tuples for every chunk in the entry
AND the collection SHALL preserve the order in which chunks appear in the chain

#### Scenario: Truncated entry header

WHEN the WAD entry data is too short to contain a complete entry header at the current chain position
THEN the parser SHALL return an error indicating a malformed entry
AND the error SHALL identify the WAD entry index and the byte offset where the truncation occurred

#### Scenario: Entry data extends beyond WAD entry bounds

WHEN an entry header's `length` field would cause the data payload to extend beyond the WAD entry's total `length` (as specified in the directory entry)
THEN the parser SHALL return an error indicating the tag data is truncated
AND the error SHALL include the tag code, the claimed length, and the available bytes

#### Scenario: next_offset points outside WAD entry bounds

WHEN a `next_offset` value points to a location outside the WAD entry data region
THEN the parser SHALL return an error indicating an invalid chain pointer
AND the error SHALL include the tag code of the current entry and the out-of-bounds offset value

---

### Requirement: Handle overlay and patch WADs

A WAD file with a non-zero `parent_checksum` in its header is an overlay (patch) WAD. Overlay WADs contain modifications that are applied on top of a parent WAD file identified by checksum.

#### Scenario: Identifying an overlay WAD

WHEN a WAD header has `parent_checksum` not equal to 0
THEN the parser SHALL identify the file as an overlay/patch WAD
AND the parser SHALL expose the `parent_checksum` value so callers can locate the parent file

#### Scenario: Identifying a base WAD

WHEN a WAD header has `parent_checksum` equal to 0
THEN the parser SHALL identify the file as a base (non-overlay) WAD

#### Scenario: Matching parent by checksum

WHEN a caller needs to find the parent WAD for an overlay
THEN the parser SHALL provide a function that accepts a file and a checksum value
AND the function SHALL read the candidate file's header checksum
AND the function SHALL return whether the candidate file's `checksum` matches the provided value

#### Scenario: Overlay entry index field

WHEN an overlay WAD has version 2 or higher
THEN the `index` field in each directory entry SHALL identify which logical entry in the parent WAD is being replaced or modified
AND the `offset` field in entry headers SHALL indicate the byte offset for in-place data expansion within the parent entry

---

### Requirement: Validate checksums

The WAD file checksum is a CRC-32 computed over the entire file contents with the `checksum` header field temporarily set to zero during computation. The CRC-32 uses polynomial `0xEDB88320` (reflected/LSB-first form of the standard CRC-32 polynomial), initial value `0xFFFFFFFF`, and final XOR of `0xFFFFFFFF`.

#### Scenario: Computing the file checksum

WHEN validating a WAD file's checksum
THEN the parser SHALL read the stored `checksum` value from the header
AND the parser SHALL compute the CRC-32 of the entire file with the 4-byte checksum field (at header offset 68) treated as all zeros
AND the CRC-32 SHALL use polynomial `0xEDB88320`, initial value `0xFFFFFFFF`, and final XOR value `0xFFFFFFFF`
AND the parser SHALL compare the computed CRC-32 against the stored checksum

#### Scenario: Checksum matches

WHEN the computed CRC-32 equals the stored `checksum` value
THEN the parser SHALL indicate the checksum is valid

#### Scenario: Checksum mismatch

WHEN the computed CRC-32 does not equal the stored `checksum` value
THEN the parser SHALL return an error or warning indicating a checksum mismatch
AND the error SHALL include both the expected (stored) and actual (computed) checksum values

#### Scenario: Checksum of zero

WHEN the stored `checksum` field is 0
THEN the parser SHALL treat the file as having no checksum
AND checksum validation SHALL be skipped (a zero checksum indicates the file was not checksummed)

---

### Requirement: Report clear errors for malformed or truncated files

The parser SHALL produce specific, actionable error messages for all failure modes. Errors MUST include enough context for a caller to diagnose the problem without inspecting raw bytes.

#### Scenario: File too small for header

WHEN the input data is fewer than 128 bytes
THEN the parser SHALL return an error stating the file is too small to contain a WAD header
AND the error SHALL include the actual file size

#### Scenario: Invalid directory offset

WHEN the `directory_offset` field in the header is negative or points beyond the file size
THEN the parser SHALL return an error identifying the invalid directory offset
AND the error SHALL include the offset value and the file size

#### Scenario: Wad count is zero

WHEN the `wad_count` field in the header is 0
THEN the parser SHALL accept the file as a valid but empty WAD
AND the parser SHALL not attempt to read directory entries

#### Scenario: Negative wad count

WHEN the `wad_count` field in the header is negative
THEN the parser SHALL return an error indicating an invalid wad count

#### Scenario: Directory entry points to invalid data region

WHEN a directory entry's `offset_to_start` is negative or points beyond the file size
THEN the parser SHALL return an error identifying the invalid entry offset
AND the error SHALL include the directory entry index and the offset value

#### Scenario: Directory entry length exceeds file bounds

WHEN a directory entry's `offset_to_start + length` exceeds the file size
THEN the parser SHALL return an error indicating the entry data is truncated
AND the error SHALL include the directory entry index, the offset, the claimed length, and the file size

#### Scenario: Cyclic tag chain detection

WHEN following the `next_offset` chain within a WAD entry, a previously visited offset is encountered again
THEN the parser SHALL return an error indicating a cyclic tag chain
AND the error SHALL include the WAD entry index and the repeated offset

#### Scenario: Maximum directory entry count

WHEN the `wad_count` exceeds `MAXIMUM_DIRECTORY_ENTRIES_PER_FILE` (64)
THEN the parser SHOULD accept the file but MAY issue a warning
AND the parser SHALL NOT use the limit as a hard rejection criterion, since third-party tools may produce files exceeding this limit
