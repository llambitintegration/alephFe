# marathon-formats: Tasks

Implementation checklist for the `marathon-formats` crate. Tasks are ordered by dependency -- earlier sections must be completed before later ones can begin. Each task is scoped to be completable in a single session.

---

## 1. Project Setup

- [x] 1.1 Create `marathon-formats/` crate directory with `Cargo.toml` (edition 2021, lib crate) and add it to the workspace `Cargo.toml`
- [x] 1.2 Add dependencies: `binrw` (0.14+), `thiserror` (2.x), `bitflags` (2.x), `quick-xml` (0.37+)
- [x] 1.3 Add dev-dependencies: `pretty_assertions`
- [x] 1.4 Create module file stubs: `lib.rs`, `wad.rs`, `map.rs`, `shapes.rs`, `sounds.rs`, `physics.rs`, `mml.rs`, `plugin.rs`, `tags.rs`, `types.rs`, `error.rs`
- [x] 1.5 Set up `lib.rs` with module declarations and public re-exports; verify `cargo check` passes

## 2. Core Types & Error Handling

- [x] 2.1 Implement shared types in `types.rs`: `WorldPoint2d` (two `i16`), `WorldPoint3d` (three `i16`), `SideTexture` (x0, y0, texture), `DamageDefinition` (type, flags, base, random, scale as fixed-to-f32)
- [x] 2.2 Implement `ShapeDescriptor` newtype (`u16`) with methods: `collection()` (bits 8-12), `clut()` (bits 13-15), `shape_index()` (bits 0-7), `is_none()`, and `From`/`Into` conversions
- [x] 2.3 Implement `MarathonAngle` newtype (`i16`) with `to_radians() -> f32` and `to_degrees() -> f32` methods
- [x] 2.4 Implement fixed-point conversion helpers: `fixed_to_f32(i32) -> f32` (divide by 65536.0), `world_distance_to_f32(i16) -> f32` (divide by 1024.0)
- [x] 2.5 Implement `four_chars(a, b, c, d) -> u32` constant function for four-character code packing
- [x] 2.6 Implement `WadTag` enum in `tags.rs` with all known tag variants (map, physics, M1 physics, embedded content) plus `Unknown(u32)`; implement `From<u32>` and `Into<u32>`
- [x] 2.7 Implement error types in `error.rs`: top-level `ParseError` with `From` conversions, plus `WadError`, `MapError`, `ShapeError`, `SoundError`, `PhysicsError`, `MmlError`, `PluginError` sub-error enums with contextual fields
- [x] 2.8 Write unit tests for `ShapeDescriptor` field extraction, `fixed_to_f32` conversion (0x10000 -> 1.0, 0xFFFF0000 -> -1.0, 0x8000 -> 0.5), `MarathonAngle` conversion, and `four_chars` packing

## 3. WAD Format

- [x] 3.1 Implement `WadHeader` struct with `#[derive(BinRead)]` and `#[br(big)]`: all 128-byte header fields, `file_name` as null-terminated string from 64-byte buffer
- [x] 3.2 Implement WAD version detection and validation: accept versions 0-4, return `WadError::UnsupportedVersion` for others
- [x] 3.3 Implement old directory entry parsing (8 bytes: offset_to_start, length) for versions 0-1
- [x] 3.4 Implement new directory entry parsing (10+ bytes: offset_to_start, length, index) for versions 2+; use `directory_entry_base_size` from header
- [x] 3.5 Implement application-specific directory data reading: parse raw bytes of `application_specific_directory_data_size` per entry; for map files, parse the 74-byte `directory_data` struct (mission_flags, environment_flags, entry_point_flags, level_name)
- [x] 3.6 Implement old entry header parsing (12 bytes: tag, next_offset, length) for versions 0-1
- [x] 3.7 Implement new entry header parsing (16 bytes: tag, next_offset, length, offset) for versions 2+
- [x] 3.8 Implement tag chain walking: follow `next_offset` links within a WAD entry to enumerate all tagged chunks; detect cycles and out-of-bounds offsets
- [x] 3.9 Implement `WadFile` struct with `open(path)` and `from_bytes(&[u8])` constructors; eagerly parse header + directory + tag boundaries; store `RawTagData` per entry
- [x] 3.10 Implement `WadEntry` tag extraction: `get_tag_data(WadTag) -> Option<&[u8]>`, `all_tags() -> &[RawTagData]`, and `parse_tag::<T>(WadTag) -> Result<T>`
- [x] 3.11 Implement overlay WAD identification: expose `parent_checksum`, `is_overlay()` method, and checksum matching helper
- [x] 3.12 Implement CRC-32 checksum validation: polynomial `0xEDB88320`, initial `0xFFFFFFFF`, final XOR `0xFFFFFFFF`; zero checksum field during computation; skip validation when stored checksum is 0
- [x] 3.13 Implement error reporting for all WAD failure modes: truncated header, invalid directory offset, directory out of bounds, entry data out of bounds, cyclic tag chain, negative wad_count
- [x] 3.14 Write unit tests for WAD header parsing, version branching, directory reading, tag chain walking, checksum validation, and error cases (truncated, invalid offsets, cyclic chains)

## 4. Map Geometry

- [x] 4.1 Implement `Endpoint` struct from EPNT tag (16 bytes: flags, heights, vertex, transformed, supporting_polygon_index) with `BinRead`; implement legacy PNTS fallback (4 bytes: x, y with default fields)
- [x] 4.2 Implement PNTS-over-EPNT precedence logic: when both tags are present, use PNTS data
- [x] 4.3 Implement `Line` struct from LINS tag (32 bytes) with `BinRead`; implement `LineFlags` bitflags (solid, transparent, landscape, elevation, variable_elevation, has_transparent_side, decorative)
- [x] 4.4 Implement `Side` struct from SIDS tag (64 bytes) with `BinRead` using `SideTexture` for the three texture definitions; implement `SideFlags` bitflags (control_panel_status, is_control_panel, is_repair_switch, etc.)
- [x] 4.5 Implement `Polygon` struct from POLY tag (128 bytes) with `BinRead`; implement `PolygonType` enum (normal through superglue, 24 variants); handle 8-element index arrays with `vertex_count` validity
- [x] 4.6 Implement `MapObject` struct from OBJS tag (16 bytes) with `BinRead`; implement `MapObjectType` enum (monster, scenery, item, player, goal, sound_source); implement `MapObjectFlags` bitflags
- [x] 4.7 Implement `StaticLightData` struct from LITE tag (100 bytes) with `BinRead`; implement `LightingFunctionSpec` sub-struct (14 bytes with fixed-point intensity fields); implement old M1 light format (32 bytes)
- [x] 4.8 Implement `StaticPlatformData` struct from plat tag (32 bytes) with `BinRead`; implement platform flags as `u32` bitflags
- [x] 4.9 Implement `MediaData` struct from medi tag (32 bytes) with `BinRead`; implement media type enum (water, lava, goo, sewage, jjaro); convert fixed-point light intensity
- [x] 4.10 Implement `MapAnnotation` struct from NOTE tag (72 bytes) with `BinRead`; handle 64-byte null-terminated text field
- [x] 4.11 Implement terminal data parsing from term tag: parse `StaticPreprocessedTerminalData` header (10 bytes), grouping records (12 bytes each), font change records (6 bytes each), and text body; handle variable-length format
- [x] 4.12 Implement `AmbientSoundImage` struct from ambi tag (16 bytes) and `RandomSoundImage` struct from bonk tag (32 bytes) with `BinRead`; convert fixed-point pitch/delta fields
- [x] 4.13 Implement `MapInfo` struct from Minf tag (88 bytes) with `BinRead`; implement `MissionFlags`, `EnvironmentFlags`, and `EntryPointFlags` bitflags; handle 66-byte level name
- [x] 4.14 Implement `ObjectFrequencyDefinition` struct from plac tag (12 bytes) with `BinRead`
- [x] 4.15 Implement guard path stub from `p\x8Cth` tag: parse as opaque byte data
- [x] 4.16 Implement `MapData::from_entry(WadEntry)` convenience method that parses all known map tags from a WAD entry into a single `MapData` struct with optional fields
- [x] 4.17 Implement tag data length validation: verify `tag_length % struct_size == 0` for all fixed-size map tags; return `MapError` on mismatch
- [x] 4.18 Implement cross-reference validation: check polygon endpoint/line/side/adjacent references, line endpoint/polygon references, side polygon/line back-references, object polygon references; collect errors without aborting; make validation opt-in
- [x] 4.19 Write unit tests for each map struct parser (construct minimal binary payloads, verify field values, flag decoding, type discrimination, and error cases)

## 5. Shape Collections

- [ ] 5.1 Implement collection header array parsing: read 32 `CollectionHeader` entries (32 bytes each, total 1024 bytes) from start of Shapes file; handle offset=-1 as "no data"
- [ ] 5.2 Implement 8-bit vs 16-bit data path selection: use offset/length for 8-bit, offset16/length16 for 16-bit; fall back from 16-bit to 8-bit when offset16 is -1
- [ ] 5.3 Implement `CollectionDefinition` struct parsing (544 bytes): version (must be 3), type, flags, counts/offsets for CLUTs/high-level/low-level/bitmaps, pixels_to_world, size; validate version field
- [ ] 5.4 Implement collection type enum: unused (0), wall (1), object (2), interface (3), scenery (4); store type to determine RLE vs raw bitmap decoding
- [ ] 5.5 Implement CLUT parsing: read `clut_count * color_count` rgb_color_value entries (8 bytes each: flags, value, r, g, b as u16); identify self-luminescent colors (flag 0x80)
- [ ] 5.6 Implement high-level shape offset table parsing and high-level shape definition parsing: 90-byte fixed header (type, flags, name, number_of_views, frames_per_view, ticks_per_frame, key_frame, transfer_mode, sounds, loop_frame) plus variable-length `low_level_shape_indexes` array
- [ ] 5.7 Implement actual view count computation from `number_of_views` field: map animated1/unanimated to 1, animated3to4/animated4 to 4, animated3to5/animated5 to 5, animated2to8/animated5to8/animated8 to 8
- [ ] 5.8 Implement low-level shape offset table parsing and low-level shape definition parsing (36 bytes each): flags (x_mirror 0x8000, y_mirror 0x4000, keypoint_obscured 0x2000), minimum_light_intensity (fixed-point), bitmap_index, origin, key, world bounds
- [ ] 5.9 Implement bitmap header parsing (30 bytes): width, height, bytes_per_row, flags (column_order bit 15, transparent bit 14), bit_depth; skip row/column address pointers ((row_count + 1) * 4 bytes)
- [ ] 5.10 Implement raw (uncompressed) bitmap data reading for wall and interface collections: read `row_count * bytes_per_row` bytes of 8-bit indexed pixel data
- [ ] 5.11 Implement RLE bitmap decompression for object and scenery collections: read per-scanline first/last pixel indices (i16 each), fill transparent regions with index 0, read opaque pixel spans; handle column-major storage
- [ ] 5.12 Implement `Bitmap` output struct with decompressed pixel data: `width`, `height`, `column_order`, `transparent`, `pixels: Vec<u8>`
- [ ] 5.13 Implement `ShapesFile` struct with `open(path)` and `from_bytes(&[u8])` constructors; implement `collection(index) -> CollectionDefinition` accessor that parses on demand
- [ ] 5.14 Implement `ShapeDescriptor` builder: `from_parts(collection, clut, shape_index) -> ShapeDescriptor` with range validation (collection 0-31, clut 0-7, shape 0-255)
- [ ] 5.15 Write unit tests for collection header parsing, collection definition version validation, CLUT parsing, view count computation, low-level shape flag decoding, RLE decompression (including edge cases: fully transparent columns, fully opaque columns, single-pixel spans)

## 6. Sound Formats

- [ ] 6.1 Implement `SoundFileHeader` struct (260 bytes) with `BinRead`: version (i32), tag (i32, must be 'snd2' = 0x736E6432), source_count (i16), sound_count (i16), 248 bytes padding; validate tag and version
- [ ] 6.2 Implement legacy layout fallback: when sound_count is 0 and source_count > 0, treat source_count as sound_count and set source_count to 1
- [ ] 6.3 Implement `SoundDefinition` struct (64 bytes) with `BinRead`: sound_code, behavior_index, flags, chance, low/high pitch (fixed-point), permutations, permutations_played, group_offset, single_length, total_length, sound_offsets[5]; skip runtime-only fields (last_played, ptr, size)
- [ ] 6.4 Implement `SoundBehavior` enum (Quiet=0, Normal=1, Loud=2) and `SoundFlags` bitflags (cannot_be_restarted 0x01, does_not_self_abort 0x02, resists_pitch_changes 0x04, cannot_change_pitch 0x08, cannot_be_obstructed 0x10, cannot_be_media_obstructed 0x20, is_ambient 0x40)
- [ ] 6.5 Implement sound definition array reading: parse `source_count * sound_count` definitions after header; organize by (source_index, sound_index)
- [ ] 6.6 Implement permutation metadata accessors: valid permutation count from `permutations` field, individual permutation byte lengths computed from offset differences, out-of-range permutation index error
- [ ] 6.7 Implement audio data extraction: seek to `group_offset + sound_offsets[i]` for a given permutation; return raw audio bytes; return error if offset exceeds file bounds
- [ ] 6.8 Implement `SoundsFile` struct with `open(path)` and `from_bytes(&[u8])` constructors; implement `header()`, `sound(index)`, and `audio_data(sound_index, permutation_index)` accessors
- [ ] 6.9 Implement graceful handling of empty/unused slots: sound_code=-1, permutations=0, zero group_offset with zero total_length
- [ ] 6.10 Write unit tests for header validation (valid tag, invalid tag, invalid version, negative counts), sound definition parsing, behavior/flag decoding, permutation length computation, empty slot handling, truncated file error

## 7. Physics Formats

- [ ] 7.1 Implement `PhysicsConstants` struct (104 bytes, 26 fixed-point fields) with `BinRead`; use `#[br(map = |v: i32| fixed_to_f32(v))]` for all 26 fields; verify field order matches spec
- [ ] 7.2 Implement `MonsterDefinition` struct (156 bytes) with `BinRead`: all fields including embedded `DamageDefinition` (shrapnel_damage), two embedded `AttackDefinition` (melee/ranged, 16 bytes each), shape descriptors, sound indices, fixed-point fields
- [ ] 7.3 Implement `AttackDefinition` sub-struct (16 bytes): type, repetitions, error (angle), range (world_distance), attack_shape, dx, dy, dz
- [ ] 7.4 Implement `ProjectileDefinition` struct (48 bytes) with `BinRead`: collection, shape, effects, contrails, radius, area_of_effect, embedded `DamageDefinition`, flags (u32), speed, maximum_range, sound_pitch (fixed-point), sounds
- [ ] 7.5 Implement `EffectDefinition` struct (14 bytes) with `BinRead`: collection, shape, sound_pitch (fixed-point), flags (u16), delay, delay_sound
- [ ] 7.6 Implement `WeaponDefinition` struct (134 bytes) with `BinRead`: item_type, powerup_type, weapon_class, flags, fixed-point visual fields, shapes, timing ticks, plus two embedded `TriggerDefinition` sub-structs (38 bytes each)
- [ ] 7.7 Implement `TriggerDefinition` sub-struct (38 bytes): rounds_per_magazine, ammunition_type, timing fields, recoil, 6 sound indices, projectile_type, theta_error, dx, dz, shell_casing_type, burst_count, sound_activation_range
- [ ] 7.8 Implement M2/Infinity vs M1 tag dispatch: check for `MNpx`/`FXpx`/`PRpx`/`PXpx`/`WPpx` first, then fall back to `mons`/`effe`/`proj`/`phys`/`weap`; M2 tags take precedence when both are present
- [ ] 7.9 Implement M1 `phys` tag handling: skip the first 100-byte editor record before parsing player physics constants
- [ ] 7.10 Implement `PhysicsData` aggregate struct with optional fields for all five physics types; implement `PhysicsData::from_entry(WadEntry)` that parses whichever tags are present
- [ ] 7.11 Implement tag data length validation: verify `tag_length % record_size == 0` for each physics tag; return `PhysicsError` on mismatch
- [ ] 7.12 Write unit tests for player physics fixed-point conversion, monster definition parsing, projectile/effect/weapon parsing, M1 vs M2 tag selection, partial-content handling (missing tags), and length validation errors

## 8. MML Configuration

- [ ] 8.1 Implement MML document parser using `quick-xml`: validate `<marathon>` root element; return error with element name for wrong root; handle empty `<marathon/>` document
- [ ] 8.2 Define `MmlDocument` struct with optional typed fields for each recognized section: stringset, interface, motion_sensor, overhead_map, infravision, animated_textures, control_panels, platforms, liquids, sounds, faders, player, weapons, items, monsters, scenery, landscapes, texture_loading, opengl, software, dynamic_limits, scenario, console, logging
- [ ] 8.3 Implement section parsing: for each recognized child element of `<marathon>`, parse into a corresponding section struct preserving attributes and nested elements; silently ignore unrecognized elements
- [ ] 8.4 Implement MML layering: `MmlDocument::layer(base, overlay) -> MmlDocument` where present sections in overlay replace corresponding sections in base; absent sections in overlay preserve base values
- [ ] 8.5 Implement embedded MML extraction from WAD MMLS tags: extract chunk data, strip trailing null bytes, parse as MML XML
- [ ] 8.6 Implement MML error reporting with source context: include file path or WAD entry identifier, byte offset or line number from `quick-xml` errors
- [ ] 8.7 Write unit tests for valid document parsing, wrong root element, empty document, section presence/absence, layering (override, add, preserve), embedded extraction with null stripping, and malformed XML error messages

## 9. Plugin Metadata

- [ ] 9.1 Implement `PluginMetadata` struct and `Plugin.xml` root parser using `quick-xml`: extract name (required), description, version, minimum_version, auto_enable (default true), theme_dir from `<plugin>` attributes
- [ ] 9.2 Implement scenario requirement parsing: parse `<scenario>` child elements with name (max 31 chars), id (max 23 chars), version (max 7 chars) with truncation; skip elements missing both name and id
- [ ] 9.3 Implement MML reference parsing: collect `<mml file="..."/>` elements, sort paths alphabetically
- [ ] 9.4 Implement Lua script reference parsing: parse hud_lua attribute, `<solo_lua>` element with `<write_access>` children (6 flag types: world, fog, music, overlays, ephemera, sound), stats_lua attribute; handle legacy solo_lua attribute fallback; enforce single `<solo_lua>` element
- [ ] 9.5 Implement `SoloLuaWriteAccess` bitflags: world (0x01), fog (0x02), music (0x04), overlays (0x08), ephemera (0x10), sound (0x20); default to world when no write_access children
- [ ] 9.6 Implement shapes/sounds patch parsing: `<shapes_patch file="..." requires_opengl="..."/>` with default requires_opengl=false; `<sounds_patch file="..."/>`
- [ ] 9.7 Implement map patch parsing: `<map_patch>` with `<checksum>` children (u32 set) and `<resource type="..." id="..." data="..."/>` children; convert 4-char type to u32 via Mac Roman encoding; skip patches with no checksums or no resources; skip resources with type not exactly 4 bytes
- [ ] 9.8 Implement theme_dir handling: when theme_dir is set, clear hud_lua, solo_lua, shapes_patches, sounds_patches, and map_patches
- [ ] 9.9 Implement plugin directory discovery: recursively scan directories for Plugin.xml files; skip dot-prefixed directories; handle ZIP archives containing Plugin.xml
- [ ] 9.10 Implement plugin load ordering: sort by name alphabetically; resolve exclusive resources (HUD Lua, stats Lua, theme, solo Lua write access) by last-wins from sorted order
- [ ] 9.11 Implement graceful error handling: skip unparseable Plugin.xml files, ignore unrecognized elements/attributes, clear references to missing files (hud_lua, solo_lua, stats_lua, MML, shapes/sounds patches)
- [ ] 9.12 Write unit tests for Plugin.xml parsing (all field extraction, missing name rejection, auto_enable default), scenario requirement truncation, MML sorting, Lua write access flag parsing, map patch resource type conversion, theme_dir clearing, and malformed XML handling

## 10. Integration Testing

- [ ] 10.1 Create `tests/fixtures/` directory; document how to obtain Marathon 2 and Marathon Infinity data files for testing (do not commit copyrighted files; use `.gitignore` to exclude fixtures)
- [ ] 10.2 Write integration tests for WAD parsing against real Marathon 2 map files: verify header fields, directory entry count, tag enumeration, and level name extraction
- [ ] 10.3 Write integration tests for WAD parsing against Marathon Infinity map files: verify version 4 format, directory entries with index fields, and application-specific directory data
- [ ] 10.4 Write integration tests for map geometry: parse a complete level from a Marathon 2/Infinity WAD, verify endpoint/line/polygon counts, spot-check field values, run cross-reference validation
- [ ] 10.5 Write integration tests for shapes parsing: load a real Shapes file, verify collection header array (32 entries), parse at least one collection, verify CLUT colors, verify bitmap dimensions and decompressed pixel data
- [ ] 10.6 Write integration tests for sounds parsing: load a real Sounds file, verify header (tag 'snd2', source_count, sound_count), parse sound definitions, verify permutation offsets and lengths
- [ ] 10.7 Write integration tests for physics parsing: load a real Physics file as WAD, verify player physics constants (walking/running models), verify monster/weapon/projectile/effect record counts
- [ ] 10.8 Write integration tests for MML parsing: parse sample MML files from community scenarios; verify section detection and layering behavior
- [ ] 10.9 Write integration tests for plugin metadata: parse sample Plugin.xml files; verify scenario requirements, resource references, and load ordering
- [ ] 10.10 Write integration tests against at least one community scenario (e.g., Rubicon, Phoenix, or Eternal) to verify cross-format compatibility: WAD + shapes + sounds + physics + MML + plugins all parse without errors
