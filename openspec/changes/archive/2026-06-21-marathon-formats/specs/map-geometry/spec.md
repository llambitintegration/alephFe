# Spec: map-geometry

Parsing Marathon map geometry and supplementary data from WAD tag entries into well-typed Rust structures.

---

## ADDED Requirements

### Requirement: Parse endpoint data from WAD entries

The parser SHALL extract vertex data from either the legacy `PNTS` tag (four-character code `PNTS`, 4 bytes per record) or the extended `EPNT` tag (four-character code `EPNT`, 16 bytes per record).

#### Scenario: Parse legacy PNTS point data

WHEN a WAD entry contains a `PNTS` tag (`0x504E5453`)
THEN the parser SHALL interpret each 4-byte record as a `world_point2d` consisting of two big-endian `i16` fields: `x` and `y`.
AND the parser SHALL derive endpoint structures from these points with default values for flags (0), highest_adjacent_floor (0), lowest_adjacent_ceiling (0), and supporting_polygon_index (NONE / -1).
AND the total tag data length MUST be evenly divisible by 4.

#### Scenario: Parse extended EPNT endpoint data

WHEN a WAD entry contains an `EPNT` tag (`0x45504E54`) and no `PNTS` tag
THEN the parser SHALL interpret each 16-byte record as an `endpoint_data` structure with the following big-endian fields in order:
- `flags`: `u16`
- `highest_adjacent_floor_height`: `i16` (world_distance)
- `lowest_adjacent_ceiling_height`: `i16` (world_distance)
- `vertex`: `world_point2d` (two `i16` fields: `x`, `y`)
- `transformed`: `world_point2d` (two `i16` fields, runtime-only, parsed but not used)
- `supporting_polygon_index`: `i16`

AND the total tag data length MUST be evenly divisible by 16.

#### Scenario: PNTS tag takes precedence over EPNT

WHEN a WAD entry contains both a `PNTS` tag and an `EPNT` tag
THEN the parser SHALL use the `PNTS` data and ignore the `EPNT` tag, matching the original engine behavior.

---

### Requirement: Parse line data from WAD entries

The parser SHALL extract line (edge) definitions from the `LINS` tag (four-character code `LINS`, `0x4C494E53`).

#### Scenario: Parse LINS line records

WHEN a WAD entry contains a `LINS` tag
THEN the parser SHALL interpret each 32-byte record as a `line_data` structure with the following big-endian fields in order:
- `endpoint_indexes`: two `i16` values (start and end vertex)
- `flags`: `u16`
- `length`: `i16` (world_distance)
- `highest_adjacent_floor`: `i16` (world_distance)
- `lowest_adjacent_ceiling`: `i16` (world_distance)
- `clockwise_polygon_side_index`: `i16`
- `counterclockwise_polygon_side_index`: `i16`
- `clockwise_polygon_owner`: `i16`
- `counterclockwise_polygon_owner`: `i16`
- `unused`: six `i16` values (parsed and discarded)

AND the total tag data length MUST be evenly divisible by 32.

#### Scenario: Decode line flags

WHEN a line record is parsed
THEN the parser SHALL expose the following flag bits from the `flags` field:
- Bit 14 (`0x4000`): solid (impassable)
- Bit 13 (`0x2000`): transparent
- Bit 12 (`0x1000`): landscape
- Bit 11 (`0x0800`): elevation
- Bit 10 (`0x0400`): variable elevation
- Bit 9 (`0x0200`): has transparent side
- Bit 8 (`0x0100`): decorative

---

### Requirement: Parse side data from WAD entries

The parser SHALL extract wall surface definitions from the `SIDS` tag (four-character code `SIDS`, `0x53494453`).

#### Scenario: Parse SIDS side records

WHEN a WAD entry contains a `SIDS` tag
THEN the parser SHALL interpret each 64-byte record as a `side_data` structure with the following big-endian fields in order:
- `type`: `i16` (full=0, high=1, low=2, composite=3, split=4)
- `flags`: `u16`
- `primary_texture`: `side_texture_definition` (6 bytes: `x0` i16, `y0` i16, `texture` u16)
- `secondary_texture`: `side_texture_definition` (6 bytes)
- `transparent_texture`: `side_texture_definition` (6 bytes)
- `exclusion_zone`: four `world_point2d` values (16 bytes total, runtime-only)
- `control_panel_type`: `i16`
- `control_panel_permutation`: `i16`
- `primary_transfer_mode`: `i16`
- `secondary_transfer_mode`: `i16`
- `transparent_transfer_mode`: `i16`
- `polygon_index`: `i16`
- `line_index`: `i16`
- `primary_lightsource_index`: `i16`
- `secondary_lightsource_index`: `i16`
- `transparent_lightsource_index`: `i16`
- `ambient_delta`: `i32`
- `unused`: one `i16` value (parsed and discarded)

AND the total tag data length MUST be evenly divisible by 64.

#### Scenario: Decode side flags

WHEN a side record is parsed
THEN the parser SHALL expose the following flag bits from the `flags` field:
- Bit 0 (`0x0001`): control panel status
- Bit 1 (`0x0002`): is control panel
- Bit 2 (`0x0004`): is repair switch
- Bit 3 (`0x0008`): is destructive switch
- Bit 4 (`0x0010`): is lighted switch
- Bit 5 (`0x0020`): switch can be destroyed
- Bit 6 (`0x0040`): switch can only be hit by projectiles
- Bit 7 (`0x0080`): item is optional

---

### Requirement: Parse polygon data from WAD entries

The parser SHALL extract room/space definitions from the `POLY` tag (four-character code `POLY`, `0x504F4C59`).

#### Scenario: Parse POLY polygon records

WHEN a WAD entry contains a `POLY` tag
THEN the parser SHALL interpret each 128-byte record as a `polygon_data` structure with the following big-endian fields in order:
- `type`: `i16`
- `flags`: `u16`
- `permutation`: `i16`
- `vertex_count`: `u16`
- `endpoint_indexes`: eight `i16` values (clockwise vertex references)
- `line_indexes`: eight `i16` values
- `floor_texture`: `u16` (shape_descriptor)
- `ceiling_texture`: `u16` (shape_descriptor)
- `floor_height`: `i16` (world_distance)
- `ceiling_height`: `i16` (world_distance)
- `floor_lightsource_index`: `i16`
- `ceiling_lightsource_index`: `i16`
- `area`: `i32`
- `first_object`: `i16` (runtime-only, ignored for map loading)
- `first_exclusion_zone_index`: `i16` (runtime-only)
- `line_exclusion_zone_count`: `i16` (runtime-only)
- `point_exclusion_zone_count`: `i16` (runtime-only)
- `floor_transfer_mode`: `i16`
- `ceiling_transfer_mode`: `i16`
- `adjacent_polygon_indexes`: eight `i16` values
- `first_neighbor_index`: `i16` (runtime-only)
- `neighbor_count`: `i16` (runtime-only)
- `center`: `world_point2d` (two `i16` values)
- `side_indexes`: eight `i16` values
- `floor_origin`: `world_point2d` (two `i16` values)
- `ceiling_origin`: `world_point2d` (two `i16` values)
- `media_index`: `i16`
- `media_lightsource_index`: `i16`
- `sound_source_indexes`: `i16`
- `ambient_sound_image_index`: `i16`
- `random_sound_image_index`: `i16`
- `unused`: one `i16` value (parsed and discarded)

AND the total tag data length MUST be evenly divisible by 128.
AND only the first `vertex_count` entries of the eight-element index arrays SHALL be considered valid (remaining entries MAY be NONE / -1).

#### Scenario: Decode polygon types

WHEN a polygon record is parsed
THEN the parser SHALL map the `type` field to a polygon type enum covering at minimum:
- 0: normal
- 1: item impassable
- 2: monster impassable
- 3: hill (king-of-the-hill)
- 4: base (capture the flag; team in permutation)
- 5: platform (platform index in permutation)
- 6: light on trigger
- 7: platform on trigger
- 8: light off trigger
- 9: platform off trigger
- 10: teleporter (destination polygon in permutation)
- 11: zone border
- 12: goal
- 13: visible monster trigger
- 14: invisible monster trigger
- 15: dual monster trigger
- 16: item trigger
- 17: must be explored
- 18: automatic exit
- 19: minor ouch
- 20: major ouch
- 21: glue
- 22: glue trigger
- 23: superglue

---

### Requirement: Parse polygon-line-endpoint connectivity

The parser SHALL preserve the full connectivity model encoded in the geometry arrays.

#### Scenario: Polygon to line and endpoint connectivity

WHEN polygon data is parsed
THEN each polygon SHALL reference its bounding lines via `line_indexes[0..vertex_count]` and its vertices via `endpoint_indexes[0..vertex_count]` in clockwise winding order.
AND each polygon SHALL reference its adjacent polygons via `adjacent_polygon_indexes[0..vertex_count]`, where each entry is either a valid polygon index or NONE (-1) for solid walls.

#### Scenario: Line to polygon ownership

WHEN line data is parsed
THEN each line SHALL reference its `clockwise_polygon_owner` and `counterclockwise_polygon_owner`, either of which MAY be NONE (-1).
AND each line SHALL reference its `clockwise_polygon_side_index` and `counterclockwise_polygon_side_index`, either of which MAY be NONE (-1).
AND the two endpoint indexes SHALL reference valid entries in the endpoint array.

#### Scenario: Side to polygon and line back-references

WHEN side data is parsed
THEN each side SHALL contain a `polygon_index` referencing the polygon it belongs to and a `line_index` referencing the line it is attached to.

---

### Requirement: Parse map objects with type discrimination

The parser SHALL extract placed map objects from the `OBJS` tag (four-character code `OBJS`, `0x4F424A53`).

#### Scenario: Parse OBJS map object records

WHEN a WAD entry contains an `OBJS` tag
THEN the parser SHALL interpret each 16-byte record as a `map_object` structure with the following big-endian fields in order:
- `type`: `i16`
- `index`: `i16`
- `facing`: `i16` (angle, 0..511 representing 0..360 degrees)
- `polygon_index`: `i16`
- `location`: `world_point3d` (three `i16` values: `x`, `y`, `z` where `z` is a delta)
- `flags`: `u16`

AND the total tag data length MUST be evenly divisible by 16.

#### Scenario: Discriminate map object types

WHEN a map object record is parsed
THEN the parser SHALL map the `type` field to a discriminated type:
- 0: monster (index is monster type)
- 1: scenery object (index is scenery type)
- 2: item (index is item type)
- 3: player (index is team bitfield)
- 4: goal (index is goal number)
- 5: sound source (index is source type, facing is sound volume)

#### Scenario: Decode map object flags

WHEN a map object record is parsed
THEN the parser SHALL expose the following flag bits:
- Bit 0 (`0x0001`): invisible (or platform sound, context-dependent)
- Bit 1 (`0x0002`): hanging from ceiling (affects absolute z calculation)
- Bit 2 (`0x0004`): blind (monster cannot activate by sight)
- Bit 3 (`0x0008`): deaf (monster cannot activate by sound)
- Bit 4 (`0x0010`): floats (used by sound sources on media)
- Bit 5 (`0x0020`): network only (items only)
AND the top four bits (bits 12-15) SHALL be exposed as the activation bias for monsters.

---

### Requirement: Parse light data from WAD entries

The parser SHALL extract light source definitions from the `LITE` tag (four-character code `LITE`, `0x4C495445`).

#### Scenario: Parse Marathon 2/Infinity static light data

WHEN a WAD entry contains a `LITE` tag and the map data version is greater than Marathon 1
THEN the parser SHALL interpret each 100-byte record as a `static_light_data` structure with the following big-endian fields:
- `type`: `i16`
- `flags`: `u16`
- `phase`: `i16`
- Six `lighting_function_specification` blocks (14 bytes each): `primary_active`, `secondary_active`, `becoming_active`, `primary_inactive`, `secondary_inactive`, `becoming_inactive`
- `tag`: `i16`
- `unused`: four `i16` values

AND each `lighting_function_specification` SHALL contain:
- `function`: `i16`
- `period`: `i16`
- `delta_period`: `i16`
- `intensity`: `i32` (fixed-point 16.16)
- `delta_intensity`: `i32` (fixed-point 16.16)

AND the total tag data length MUST be evenly divisible by 100.

#### Scenario: Parse Marathon 1 old light data

WHEN a WAD entry contains a `LITE` tag and the map data version is Marathon 1
THEN the parser SHALL interpret each 32-byte record as an `old_light_data` structure with the following big-endian fields:
- `flags`: `u16`
- `type`: `i16`
- `mode`: `i16`
- `phase`: `i16`
- `minimum_intensity`: `i32` (fixed-point 16.16)
- `maximum_intensity`: `i32` (fixed-point 16.16)
- `period`: `i16`
- `intensity`: `i32` (fixed-point 16.16, runtime state)
- `unused`: five `i16` values

AND the total tag data length MUST be evenly divisible by 32.

---

### Requirement: Parse platform data from WAD entries

The parser SHALL extract platform (elevator/door) definitions from the `plat` tag (four-character code `plat`, `0x706C6174`).

#### Scenario: Parse static platform records

WHEN a WAD entry contains a `plat` tag
THEN the parser SHALL interpret each 32-byte record as a `static_platform_data` structure with the following big-endian fields:
- `type`: `i16`
- `speed`: `i16`
- `delay`: `i16`
- `maximum_height`: `i16` (world_distance; NONE if calculated)
- `minimum_height`: `i16` (world_distance; NONE if calculated)
- `static_flags`: `u32`
- `polygon_index`: `i16`
- `tag`: `i16`
- `unused`: seven `i16` values

AND the total tag data length MUST be evenly divisible by 32.

---

### Requirement: Parse media data from WAD entries

The parser SHALL extract liquid/media definitions from the `medi` tag (four-character code `medi`, `0x6D656469`).

#### Scenario: Parse media records

WHEN a WAD entry contains a `medi` tag
THEN the parser SHALL interpret each 32-byte record as a `media_data` structure with the following big-endian fields:
- `type`: `i16` (0=water, 1=lava, 2=goo, 3=sewage, 4=jjaro)
- `flags`: `u16`
- `light_index`: `i16` (controls media height via light intensity)
- `current_direction`: `i16` (angle)
- `current_magnitude`: `i16` (world_distance)
- `low`: `i16` (world_distance)
- `high`: `i16` (world_distance)
- `origin`: `world_point2d` (two `i16` values)
- `height`: `i16` (world_distance, runtime state)
- `minimum_light_intensity`: `i32` (fixed-point 16.16)
- `texture`: `u16` (shape_descriptor)
- `transfer_mode`: `i16`
- `unused`: two `i16` values

AND the total tag data length MUST be evenly divisible by 32.

---

### Requirement: Parse map annotation data from WAD entries

The parser SHALL extract map annotations from the `NOTE` tag (four-character code `NOTE`, `0x4E4F5445`).

#### Scenario: Parse annotation records

WHEN a WAD entry contains a `NOTE` tag
THEN the parser SHALL interpret each 72-byte record as a `map_annotation` structure with the following big-endian fields:
- `type`: `i16`
- `location`: `world_point2d` (two `i16` values)
- `polygon_index`: `i16`
- `text`: 64 bytes of null-terminated text (MacRoman encoding)

AND the total tag data length MUST be evenly divisible by 72.

---

### Requirement: Parse terminal data from WAD entries

The parser SHALL extract computer terminal content from the `term` tag (four-character code `term`, `0x7465726D`).

#### Scenario: Parse terminal header and groupings

WHEN a WAD entry contains a `term` tag
THEN the parser SHALL interpret the terminal data as a sequence of terminal entries, each beginning with a 10-byte `static_preprocessed_terminal_data` header:
- `total_length`: `i16` (total byte length of this terminal entry including header)
- `flags`: `i16`
- `lines_per_page`: `i16`
- `grouping_count`: `i16`
- `font_changes_count`: `i16`

AND following the header, the parser SHALL read `grouping_count` terminal grouping records of 12 bytes each:
- `flags`: `i16`
- `type`: `i16` (logon, unfinished, success, failure, information, checkpoint, briefing, sound, movie, track, interlevel teleport, intralevel teleport, etc.)
- `permutation`: `i16`
- `start_index`: `i16`
- `length`: `i16`
- `maximum_line_count`: `i16`

AND following the groupings, the parser SHALL read `font_changes_count` text face records of 6 bytes each:
- `index`: `i16`
- `face`: `i16`
- `color`: `i16`

AND the remaining bytes up to `total_length` SHALL be the terminal text content.

---

### Requirement: Parse ambient sound data from WAD entries

The parser SHALL extract ambient sound images from the `ambi` tag (four-character code `ambi`, `0x616D6269`).

#### Scenario: Parse ambient sound records

WHEN a WAD entry contains an `ambi` tag
THEN the parser SHALL interpret each 16-byte record as an `ambient_sound_image_data` structure with the following big-endian fields:
- `flags`: `u16`
- `sound_index`: `i16`
- `volume`: `i16`
- `unused`: five `i16` values

AND the total tag data length MUST be evenly divisible by 16.

---

### Requirement: Parse random sound data from WAD entries

The parser SHALL extract random sound images from the `bonk` tag (four-character code `bonk`, `0x626F6E6B`).

#### Scenario: Parse random sound records

WHEN a WAD entry contains a `bonk` tag
THEN the parser SHALL interpret each 32-byte record as a `random_sound_image_data` structure with the following big-endian fields:
- `flags`: `u16`
- `sound_index`: `i16`
- `volume`: `i16`
- `delta_volume`: `i16`
- `period`: `i16`
- `delta_period`: `i16`
- `direction`: `i16` (angle)
- `delta_direction`: `i16` (angle)
- `pitch`: `i32` (fixed-point 16.16)
- `delta_pitch`: `i32` (fixed-point 16.16)
- `phase`: `i16` (runtime-only, initialize to NONE)
- `unused`: three `i16` values

AND the total tag data length MUST be evenly divisible by 32.

#### Scenario: Decode random sound flags

WHEN a random sound record is parsed
THEN the parser SHALL expose bit 0 (`0x0001`) as the non-directional flag, indicating the direction field is ignored.

---

### Requirement: Parse map info from WAD entries

The parser SHALL extract static map metadata from the `Minf` tag (four-character code `Minf`, `0x4D696E66`).

#### Scenario: Parse map info record

WHEN a WAD entry contains a `Minf` tag
THEN the parser SHALL interpret the data as a single 88-byte `static_data` structure with the following big-endian fields:
- `environment_code`: `i16`
- `physics_model`: `i16`
- `song_index`: `i16`
- `mission_flags`: `i16`
- `environment_flags`: `i16`
- `ball_in_play`: one byte (boolean)
- `unused1`: one byte
- `unused`: three `i16` values
- `level_name`: 66 bytes of null-terminated text
- `entry_point_flags`: `u32`

#### Scenario: Decode mission flags

WHEN map info is parsed
THEN the parser SHALL expose the following mission flag bits:
- Bit 0 (`0x0001`): extermination
- Bit 1 (`0x0002`): exploration
- Bit 2 (`0x0004`): retrieval
- Bit 3 (`0x0008`): repair
- Bit 4 (`0x0010`): rescue

#### Scenario: Decode environment flags

WHEN map info is parsed
THEN the parser SHALL expose the following environment flag bits:
- Bit 0 (`0x0001`): vacuum
- Bit 1 (`0x0002`): magnetic
- Bit 2 (`0x0004`): rebellion
- Bit 3 (`0x0008`): low gravity

#### Scenario: Decode entry point flags

WHEN map info is parsed
THEN the parser SHALL expose the following entry point flag bits:
- Bit 0 (`0x01`): single player
- Bit 1 (`0x02`): multiplayer cooperative
- Bit 2 (`0x04`): multiplayer carnage
- Bit 3 (`0x08`): kill the man with the ball
- Bit 4 (`0x10`): king of the hill
- Bit 5 (`0x20`): defense
- Bit 6 (`0x40`): rugby
- Bit 7 (`0x80`): capture the flag

---

### Requirement: Parse item placement data from WAD entries

The parser SHALL extract item/monster random placement rules from the `plac` tag (four-character code `plac`, `0x706C6163`).

#### Scenario: Parse item placement records

WHEN a WAD entry contains a `plac` tag
THEN the parser SHALL interpret each 12-byte record as an `object_frequency_definition` structure with the following big-endian fields:
- `flags`: `u16`
- `initial_count`: `i16`
- `minimum_count`: `i16`
- `maximum_count`: `i16`
- `random_count`: `i16`
- `random_chance`: `u16`

AND the total tag data length MUST be evenly divisible by 12.
AND bit 0 (`0x0001`) of `flags` SHALL indicate whether the object reappears in a random location.

---

### Requirement: Parse guard path data from WAD entries

The parser SHALL extract monster guard path definitions from the tag with four-character code `p\x8Cth` (`0x708C7468`).

#### Scenario: Parse guard path records

WHEN a WAD entry contains a guard path tag
THEN the parser SHALL interpret the tag data as a series of guard path records.
AND each record SHALL be parsed as raw bytes and exposed as an opaque guard path structure, since the format is not fully documented in the engine source.

---

### Requirement: Convert world coordinates to floating-point

The parser SHALL convert Marathon's fixed-point `world_distance` values to `f32` for downstream use.

#### Scenario: Convert world_distance (10 fractional bits) to f32

WHEN a `world_distance` value (`i16`) is read from any map geometry structure
THEN the parser SHALL convert it to `f32` by dividing the raw integer value by 1024.0 (2^10, since `WORLD_FRACTIONAL_BITS` = 10).
AND the value `WORLD_ONE` (1024 as `i16`) SHALL convert to exactly 1.0 as `f32`.
AND the raw integer value SHALL also be preserved for lossless round-tripping.

#### Scenario: Convert fixed-point 16.16 values to f32

WHEN a fixed-point `_fixed` value (`i32`) is read from light or media structures
THEN the parser SHALL convert it to `f32` by dividing the raw integer value by 65536.0 (2^16).
AND the raw integer value SHALL also be preserved for lossless round-tripping.

#### Scenario: Convert angle values to f32

WHEN an `angle` value (`i16`) is read from any map structure
THEN the parser SHALL support conversion to degrees by multiplying by (360.0 / 512.0), since `NUMBER_OF_ANGLES` = 512 (9 angular bits).
AND the raw integer value SHALL also be preserved.

---

### Requirement: Validate cross-references between geometry structures

The parser SHALL validate that index references between structures are consistent.

#### Scenario: Validate polygon endpoint references

WHEN polygon data is parsed
THEN for each polygon, every `endpoint_indexes[i]` for `i` in `0..vertex_count` MUST be either NONE (-1) or a valid index into the parsed endpoint array (0 <= index < endpoint_count).
AND if any endpoint index is out of range, the parser SHALL report a validation error identifying the polygon index and the invalid endpoint reference.

#### Scenario: Validate polygon line references

WHEN polygon data is parsed
THEN for each polygon, every `line_indexes[i]` for `i` in `0..vertex_count` MUST be either NONE (-1) or a valid index into the parsed line array (0 <= index < line_count).
AND if any line index is out of range, the parser SHALL report a validation error identifying the polygon index and the invalid line reference.

#### Scenario: Validate polygon adjacent polygon references

WHEN polygon data is parsed
THEN for each polygon, every `adjacent_polygon_indexes[i]` for `i` in `0..vertex_count` MUST be either NONE (-1) or a valid index into the parsed polygon array (0 <= index < polygon_count).
AND if any adjacent polygon index is out of range, the parser SHALL report a validation error identifying the polygon index and the invalid adjacency reference.

#### Scenario: Validate polygon side references

WHEN polygon data is parsed
THEN for each polygon, every `side_indexes[i]` for `i` in `0..vertex_count` MUST be either NONE (-1) or a valid index into the parsed side array (0 <= index < side_count).

#### Scenario: Validate line endpoint references

WHEN line data is parsed
THEN for each line, both `endpoint_indexes[0]` and `endpoint_indexes[1]` MUST be valid indexes into the parsed endpoint array (0 <= index < endpoint_count).
AND if either endpoint index is out of range, the parser SHALL report a validation error identifying the line index and the invalid endpoint reference.

#### Scenario: Validate line polygon owner references

WHEN line data is parsed
THEN for each line, `clockwise_polygon_owner` and `counterclockwise_polygon_owner` MUST each be either NONE (-1) or a valid index into the parsed polygon array (0 <= index < polygon_count).

#### Scenario: Validate side back-references

WHEN side data is parsed
THEN for each side, `polygon_index` MUST be a valid index into the parsed polygon array (0 <= index < polygon_count).
AND `line_index` MUST be a valid index into the parsed line array (0 <= index < line_count).

#### Scenario: Validate map object polygon references

WHEN map object data is parsed
THEN for each map object, `polygon_index` MUST be a valid index into the parsed polygon array (0 <= index < polygon_count).

#### Scenario: Validation is optional and non-blocking

WHEN the parser encounters invalid cross-references
THEN validation errors SHALL be collected and returned alongside the parsed data, rather than aborting the parse.
AND the caller SHALL be able to opt into or out of cross-reference validation.
