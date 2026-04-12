---
tags: [alephone, reference, file-formats, wad, shapes, sounds, physics]
---

# File Format Specifications

Marathon uses several binary file formats, all stored in big-endian byte order (Mac heritage). The Rust rebuild parses these in the `marathon-formats` crate using the `binrw` library.

## WAD Container Format

All Marathon data files (maps, physics, save games, films) use the WAD container format. A WAD file contains a header, a sequence of entries, and a directory.

### WAD Header (128 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 2 | i16 | version (0-4) |
| 2 | 2 | i16 | data_version |
| 4 | 64 | char[] | file_name (null-terminated) |
| 68 | 4 | u32 | checksum |
| 72 | 4 | i32 | directory_offset |
| 76 | 2 | i16 | wad_count (number of entries) |
| 78 | 2 | i16 | application_specific_directory_data_size |
| 80 | 2 | i16 | entry_header_size |
| 82 | 2 | i16 | directory_entry_base_size |
| 84 | 4 | u32 | parent_checksum (non-zero = overlay/patch) |
| 88 | 40 | -- | unused padding |

**Version History:**
- Version 0: Marathon 1 base format
- Version 1: Marathon 2 format (added entry_header_size, etc.)
- Version 2: Marathon Infinity format
- Version 4: Aleph One extensions

### Directory Structure

The directory starts at `directory_offset` and has `wad_count` entries. Each directory entry stores the offset and size of the corresponding WAD entry.

Base directory entry size is 8 bytes (offset: i32, size: i32), but may be larger based on `directory_entry_base_size` and `application_specific_directory_data_size`.

### WAD Entry (Tag Chunks)

Each WAD entry contains a sequence of tagged data chunks. Each chunk has:

| Size | Type | Field |
|------|------|-------|
| 4 | u32 | tag (FourCC identifier) |
| 4 | i32 | next_offset (offset to next chunk, or 0) |
| 4 | i32 | length (data length) |
| N | u8[] | data |

Tags are four-character codes stored as big-endian u32.

## Map Tags

Map WAD entries contain these tags (parsed in `marathon-formats/src/map.rs`):

### Geometry Tags

| Tag | FourCC | Record Size | Contents |
|-----|--------|-------------|----------|
| Endpoints | `EPNT` | 16 bytes | Vertex positions + flags |
| Points | `PNTS` | 4 bytes | Simple vertex positions (Marathon 1) |
| Lines | `LINS` | 32 bytes | Edges connecting endpoints |
| Sides | `SIDS` | 64 bytes | Wall texture assignments |
| Polygons | `POLY` | 128 bytes | Convex map regions |

### Endpoint (16 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 2 | i16 | flags |
| 2 | 2 | i16 | highest_adjacent_floor_height |
| 4 | 2 | i16 | lowest_adjacent_ceiling_height |
| 6 | 2 | i16 | vertex.x |
| 8 | 2 | i16 | vertex.y |
| 10 | 2 | i16 | transformed.x |
| 12 | 2 | i16 | transformed.y |
| 14 | 2 | i16 | supporting_polygon_index |

Coordinates are in Marathon world units (1024 = 1 WU).

### Line (32 bytes)

Key fields:
- `endpoint_indexes[2]`: i16 pair
- `flags`: u16 (bit 14 = solid, bit 9 = has transparent side)
- `clockwise/counterclockwise_polygon_owner`: i16 (-1 = none)
- `clockwise/counterclockwise_polygon_side_index`: i16

### Side (64 bytes)

Each side has three texture slots:
- **Primary texture**: Main wall texture
- **Secondary texture**: Used for split walls (lower section)
- **Transparent texture**: Visible-through section

Each texture slot is a `SideTexture`:
- `x0, y0`: i16 texture offset
- `texture`: ShapeDescriptor (u16)

`side_type` determines geometry:
- 0 = Full wall (floor to ceiling)
- 1 = High wall (adjacent ceiling to owner ceiling)
- 2 = Low wall (owner floor to adjacent floor)
- 3, 4 = Split wall (high + low + transparent middle)

### Polygon (128 bytes)

Key fields:
- `polygon_type`: i16 (Normal, Platform, Teleporter, MinorOuch, MajorOuch, etc.)
- `flags`: u16
- `permutation`: i16 (platform/teleport/sound index depending on type)
- `vertex_count`: i16 (typically 3-8)
- `endpoint_indexes[8]`: i16 array
- `line_indexes[8]`: i16 array
- `adjacent_polygon_indexes[8]`: i16 array (-1 = solid wall)
- `floor_texture, ceiling_texture`: ShapeDescriptor
- `floor_height, ceiling_height`: i16 (world units)
- `floor_lightsource_index, ceiling_lightsource_index`: i16
- `floor_origin, ceiling_origin`: WorldPoint2d (texture offsets)
- `media_index`: i16 (-1 = no liquid)
- `floor_transfer_mode, ceiling_transfer_mode`: i16

### Map Info (`Minf`, 88 bytes)

- `environment_code`: i16
- `physics_model_index`: i16
- `song_index`: i16
- `mission_flags`: i16
- `environment_flags`: i16
- `level_name`: 66-byte Pascal string

### Object Tags

| Tag | FourCC | Record Size | Contents |
|-----|--------|-------------|----------|
| Objects | `OBJS` | 16 bytes | Entity spawn points |
| Platforms | `plat` | 32 bytes | Moving floor/ceiling definitions |
| Media | `medi` | 32 bytes | Liquid definitions |
| Lights | `LITE` | 100 bytes | Light definitions (static format) |
| Annotations | `NOTE` | 72 bytes | Map editor annotations |
| Terminals | `term` | Variable | Terminal text data |
| Ambient Sounds | `ambi` | 16 bytes | Per-polygon ambient sound images |
| Random Sounds | `bonk` | 32 bytes | Per-polygon random sound triggers |
| Item Placement | `plac` | 12 bytes | Difficulty-based item spawn rules |

### Map Object (16 bytes)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 2 | i16 | object_type (0=monster, 2=item, 3=player, etc.) |
| 2 | 2 | i16 | index (monster/item type index) |
| 4 | 2 | i16 | facing (0-511, Marathon angle units) |
| 6 | 2 | i16 | polygon_index |
| 8 | 2 | i16 | location.x |
| 10 | 2 | i16 | location.y |
| 12 | 2 | i16 | location.z |
| 14 | 2 | u16 | flags |

## Physics Tags

Physics data can be embedded in map WAD entries or in separate physics files.

| Tag | FourCC | Record Size | Contents |
|-----|--------|-------------|----------|
| Player Physics | `PXpx` | 104 bytes | PhysicsConstants (walking/running) |
| Monster Physics | `MNpx` | 156 bytes | MonsterDefinition |
| Projectile Physics | `PRpx` | 48 bytes | ProjectileDefinition |
| Weapon Physics | `WPpx` | 134 bytes | WeaponDefinition |
| Effects Physics | `FXpx` | 14 bytes | EffectDefinition |

Marathon 1 variants use lowercase tags (`phys`, `mons`, `proj`, `weap`, `effe`).

### PhysicsConstants (104 bytes, 26 fields)

All fields are 32-bit fixed-point (16.16) stored big-endian, converted to f32 by dividing by 65536.

Key fields:
- `maximum_forward_velocity`, `maximum_backward_velocity`, `maximum_perpendicular_velocity`
- `acceleration`, `deceleration`, `airborne_deceleration`
- `gravitational_acceleration`, `terminal_velocity`
- `angular_acceleration`, `angular_deceleration`, `maximum_angular_velocity`
- `maximum_elevation`
- `step_delta`, `step_amplitude`
- `radius`, `height`, `dead_height`, `camera_height`

Angular fields are in Marathon angle units (512 = full circle). The Rust sim converts these to radians (multiply by TAU/512).

Typically 2 entries exist: index 0 = walking physics, index 1 = running physics.

### MonsterDefinition (156 bytes)

Key fields:
- `collection`, `stationary_shape`, `moving_shape`, etc.: rendering references
- `vitality`: i16 (hit points)
- `immunities`, `weaknesses`: u32 bitmasks
- `flags`: u32 (bit 1 = flies)
- `radius`, `height`: i16 (world units)
- `preferred_hover_height`: i16
- `visual_range`, `half_visual_arc`: perception parameters
- Melee and ranged attack definitions (AttackDefinition sub-structs)

### WeaponDefinition (134 bytes)

Key fields:
- `weapon_class`: i16 (melee, normal, dual-wield, twofisted, multipurpose)
- `idle_height`, `bob_amplitude`: visual parameters
- Primary and secondary trigger definitions (TriggerDefinition sub-structs)
- `rounds_per_magazine`, `reload_ticks`, `ready_ticks`

## Shapes File Format

The Shapes file contains all 2D graphics: wall textures, sprites, HUD elements, landscapes.

### File Layout

```
+----------------------------+
| 32 Collection Headers      |  1024 bytes (32 x 32 bytes each)
+----------------------------+
| Collection 0 data block    |  Variable size
| Collection 1 data block    |
| ...                        |
| Collection 31 data block   |
+----------------------------+
```

### Collection Header (32 bytes)

| Field | Type | Description |
|-------|------|-------------|
| status | i16 | Load status |
| flags | u16 | Collection flags |
| offset | i32 | Offset to 8-bit data (-1 = none) |
| length | i32 | Length of 8-bit data |
| offset16 | i32 | Offset to 16-bit data (-1 = none) |
| length16 | i32 | Length of 16-bit data |
| padding | 12 bytes | Unused |

### Collection Data Block

Each collection contains:
1. **Collection Definition**: version, type, color count, bitmap count, etc.
2. **Color Tables (CLUTs)**: 256 entries of 16-bit RGB values each
3. **High-Level Shapes (Sequences)**: Animation sequence definitions
4. **Low-Level Shapes (Frames)**: Individual frame definitions with bitmap references
5. **Bitmaps**: Raw pixel data (8-bit indexed into CLUT)

### Collection Types

| Type | Value | Bitmap Storage |
|------|-------|----------------|
| Unused | 0 | -- |
| Wall | 1 | Uncompressed |
| Object | 2 | RLE compressed |
| Interface | 3 | Uncompressed |
| Scenery | 4 | RLE compressed |

### Bitmap Data

- Pixels are 8-bit indices into the current CLUT
- Can be stored in column-major or row-major order (`column_order` flag)
- Transparent bitmaps: pixel index 0 = transparent
- RLE compression: for Object/Scenery types

### ShapeDescriptor (u16)

Packed reference to a specific shape:
```
Bits [15:13]: CLUT index (color table)
Bits [12:8]:  Collection index (0-31)
Bits [7:0]:   Shape index within collection
```

The `is_none()` check: descriptor == 0xFFFF means "no texture".

### Color Values

CLUT entries use 16-bit per channel (Marathon's Mac heritage):
```
struct ColorValue {
    flags: u8,
    value: u8,
    red: u16,    // 0-65535
    green: u16,
    blue: u16,
}
```

The Rust code converts to 8-bit: `(color.red >> 8) as u8`.

## Sounds File Format

### Sound File Header (260 bytes)

- `version`: i32
- `tag`: i32 (expected: 'snd2' = 0x736E6432)
- `source_count`: i16 (number of sources: typically 2 for 8-bit and 16-bit)
- `sound_count`: i16 (number of sound definitions)
- Padding: 248 bytes

### Sound Definition (64 bytes per definition)

Key fields:
- `sound_code`: i16 (engine sound code)
- `behavior_index`: i16 (Quiet=0, Normal=1, Loud=2)
- `flags`: u16 (SoundFlags bitfield)
- `chance`: u16 (probability of playing, 0xFFFF = always)
- `low_pitch`, `high_pitch`: fixed-point (random pitch range)
- `permutations`: i16 (number of sound variants)
- `group_offset`: i32 (offset to audio data in file)
- `single_length`: i32 (length of single permutation)
- `total_length`: i32 (total audio data length)

### Sound Flags

| Flag | Bit | Meaning |
|------|-----|---------|
| CANNOT_BE_RESTARTED | 0x01 | Don't restart if already playing |
| DOES_NOT_SELF_ABORT | 0x02 | Don't stop existing instance when re-triggered |
| RESISTS_PITCH_CHANGES | 0x04 | Limit pitch randomization |
| CANNOT_CHANGE_PITCH | 0x08 | No pitch variation at all |
| CANNOT_BE_OBSTRUCTED | 0x10 | Ignore wall obstruction |
| CANNOT_BE_MEDIA_OBSTRUCTED | 0x20 | Ignore liquid obstruction |
| IS_AMBIENT | 0x40 | Ambient sound (loops, follows listener) |

### Audio Data

Raw 8-bit unsigned mono PCM at 22050 Hz. Conversion to modern format:
```rust
let value = (sample as f32 - 128.0) / 128.0;  // 0-255 -> -1.0 to ~1.0
```

## MML (Marathon Markup Language)

XML-based configuration format used by Aleph One for overriding engine defaults. Parsed by `quick-xml` in `marathon-formats/src/mml.rs`.

Structure: `<marathon>` root with sections like `<opengl>`, `<interface>`, `<player>`, etc.

## Plugin Format

Plugins are WAD files that overlay the base scenario. They can patch:
- Shapes (`ShPa` tag)
- Sounds (`SnPa` tag)
- MML scripts (`MMLS` tag)
- Lua scripts (`LUAS` tag)

Plugin metadata includes scenario requirements for compatibility checking.

## Coordinate Systems and Units

| System | Unit | Scale |
|--------|------|-------|
| World coordinates | i16 | 1024 = 1 World Unit |
| Angles | i16 | 512 = full circle (360 degrees) |
| Fixed-point | i32 | 65536 = 1.0 (16.16 format) |
| Height | i16 | Same as world coordinates |
| Texture coordinates | i16 | 1024 = 1 texture repeat |

1 World Unit is approximately 2 meters in real-world scale.
