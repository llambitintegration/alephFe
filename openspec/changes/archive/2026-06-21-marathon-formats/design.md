# marathon-formats: Design

## Context

This is a greenfield Rust crate in the `alephone-rust` workspace. There is no existing Rust code -- we are building from scratch. The C++ Aleph One codebase at `../alephone` is the reference implementation for all binary format details.

The crate's purpose is to parse every Marathon binary format into clean, typed Rust structs. It will be the foundation that all downstream crates (viewer, simulation, audio) depend on for data access. It must be publishable to crates.io with no internal-only dependencies.

Marathon uses big-endian byte order throughout and relies heavily on fixed-point arithmetic (16.16 format stored as `int32`, 10-bit fractional `world_distance` stored as `int16`). The crate will convert these to native Rust numeric types at the parse boundary so downstream consumers never deal with fixed-point.

## Goals

- Parse all Marathon binary formats correctly: WAD containers, map geometry, shapes/sprites, sounds, physics models, MML configuration, and plugin metadata
- Provide a clean, idiomatic Rust API with strong types and enums -- no raw integer soup
- Produce good error messages for malformed or truncated content
- Use zero-copy where practical (parse from `&[u8]` byte slices)
- Convert Marathon's integer and fixed-point values to Rust-native types (`f32`, `i16`, etc.) at the parse boundary

## Non-Goals

- Writing or serializing Marathon formats (read-only for now; `binrw` supports write for the future)
- Game logic, simulation, or rendering (this crate is pure data structures and parsing)
- Bit-identical reproduction of C++ parsing quirks or undefined behavior
- Supporting pre-Marathon 2 formats beyond what is needed for M1 compatibility tags (`m1_*` physics tags)

## Decisions

### 1. binrw for binary parsing

**Decision**: Use `binrw` for all binary format parsing.

**Alternatives considered**:
- `nom`: Combinator-based, excellent for streaming/network protocols, but produces verbose code for struct-heavy binary formats. No write support.
- Manual `byteorder` + `Read`: Maximum control but tedious and error-prone for the ~30 struct types we need to parse.

**Rationale**: `binrw` is declarative -- you annotate structs with `#[br(big)]` and field-level attributes for endianness, padding, counts, and conditionals. This maps naturally onto Marathon's format where every struct has a known fixed size with well-defined field layouts. It handles big-endian transparently. It also supports `BinWrite` which we can enable later for serialization without rewriting the data model. Example:

```rust
#[derive(BinRead, Debug)]
#[br(big)]
pub struct WadHeader {
    pub version: i16,
    pub data_version: i16,
    #[br(count = 64, map = |b: Vec<u8>| bytes_to_string(&b))]
    pub file_name: String,
    pub checksum: u32,
    pub directory_offset: i32,
    pub wad_count: i16,
    pub application_specific_directory_data_size: i16,
    pub entry_header_size: i16,
    pub directory_entry_base_size: i16,
    pub parent_checksum: u32,
    #[br(count = 20)]
    _unused: Vec<i16>,
}
```

### 2. Single crate with modules

**Decision**: Ship one crate (`marathon-formats`) with internal modules, not a workspace of micro-crates.

**Module layout**:
```
marathon-formats/
  src/
    lib.rs           // Re-exports, top-level WadFile API
    wad.rs           // WAD container: header, directory, entry parsing
    map.rs           // Map geometry: endpoints, lines, sides, polygons, etc.
    shapes.rs        // Shape collections: definitions, CLUTs, bitmaps, RLE
    sounds.rs        // Sound file header, definitions, permutations
    physics.rs       // Physics models: monster, effect, projectile, player, weapon
    mml.rs           // MML XML configuration parsing
    plugin.rs        // Plugin.xml metadata
    tags.rs          // Tag constants (four-character codes) and enum dispatch
    types.rs         // Shared types: WorldPoint2d, WorldPoint3d, DamageDefinition, etc.
    error.rs         // Error types
```

**Rationale**: Users of this library want "parse a Marathon scenario" -- they do not want to manually select and version-align 7 micro-crates. A single crate with modules keeps the dependency graph simple while still allowing internal modularity. The module boundaries are clean enough that splitting later (if needed) would be mechanical.

### 3. Fixed-point conversion at parse boundary

**Decision**: Convert all fixed-point values to `f32` during parsing. Store `f32` in the Rust struct fields. Do not expose Marathon's fixed-point representation in the public API.

**Marathon's numeric types**:
- `_fixed` (alias for `int32`): 16.16 fixed-point. `FIXED_ONE = 0x10000 = 65536`. Conversion: `value as f32 / 65536.0`.
- `world_distance` (alias for `int16`): 10-bit fractional. `WORLD_ONE = 1024`. These are direct `i16` values that represent world units; they remain as `i16` in the Rust structs since they are not floating-point encoded.
- `angle` (alias for `int16`): Full circle = 512 units. Conversion: `value as f32 * (2.0 * PI / 512.0)` to radians, or kept as raw `i16` with a newtype wrapper.

**Rationale**: Downstream code (renderers, simulation) works in floating-point. Forcing every consumer to manually convert fixed-point values is error-prone and leaks an implementation detail of 1990s Macintosh game engineering. The parse boundary is the natural place to normalize representations.

**Implementation**: Use binrw's `map` attribute for inline conversion:
```rust
#[br(map = |v: i32| fixed_to_f32(v))]
pub maximum_forward_velocity: f32,
```

Where `fixed_to_f32` is:
```rust
pub(crate) fn fixed_to_f32(v: i32) -> f32 {
    v as f32 / 65536.0
}
```

For `world_distance` fields, keep as `i16` -- they are already in a usable integer form (world units with 1024 = 1 world unit). Downstream can convert to `f32` if needed, but the integer form is canonical.

For `angle` fields, provide a newtype `MarathonAngle(i16)` with a `.to_radians() -> f32` method, keeping the raw value for simulation fidelity while making conversion easy.

### 4. Tag dispatch via enum

**Decision**: WAD tags are 4-byte codes (e.g., `PNTS`, `LINS`, `POLY`). Represent them as a Rust enum with known variants and an `Unknown(u32)` catch-all.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WadTag {
    // Map tags
    Points,          // b"PNTS"
    Lines,           // b"LINS"
    Sides,           // b"SIDS"
    Polygons,        // b"POLY"
    Lights,          // b"LITE"
    Annotations,     // b"NOTE"
    Objects,         // b"OBJS"
    MapInfo,         // b"Minf"
    Endpoints,       // b"EPNT"
    Platforms,       // b"plat"
    Media,           // b"medi"
    AmbientSounds,   // b"ambi"
    RandomSounds,    // b"bonk"
    Terminals,       // b"term"
    ItemPlacement,   // b"plac"
    // Physics tags
    MonsterPhysics,     // b"MNpx"
    EffectsPhysics,     // b"FXpx"
    ProjectilePhysics,  // b"PRpx"
    PlayerPhysics,      // b"PXpx"
    WeaponsPhysics,     // b"WPpx"
    // M1 Physics tags
    M1MonsterPhysics,    // b"mons"
    M1EffectsPhysics,    // b"effe"
    M1ProjectilePhysics, // b"proj"
    M1PlayerPhysics,     // b"phys"
    M1WeaponsPhysics,    // b"weap"
    // Embedded content
    ShapePatch,     // b"ShPa"
    SoundPatch,     // b"SnPa"
    MmlScript,      // b"MMLS"
    LuaScript,      // b"LUAS"
    // Save/game state tags (parsed as opaque for now)
    // ...
    Unknown(u32),
}
```

Implement `From<u32>` and `Into<u32>` for conversion. Use `FOUR_CHARS_TO_INT` logic (big-endian packing of 4 ASCII bytes into `u32`):
```rust
pub const fn four_chars(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}
```

Unknown tags are preserved as raw bytes in `TagData::Unknown { tag: u32, data: Vec<u8> }` so that downstream tools can round-trip WAD files without data loss.

### 5. Error handling

**Decision**: Use `thiserror` for typed errors. One top-level `ParseError` enum with nested variants.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("WAD error: {0}")]
    Wad(#[from] WadError),
    #[error("Map error: {0}")]
    Map(#[from] MapError),
    #[error("Shape error: {0}")]
    Shape(#[from] ShapeError),
    #[error("Sound error: {0}")]
    Sound(#[from] SoundError),
    #[error("Physics error: {0}")]
    Physics(#[from] PhysicsError),
    #[error("MML error: {0}")]
    Mml(#[from] MmlError),
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

Each sub-error provides context:
```rust
#[derive(Debug, thiserror::Error)]
pub enum WadError {
    #[error("invalid WAD header: expected 128 bytes, got {0}")]
    HeaderTooShort(usize),
    #[error("unsupported WAD version {0} (expected 0-4)")]
    UnsupportedVersion(i16),
    #[error("directory offset {offset} exceeds file size {file_size}")]
    DirectoryOutOfBounds { offset: i32, file_size: u64 },
    #[error("failed to parse entry with tag {tag:#010x}: {source}")]
    EntryParse { tag: u32, source: binrw::Error },
}
```

### 6. Shape bitmap decompression

**Decision**: Decompress RLE-encoded bitmaps at parse time. Store decompressed pixel data as `Vec<u8>`. Do not retain the compressed form.

**Background**: Marathon shapes use two bitmap storage modes determined by collection type:
- `_wall_collection` and `_interface_collection`: Raw pixel data, column-major order. `bytes_per_row` is set to the actual stride.
- `_object_collection` and `_scenery_collection`: RLE-compressed, column-major. `bytes_per_row == NONE (-1)` signals RLE encoding. The `_TRANSPARENT_BIT` flag (0x4000) indicates the bitmap has transparent pixels.

The bitmap header is 30 bytes (`SIZEOF_bitmap_definition`):
```
width: i16, height: i16, bytes_per_row: i16,
flags: i16 (bit 15 = column_order, bit 14 = transparent),
bit_depth: i16 (always 8),
unused: [i16; 8]
```

For RLE bitmaps, after the header come `width` column offset values (each `i32`), then RLE-encoded column data. Each column is a sequence of runs: a run-length byte followed by that many pixel bytes, with special handling for transparent gaps.

**Rust representation**:
```rust
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    pub column_order: bool,
    pub transparent: bool,
    pub pixels: Vec<u8>,  // width * height, decompressed, column-major
}
```

**Rationale**: Keeping compressed data forces every consumer to decompress. Since shape files are at most ~10-15 MB and decompressed bitmaps are typically small (sprites are 30x60 pixels, textures 128x128), memory cost is negligible. The decompressed form is what renderers, exporters, and editors all need.

### 7. API style

**Decision**: Two entry points per format -- file-based and byte-slice-based. Plus lower-level tag parsing functions.

**Top-level API**:
```rust
// File-based
let wad = WadFile::open("Map.sceA")?;

// In-memory
let wad = WadFile::from_bytes(&data)?;

// Access entries by index
let level = wad.entry(0)?;

// Extract typed data from an entry
let points: Vec<Endpoint> = level.parse_tag::<Vec<Endpoint>>(WadTag::Endpoints)?;
let polygons: Vec<Polygon> = level.parse_tag::<Vec<Polygon>>(WadTag::Polygons)?;
let map_info: MapInfo = level.parse_tag::<MapInfo>(WadTag::MapInfo)?;

// Or use the convenience method that parses all known map tags at once
let map = MapData::from_entry(&level)?;
```

**WadFile structure**:
```rust
pub struct WadFile {
    pub header: WadHeader,
    entries: Vec<WadEntry>,
}

pub struct WadEntry {
    pub index: i16,
    tags: Vec<RawTagData>,
}

pub struct RawTagData {
    pub tag: WadTag,
    pub data: Vec<u8>,
    pub offset: i32,
}
```

The WAD container is parsed eagerly (header + directory + all entry/tag boundaries), but individual tag data is only interpreted (into typed structs) on demand via `parse_tag`. This balances startup cost with memory: we read the full file into structured chunks but defer the expensive struct-level parsing until the caller asks for specific data.

**Shapes and Sounds files** have their own top-level types since they are not WAD-based:
```rust
let shapes = ShapesFile::open("Shapes")?;
let collection = shapes.collection(0)?;  // Returns CollectionDefinition

let sounds = SoundsFile::open("Sounds")?;
let header = sounds.header();
let definition = sounds.sound(42)?;
```

Note: Shapes files are actually stored as WAD-based files in some scenarios (embedded via `ShPa` tags), but the standalone Shapes file has its own layout with collection headers at fixed offsets. The parser handles both paths.

## Detailed Format Specifications

### WAD Container (wad.rs)

The WAD format is Marathon's universal container. Maps, physics, saves, and films all use it.

**File structure**:
```
[wad_header: 128 bytes]
[entry 0 data: variable]
  [entry_header + data for tag 0]
  [entry_header + data for tag 1]
  ...
[entry 1 data: variable]
  ...
[directory: wad_count * (directory_entry_base_size + application_specific_directory_data_size)]
```

**Version differences** (stored in `header.version`):
| Version | Constant | Directory entry | Entry header | Notes |
|---------|----------|-----------------|--------------|-------|
| 0 | `PRE_ENTRY_POINT_WADFILE_VERSION` | old: 8 bytes (offset, length) | old: 12 bytes (tag, next_offset, length) | No entry points, no directory index |
| 1 | `WADFILE_HAS_DIRECTORY_ENTRY` | new: 10 bytes (+ index field) | new: 16 bytes (+ offset field) | Added directory entry index |
| 2 | `WADFILE_SUPPORTS_OVERLAYS` | new: 10 bytes | new: 16 bytes | parent_checksum enables overlays |
| 4 | `WADFILE_HAS_INFINITY_STUFF` | new: 10 bytes | new: 16 bytes | Marathon Infinity format (current) |

The version field determines which struct layouts to use for directory entries and entry headers. Version 3 was skipped historically.

**Application-specific directory data**: For map files, each directory entry is followed by a `directory_data` struct (74 bytes) containing `mission_flags`, `environment_flags`, `entry_point_flags`, and `level_name`. The `application_specific_directory_data_size` header field indicates how many extra bytes follow each directory entry.

**Overlay WADs**: When `parent_checksum != 0`, the WAD is a patch/overlay. The engine finds the parent WAD by checksum and merges entries. Our parser exposes `parent_checksum` but does not perform merging (that is game logic).

### Map Geometry (map.rs)

Map data lives in WAD entries as tagged chunks. Each tag contains an array of fixed-size structs.

**Struct sizes** (from C++ `SIZEOF_*` constants):

| Tag | Code | Struct | Size | Key fields |
|-----|------|--------|------|------------|
| `EPNT` | Endpoints | `endpoint_data` | 16 bytes | flags, floor/ceiling heights, vertex (x,y), supporting polygon |
| `PNTS` | Points | `world_point2d` | 4 bytes | x, y (legacy format, EPNT preferred) |
| `LINS` | Lines | `line_data` | 32 bytes | 2 endpoint indices, flags, length, CW/CCW polygon + side indices |
| `SIDS` | Sides | `side_data` | 64 bytes | type, flags, 3 texture defs (primary/secondary/transparent), exclusion zone, control panel info, transfer modes, light indices |
| `POLY` | Polygons | `polygon_data` | 128 bytes | type, vertex/line/side/adjacent polygon arrays (max 8 each), floor/ceiling texture+height+light, transfer modes, media, center, sound indices |
| `OBJS` | Objects | `map_object` | 16 bytes | type (monster/object/item/player/goal/sound), index, facing, polygon, location (x,y,z), flags |
| `LITE` | Lights | `saved_static_light_data` | 100 bytes | type, flags, phase, 6 lighting function specs (14 bytes each: function, period, delta, intensity hi/lo/delta), tag |
| `plat` | Platforms | `static_platform_data` | 32 bytes | type, speed, delay, max/min height, flags (u32), polygon index, tag |
| `medi` | Media | `media_data` | 32 bytes | type, flags, light index, current direction/magnitude, low/high/height, origin, light intensity (fixed), texture, transfer mode |
| `NOTE` | Annotations | `map_annotation` | 72 bytes | type, location, polygon, 64-char text |
| `ambi` | Ambient sounds | `ambient_sound_image_data` | 16 bytes | flags, sound index, volume |
| `bonk` | Random sounds | `random_sound_image_data` | 32 bytes | flags, sound index, volume/delta, period/delta, direction/delta, pitch/delta (fixed-point) |
| `Minf` | Map info | `static_data` | 88 bytes | environment code, physics model, song index, mission/environment flags, level name (66 chars), entry point flags |
| `plac` | Item placement | `object_frequency_definition` | 12 bytes | flags, initial/min/max/random count, random chance |
| `term` | Terminals | variable | Variable | Header (total_length, grouping_count, font_count, text_length), then grouping structs, font change structs, then text |

**Element count**: The number of elements in a tag is `tag_data.length / sizeof(struct)`. The parser validates that `length % struct_size == 0` and returns a `MapError` if not.

**Polygon detail**: Polygons are the most complex map struct at 128 bytes. The `vertex_count` field (max 8) determines how many entries in the `endpoint_indexes`, `line_indexes`, `side_indexes`, and `adjacent_polygon_indexes` arrays are valid. Unused slots contain `NONE (-1)`.

### Shapes/Sprites (shapes.rs)

Shape files contain sprite and texture collections. The file is organized as a sequence of collection blocks.

**Collection structure**:
```
[collection_definition: 544 bytes (SIZEOF_collection_definition)]
  version: i16 (must be 3)
  type: i16 (0=unused, 1=wall, 2=object, 3=interface, 4=scenery)
  flags: u16
  color_count, clut_count: i16
  color_table_offset: i32
  high_level_shape_count: i16
  high_level_shape_offset_table_offset: i32
  low_level_shape_count: i16
  low_level_shape_offset_table_offset: i32
  bitmap_count: i16
  bitmap_offset_table_offset: i32
  pixels_to_world: i16
  size: i32  (total size for validation)
  unused: [i16; 253]
```

All offsets are relative to the start of the collection block.

**Offset tables**: High-level shapes, low-level shapes, and bitmaps are accessed via offset tables. Each table is an array of `i32` offsets pointing to the actual data within the collection block.

**Color tables (CLUTs)**: `clut_count` tables of `color_count` entries. Each `rgb_color_value` is 8 bytes: `flags (u8)`, `value (u8)`, `red (u16)`, `green (u16)`, `blue (u16)`. The first `NUMBER_OF_PRIVATE_COLORS (3)` entries in each CLUT are reserved.

**High-level shapes** (animations): 90 bytes base + variable-length `low_level_shape_indexes` array. Contains animation metadata: `number_of_views`, `frames_per_view`, `ticks_per_frame`, `key_frame`, transfer mode, and sound indices. The actual frame count depends on `number_of_views` (1, 2, 4, 5, or 8 views, where 5 and 8 use mirroring).

**Low-level shapes** (frames): 36 bytes each. Contains `bitmap_index` (into the collection's bitmap array), pixel-space origin/keypoint, world-space bounds, and flags (x-mirror, y-mirror, keypoint obscured).

**Bitmap data**: Header is 30 bytes (`SIZEOF_bitmap_definition`): width, height, bytes_per_row, flags, bit_depth, unused. For RLE bitmaps (`bytes_per_row == -1`), the header is followed by a column offset table (`width` entries of `i32`), then RLE-encoded column data.

**RLE decompression**: Each column consists of a sequence of runs. For transparent bitmaps, runs alternate between transparent runs (skip N pixels) and opaque runs (read N pixel bytes). The decompressor must handle column-major storage order.

Collection type determines RLE usage:
- Types 0 (`_unused_collection`) and 1 (`_wall_collection`): raw pixels
- Type 2 (`_object_collection`): RLE compressed
- Type 3 (`_interface_collection`): raw pixels
- Type 4 (`_scenery_collection`): RLE compressed

### Sounds (sounds.rs)

**Sound file header** (260 bytes, `SIZEOF_sound_file_header`):
```
version: i32    (must be 1, SOUND_FILE_VERSION)
tag: i32        (must be 0x736E6432 = 'snd2', SOUND_FILE_TAG)
source_count: i16   (usually 2: 8-bit and 16-bit)
sound_count: i16
unused: [i16; 124]
```

Immediately after the header: `source_count * sound_count` sound definitions.

**Sound definition** (64 bytes, `SIZEOF_sound_definition`):
```
sound_code: i16
behavior_index: i16     (0=quiet, 1=normal, 2=loud)
flags: u16              (restart, self-abort, pitch-resist, pitch-lock, obstruction flags)
chance: u16             (play probability; 0 = always)
low_pitch: i32 (fixed)  (0 means FIXED_ONE)
high_pitch: i32 (fixed) (0 means use low_pitch)
permutations: i16       (number of sound variants, max 5)
permutations_played: u16
group_offset: i32       (byte offset into audio data)
single_length: i32
total_length: i32
sound_offsets: [i32; 5]  (relative to group_offset)
last_played: u32
// ptr and size are runtime-only, not in file
```

The parser reads the definition metadata but does not decode the audio data itself (that belongs to the audio playback crate). It provides `group_offset` and `sound_offsets` so callers can extract raw audio bytes from the file.

### Physics Models (physics.rs)

Physics data is stored in WAD entries using five tag types. Each tag contains an array of fixed-size structs.

**physics_constants** (the player physics model): 26 `_fixed` fields = 104 bytes. All values are 16.16 fixed-point, converted to `f32` at parse time.

Fields in order:
```
maximum_forward_velocity, maximum_backward_velocity, maximum_perpendicular_velocity,
acceleration, deceleration, airborne_deceleration,
gravitational_acceleration, climbing_acceleration, terminal_velocity,
external_deceleration,
angular_acceleration, angular_deceleration, maximum_angular_velocity, angular_recentering_velocity,
fast_angular_velocity, fast_angular_maximum,
maximum_elevation,
external_angular_deceleration,
step_delta, step_amplitude,
radius, height, dead_height, camera_height, splash_height,
half_camera_separation
```

There are always `NUMBER_OF_PHYSICS_MODELS (2)` entries per `PXpx` tag: index 0 = walking, index 1 = running.

**Other physics tags**: Monster physics (`MNpx`), effect physics (`FXpx`), projectile physics (`PRpx`), and weapon physics (`WPpx`) contain arrays of their respective struct types. The exact struct layouts will be derived from the C++ `unpack_*` functions in the Aleph One source.

**M1 compatibility**: Marathon 1 used different tags (`mons`, `effe`, `proj`, `phys`, `weap`) with potentially different struct sizes. The parser checks for both M2/Infinity and M1 tag variants.

### MML Configuration (mml.rs)

MML (Marathon Markup Language) is XML that overrides engine parameters. Parsed with `quick-xml`.

```rust
pub struct MmlDocument {
    pub elements: Vec<MmlElement>,
}
```

MML parsing is deliberately shallow for the initial version: we parse the XML tree structure but do not deeply interpret every possible MML override (there are hundreds). The API provides access to elements and attributes so downstream crates can query what they need.

### Plugin Metadata (plugin.rs)

Plugin.xml files describe plugin contents. Parsed with `quick-xml`.

```rust
pub struct PluginMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub scenario_info: Option<ScenarioInfo>,
    pub mml_files: Vec<String>,
    pub shapes_patches: Vec<String>,
    pub sounds_patches: Vec<String>,
    pub lua_scripts: Vec<LuaScriptRef>,
    pub theme: Option<String>,
    pub map_patches: Vec<MapPatch>,
}

pub struct ScenarioInfo {
    pub name: String,
    pub id: Option<String>,
}

pub struct LuaScriptRef {
    pub path: String,
    pub script_type: LuaScriptType,  // Hud, Solo, Stats
}

pub struct MapPatch {
    pub path: String,
    pub parent_checksum: Option<u32>,
}
```

## Shared Types (types.rs)

Common types used across multiple modules:

```rust
/// World-space 2D point. Coordinates are in world units (i16).
pub struct WorldPoint2d {
    pub x: i16,
    pub y: i16,
}

/// World-space 3D point. Coordinates are in world units (i16).
pub struct WorldPoint3d {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// Shape descriptor: encodes collection index and shape index in a u16.
/// Bits [15:11] = collection (0-31), bits [10:8] = CLUT, bits [7:0] = shape index.
pub struct ShapeDescriptor(pub u16);

impl ShapeDescriptor {
    pub fn collection(&self) -> u8 { (self.0 >> 11) as u8 }
    pub fn clut(&self) -> u8 { ((self.0 >> 8) & 0x7) as u8 }
    pub fn shape_index(&self) -> u8 { (self.0 & 0xFF) as u8 }
    pub fn is_none(&self) -> bool { self.0 == 0xFFFF }
}

/// Texture definition used by sides (x/y offset + shape descriptor).
pub struct SideTexture {
    pub x0: i16,
    pub y0: i16,
    pub texture: ShapeDescriptor,
}

/// Damage definition used in physics and map data.
pub struct DamageDefinition {
    pub damage_type: i16,
    pub flags: i16,
    pub base: i16,
    pub random: i16,
    pub scale: f32,  // converted from _fixed
}
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `binrw` | 0.14+ | Binary format parsing with endianness support |
| `thiserror` | 2.x | Derive macro for error types |
| `bitflags` | 2.x | Type-safe bitflag fields (line flags, polygon flags, side flags, etc.) |
| `quick-xml` | 0.37+ | MML and Plugin.xml parsing |

**Dev dependencies**: `pretty_assertions` for test output, actual Marathon scenario files in `tests/fixtures/` (not shipped to crates.io).

## Risks and Trade-offs

### Undocumented format quirks
**Risk**: The C++ code has accumulated 30+ years of compatibility hacks, implicit assumptions, and undocumented format variations. Some community scenarios may rely on parser bugs.
**Mitigation**: Test against real Marathon 2, Marathon Infinity, and popular community scenario files (Rubicon, Phoenix, Eternal). Build a corpus of test WADs covering edge cases. When we find a quirk, document it in code comments referencing the C++ source location.

### Big-endian on little-endian hosts
**Risk**: Byte-swapping overhead on x86/ARM (all modern hosts).
**Mitigation**: `binrw` handles this transparently. The overhead is negligible -- these files are small (maps are typically <100KB, shapes <15MB) and parsed once at load time.

### Large shape files
**Risk**: Decompressing all bitmaps in a shapes file eagerly could use significant memory for total conversions with large sprite sheets.
**Mitigation**: The shapes parser decompresses bitmaps on access (when `collection()` is called), not when the file is first opened. Individual collections are typically 100KB-1MB decompressed. If memory becomes a concern, we can add a streaming/lazy mode later without changing the public API.

### WAD version compatibility
**Risk**: Versions 0 through 4 have different directory entry and entry header sizes. Misdetecting the version corrupts all subsequent parsing.
**Mitigation**: The version field is the first `i16` in the 128-byte header. We validate it immediately and branch on version for struct size selection. Test with WAD files from each version era.

### Fixed-point precision loss
**Risk**: Converting 16.16 fixed-point to `f32` loses precision in the low bits (f32 has 23 mantissa bits vs 32 total bits in fixed-point).
**Mitigation**: For the value ranges Marathon uses (world coordinates are typically -32768 to +32767 in the integer part), `f32` provides more than sufficient precision. The physics simulation in the original engine already operates at fixed-point granularity much coarser than the precision difference. If exact round-trip fidelity is ever needed (e.g., for a save-game writer), we can add a parallel `_raw` field or use `f64`.

### Terminal data complexity
**Risk**: Terminal data (`term` tag) has a complex variable-length format with embedded text, groupings, and font changes. It is not a simple array of fixed-size structs.
**Mitigation**: Parse the terminal header to extract counts, then parse groupings and font changes as fixed-size arrays, then extract the text body. The format is well-documented in `computer_interface.h` even if it requires more manual parsing than other tags.
