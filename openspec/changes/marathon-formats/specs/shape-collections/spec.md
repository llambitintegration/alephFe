## ADDED Requirements

### Requirement: Parse Collection Headers Array

A Shapes file begins with an array of exactly 32 collection headers (MAXIMUM_COLLECTIONS), each 32 bytes, read in big-endian byte order. Each header contains a status field, flags, an offset and length for 8-bit color data, and an offset16 and length16 for 16-bit color data, followed by 12 bytes of padding. An offset value of -1 indicates no data is present for that bit depth.

#### Scenario: Read all 32 collection headers from a valid Shapes file

- **WHEN** a Shapes file is opened for parsing
- **THEN** exactly 32 collection_header entries are read from the start of the file
- **AND** each entry consumes exactly 32 bytes (total: 1024 bytes for the full header array)
- **AND** the fields status (i16), flags (u16), offset (i32), length (i32), offset16 (i32), length16 (i32) are parsed in big-endian order

#### Scenario: Collection header indicates no data for a bit depth

- **WHEN** a collection_header has offset equal to -1
- **THEN** the parser treats that collection as having no 8-bit data available
- **AND** when offset16 equals -1, the parser treats that collection as having no 16-bit data available

#### Scenario: Collection header indicates available data

- **WHEN** a collection_header has a non-negative offset and a positive length
- **THEN** the parser records that collection data begins at that byte offset within the Shapes file
- **AND** the length field indicates the total byte size of that collection's data block

### Requirement: Parse Collection Definitions with Version Validation

Each loaded collection begins with a collection_definition structure of 544 bytes. The version field must equal 3 (COLLECTION_VERSION). The type field identifies the collection kind: unused (0), wall (1), object (2), interface (3), or scenery (4). The definition contains counts and offsets for color tables, high-level shapes, low-level shapes, and bitmaps, plus a pixels_to_world scale factor.

#### Scenario: Parse a valid collection definition

- **WHEN** the parser reads a collection_definition from the data block at a collection header's offset
- **THEN** it reads exactly 544 bytes in big-endian order
- **AND** it extracts version (i16), type (i16), flags (u16), color_count (i16), clut_count (i16), color_table_offset (i32), high_level_shape_count (i16), high_level_shape_offset_table_offset (i32), low_level_shape_count (i16), low_level_shape_offset_table_offset (i32), bitmap_count (i16), bitmap_offset_table_offset (i32), pixels_to_world (i16), and size (i32)
- **AND** the remaining 506 bytes (253 x i16) of unused padding are skipped

#### Scenario: Version validation rejects invalid version

- **WHEN** the parser reads a collection_definition whose version field is not 3
- **THEN** the parser returns an error indicating an unsupported collection version

#### Scenario: Collection type is preserved

- **WHEN** the parser reads the type field from a collection_definition
- **THEN** it stores the type as one of the recognized collection types: unused (0), wall (1), object (2), interface (3), or scenery (4)
- **AND** the type determines whether bitmaps use raw storage (wall, interface) or RLE compression (object, scenery)

### Requirement: Parse Color Tables (CLUTs)

Color tables are stored as contiguous arrays of rgb_color_value entries (8 bytes each). A collection contains clut_count CLUTs, each consisting of color_count entries. The total number of color entries is clut_count * color_count. Each rgb_color_value contains a flags byte (with bit 0x80 indicating self-luminescent), a value byte, and red/green/blue fields as big-endian u16 values.

#### Scenario: Parse color tables from a collection with multiple CLUTs

- **WHEN** a collection_definition has clut_count > 0 and color_count > 0
- **THEN** the parser seeks to color_table_offset (relative to the collection data start) and reads clut_count * color_count rgb_color_value entries
- **AND** each entry is 8 bytes: flags (u8), value (u8), red (u16 big-endian), green (u16 big-endian), blue (u16 big-endian)

#### Scenario: Identify self-luminescent colors

- **WHEN** an rgb_color_value has its flags byte with bit 0x80 set (SELF_LUMINESCENT_COLOR_FLAG)
- **THEN** the parser marks that color entry as self-luminescent

#### Scenario: Collection has no color tables

- **WHEN** a collection_definition has clut_count equal to 0 or color_count equal to 0
- **THEN** the parser produces an empty color table array for that collection

### Requirement: Parse High-Level Shape Definitions

High-level shapes define animation sequences. Each has a fixed 90-byte header (SIZEOF_high_level_shape_definition) followed by a variable-length array of low_level_shape_indexes. The number of indexes equals the actual view count (derived from the number_of_views field) multiplied by frames_per_view. High-level shapes are located via an offset table: an array of big-endian i32 offsets at high_level_shape_offset_table_offset, one per high_level_shape_count entry.

#### Scenario: Parse high-level shape offset table and definitions

- **WHEN** a collection has high_level_shape_count > 0
- **THEN** the parser seeks to high_level_shape_offset_table_offset (relative to collection data start) and reads high_level_shape_count big-endian i32 values as offsets
- **AND** for each offset, it seeks to that position (relative to collection data start) and reads the high-level shape definition

#### Scenario: Parse high-level shape header fields

- **WHEN** a high-level shape definition is read
- **THEN** the parser extracts type (i16), flags (u16), name (34 bytes, null-terminated string), number_of_views (i16), frames_per_view (i16), ticks_per_frame (i16), key_frame (i16), transfer_mode (i16), transfer_mode_period (i16), first_frame_sound (i16), key_frame_sound (i16), last_frame_sound (i16), pixels_to_world (i16), loop_frame (i16), and 14 unused i16 values
- **AND** all multi-byte fields are read in big-endian order

#### Scenario: Compute actual view count from number_of_views

- **WHEN** the number_of_views field is read from a high-level shape
- **THEN** the actual view count is determined as follows: values 1 (animated1) and 10 (unanimated) yield 1 view; values 3 (animated3to4) and 4 (animated4) yield 4 views; values 9 (animated3to5) and 11 (animated5) yield 5 views; values 2 (animated2to8), 5 (animated5to8), and 8 (animated8) yield 8 views
- **AND** any other value is used directly as the view count

#### Scenario: Read low-level shape index array

- **WHEN** the actual view count and frames_per_view are known for a high-level shape
- **THEN** the parser reads (actual_view_count * frames_per_view) big-endian i16 values as the low_level_shape_indexes array immediately following the fixed header

### Requirement: Parse Low-Level Shape Definitions

Low-level shapes define individual frames with spatial metadata. Each is exactly 36 bytes (SIZEOF_low_level_shape_definition). They are located via an offset table: an array of big-endian i32 offsets at low_level_shape_offset_table_offset, one per low_level_shape_count entry.

#### Scenario: Parse low-level shape offset table and definitions

- **WHEN** a collection has low_level_shape_count > 0
- **THEN** the parser seeks to low_level_shape_offset_table_offset (relative to collection data start) and reads low_level_shape_count big-endian i32 values as offsets
- **AND** for each offset, it seeks to that position (relative to collection data start) and reads the low-level shape definition

#### Scenario: Parse low-level shape fields

- **WHEN** a low-level shape definition is read
- **THEN** the parser extracts flags (u16), minimum_light_intensity (i32, fixed-point 16.16), bitmap_index (i16), origin_x (i16), origin_y (i16), key_x (i16), key_y (i16), world_left (i16), world_right (i16), world_top (i16), world_bottom (i16), world_x0 (i16), world_y0 (i16), and 4 unused i16 values
- **AND** all multi-byte fields are read in big-endian order

#### Scenario: Decode low-level shape flags

- **WHEN** the flags field of a low-level shape is parsed
- **THEN** bit 15 (0x8000) indicates X_MIRRORED
- **AND** bit 14 (0x4000) indicates Y_MIRRORED
- **AND** bit 13 (0x2000) indicates KEYPOINT_OBSCURED

### Requirement: Parse and Decompress Bitmap Data

Bitmaps have a 30-byte header (SIZEOF_bitmap_definition) followed by row/column address pointers (skipped during parsing) and pixel data. Bitmaps are located via an offset table at bitmap_offset_table_offset. The bytes_per_row field determines the storage format: when it equals -1 (NONE), the bitmap uses RLE compression; otherwise, it uses raw uncompressed storage. The flags field bit 15 (0x8000, _COLUMN_ORDER_BIT) indicates column-major storage order.

#### Scenario: Parse bitmap offset table and headers

- **WHEN** a collection has bitmap_count > 0
- **THEN** the parser seeks to bitmap_offset_table_offset (relative to collection data start) and reads bitmap_count big-endian i32 values as offsets
- **AND** for each offset, it seeks to that position and reads the bitmap header: width (i16), height (i16), bytes_per_row (i16), flags (i16), bit_depth (i16), 8 unused i16 values, then skips (row_count + 1) * 4 bytes of row address pointers
- **AND** row_count equals width when the COLUMN_ORDER flag (bit 15) is set, or height otherwise

#### Scenario: Read raw (uncompressed) bitmap data

- **WHEN** a bitmap header has bytes_per_row not equal to -1
- **THEN** the parser reads row_count * bytes_per_row bytes of raw pixel data following the row address pointers
- **AND** the pixel data represents 8-bit indexed color values

#### Scenario: Read RLE-compressed bitmap data

- **WHEN** a bitmap header has bytes_per_row equal to -1 (NONE)
- **THEN** the parser reads row_count scanlines of RLE-compressed data
- **AND** each scanline consists of a first (i16 big-endian) and last (i16 big-endian) pixel index, followed by (last - first) bytes of pixel data representing the non-transparent span
- **AND** pixels before first and at/after last are transparent (index 0)

#### Scenario: Column-order bitmap layout

- **WHEN** a bitmap has the COLUMN_ORDER flag (bit 15 of flags) set
- **THEN** the scanlines represent columns rather than rows
- **AND** row_count equals the bitmap width, and each scanline has a length derived from the bitmap height

### Requirement: Decode Shape Descriptor Packed Values

A shape_descriptor is a 16-bit packed value encoding a CLUT index, collection index, and shape index. Bits 0-7 (8 bits) encode the shape index within the collection. Bits 8-12 (5 bits) encode the collection index (0-31). Bits 13-15 (3 bits) encode the CLUT index (0-7).

#### Scenario: Extract fields from a shape descriptor

- **WHEN** a 16-bit shape_descriptor value is decoded
- **THEN** the shape index is extracted as bits 0-7 (value & 0xFF)
- **AND** the collection index is extracted as bits 8-12 ((value >> 8) & 0x1F)
- **AND** the CLUT index is extracted as bits 13-15 ((value >> 13) & 0x07)

#### Scenario: Build a shape descriptor from components

- **WHEN** a shape_descriptor is constructed from a collection index, CLUT index, and shape index
- **THEN** the resulting 16-bit value equals (clut << 13) | (collection << 8) | shape
- **AND** the collection index must be in range 0-31
- **AND** the CLUT index must be in range 0-7
- **AND** the shape index must be in range 0-255

#### Scenario: Extract combined collection from a shape descriptor

- **WHEN** the combined collection value is extracted (used for GET_DESCRIPTOR_COLLECTION)
- **THEN** it equals bits 8-15 of the descriptor ((value >> 8) & 0xFF), combining both collection and CLUT into a single byte

### Requirement: Handle Both 8-bit and 16-bit Collection Data Paths

Each collection header stores separate offset/length pairs for 8-bit and 16-bit color depth data. The parser must select the appropriate data path based on the requested bit depth, falling back to 8-bit if 16-bit data is not available.

#### Scenario: Load 8-bit collection data

- **WHEN** 8-bit color depth is requested or the collection header has offset16 equal to -1
- **THEN** the parser uses the offset and length fields from the collection header to locate the collection data
- **AND** parsing proceeds from that offset in the Shapes file

#### Scenario: Load 16-bit collection data

- **WHEN** 16-bit color depth is requested and the collection header has offset16 not equal to -1
- **THEN** the parser uses the offset16 and length16 fields from the collection header to locate the collection data
- **AND** parsing proceeds from that offset in the Shapes file

#### Scenario: Fall back from 16-bit to 8-bit

- **WHEN** 16-bit color depth is requested but offset16 equals -1
- **THEN** the parser falls back to using the 8-bit offset and length fields
- **AND** if the 8-bit offset is also -1, the collection is treated as unavailable
