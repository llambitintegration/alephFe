## Why

Marathon (Aleph One) content is locked inside undocumented binary formats that only the C++ engine can read, making it impossible for new tools or engines to work with existing scenarios, plugins, and total conversions. A standalone Rust crate that parses every Marathon binary format into well-typed structures will unblock the rest of the engine while also being independently useful as a published library on crates.io for the broader Marathon modding community.

## What Changes

- Create a new `marathon-formats` crate that reads all Marathon/Aleph One binary content formats
- Parse WAD container files (the universal container for maps, shapes, sounds, physics, saves, and films) across all format versions (v0 pre-entry-point through v4 Infinity)
- Parse map geometry data from WAD entries: endpoints, lines, sides, polygons, objects, lights, platforms, media, annotations, terminals, ambient/random sounds, map info, placement structures, and guard paths
- Parse shape/sprite collections from Shapes files: collection definitions (544-byte headers), color tables (CLUTs), high-level shape definitions (sequences/animations), low-level shape definitions (frames), and bitmap data with RLE decompression for object/scenery types
- Parse sound definition headers and permutation metadata from Sounds files
- Parse all five physics model tag types from Physics files: monster physics, effect physics, projectile physics, player physics (physics_constants with fixed-point values), and weapon physics
- Parse MML (Marathon Markup Language) XML configuration files using quick-xml
- Parse plugin metadata (Plugin.xml): scenario requirements, MML references, shapes/sounds patches, Lua script references, and map patches
- Use `binrw` for all binary format parsing with big-endian byte order (Marathon's native format)
- Convert Marathon fixed-point values (16.16) to Rust numeric types
- Provide `thiserror`-based error types for all parse failures

## Capabilities

### New Capabilities

- `wad-format`: WAD container parsing -- 128-byte file header, versioned directory structures (old 8-byte and new 10-byte entries), tagged chunk entries (old 12-byte and new 16-byte headers), entry extraction by four-character tag code, and support for overlay/patch WADs via parent checksums
- `map-geometry`: Map data parsing from WAD tag entries -- endpoints (EPNT), lines (LINS), sides (SIDS), polygons (POLY, 128 bytes each), map objects (OBJS), lights (LITE), platforms (plat), media (medi), annotations (NOTE), terminals (term), ambient sounds (ambi), random sounds (bonk), map info (Minf), item placement (plac), and guard paths
- `shape-collections`: Shape/sprite format parsing -- collection_definition headers (544 bytes, version 3), color lookup tables (CLUTs with private colors), high-level shape definitions (animation sequences with frame lists), low-level shape definitions (individual frames with world-space scaling), and bitmap pixel data with RLE decompression for _object_collection and _scenery_collection types
- `sound-formats`: Sound file parsing -- sound file header validation (tag 'snd2', version 1), sound definition entries with behavioral flags (quiet/normal/loud, restart/abort/pitch/obstruction control), and up to 5 permutations per sound with offset/length references into audio data
- `physics-formats`: Physics model parsing from WAD physics tags -- monster physics (MNpx), effect physics (FXpx), projectile physics (PRpx), player physics (PXpx with walking/running physics_constants), and weapon physics (WPpx), including Marathon 1 tag variants, with fixed-point to float conversion
- `mml-config`: MML (Marathon Markup Language) XML configuration parsing using quick-xml -- reading configuration overrides for engine parameters that scenarios and plugins use to customize game behavior
- `plugin-metadata`: Plugin discovery and metadata parsing -- reading Plugin.xml files for plugin name, description, version, scenario requirements (ScenarioInfo), MML file lists, shapes/sounds patch references, Lua script declarations (HUD/solo/stats with write-access flags), theme overrides, and map patch definitions with parent checksum matching

### Modified Capabilities

(none -- greenfield project)

## Impact

- **New crate**: `marathon-formats` added to the workspace, publishable independently to crates.io
- **Dependencies**: `binrw` (binary parsing), `quick-xml` (MML/plugin XML), `thiserror` (error types), `bitflags` (flag fields)
- **Public API**: Exposes Rust structs and parsing functions for every format; all downstream crates (renderer, simulation, audio, editor tooling) will depend on this crate for data access
- **No runtime impact**: This is a parse-only library with no game logic, rendering, or I/O beyond reading byte slices
- **Compatibility target**: Must correctly parse content from Marathon 2, Marathon Infinity, and Aleph One community scenarios/plugins
