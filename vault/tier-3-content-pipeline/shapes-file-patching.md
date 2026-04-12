---
tags: [tier-3, content-pipeline, shapes, patching, graphics]
status: research-complete
---

# Shapes File Patching

## Overview

Marathon Shapes files (`.shpA`) contain all 2D sprite and texture data organized into 32 collections. Plugins can deliver shapes patches (`.ShPa` files) that replace or add individual components within collections without replacing the entire Shapes file. This is the primary mechanism for high-resolution texture packs, custom monster sprites, and visual modifications.

## Shapes File Structure Recap

A standard Marathon Shapes file contains:

1. **32 Collection Headers** (1024 bytes total, 32 bytes each)
   - Each header points to 8-bit and optionally 16-bit (true color) collection data
   - Offsets are absolute byte positions in the file; -1 means no data

2. **Collection Data Blocks** -- Each contains:
   - **Collection Definition** (544 bytes) -- Version (must be 3), type, counts, offsets
   - **Color Tables** -- Arrays of CLUT entries (8 bytes each: flags, value, R, G, B as u16)
   - **High-Level Shapes** (animation sequences) -- 90-byte headers + frame index arrays
   - **Low-Level Shapes** (individual frames) -- 36 bytes each with spatial metadata
   - **Bitmaps** -- 26-byte headers + pixel data (8-bit indexed, raw or RLE compressed)

### Collection Types

| Value | Type | Bitmap Storage |
|-------|------|---------------|
| 0 | Unused | N/A |
| 1 | Wall | Raw uncompressed |
| 2 | Object | RLE compressed |
| 3 | Interface | Raw uncompressed |
| 4 | Scenery | RLE compressed |

### The 32 Collections

| Index | Name | Type |
|-------|------|------|
| 0 | Interface | Interface |
| 1 | Weapons in Hand | Object |
| 2 | Juggernaut | Object |
| 3 | Tick | Object |
| 4 | Explosions | Object |
| 5 | Hunter | Object |
| 6 | Player | Object |
| 7 | Items | Object |
| 8 | Trooper | Object |
| 9 | Pfhor Fighter | Object |
| 10 | S'pht'Kr (Defender) | Object |
| 11 | F'lickta (Yeti) | Object |
| 12 | Bob (Civilian) | Object |
| 13 | VacBob | Object |
| 14 | Enforcer | Object |
| 15 | Drone (Compiler) | Object |
| 16 | S'pht (Compiler) | Object |
| 17 | Water Walls | Wall |
| 18 | Lava Walls | Wall |
| 19 | Sewage Walls | Wall |
| 20 | Jjaro Walls | Wall |
| 21 | Pfhor Walls | Wall |
| 22 | Water Scenery | Scenery |
| 23 | Lava Scenery | Scenery |
| 24 | Sewage Scenery | Scenery |
| 25 | Jjaro Scenery | Scenery |
| 26 | Pfhor Scenery | Scenery |
| 27 | Day Landscape | Wall |
| 28 | Night Landscape | Wall |
| 29 | Moon Landscape | Wall |
| 30 | Space Landscape | Wall |
| 31 | Cyborg | Object |

## Shapes Patch Binary Format

Shapes patches use a **tag-based binary format** distinct from the main Shapes file format. The file extension `.ShPa` distinguishes patches from full shapes files (`.shpA`).

### Patch Structure

A shapes patch file is a sequence of collection patch blocks:

```
[collection_index: i16] [bit_depth: i16]
  [tag: 4 bytes] [data_length: i32] [data: bytes]
  [tag: 4 bytes] [data_length: i32] [data: bytes]
  ...
  [ENDC tag]
[collection_index: i16] [bit_depth: i16]
  ...
```

### Tag Types

| Tag (4 chars) | Meaning | Data Content |
|---------------|---------|-------------|
| `CLDF` | Collection Definition | 544-byte replacement collection definition |
| `HLSH` | High-Level Shape | Animation sequence definition (header + frame indices) |
| `LLSH` | Low-Level Shape | Frame metadata (36 bytes) |
| `BMAP` | Bitmap | Bitmap header (26 bytes) + pixel data |
| `CTAB` | Color Table | Color table entries (8 bytes per color) |
| `ENDC` | End of Collection | Marks end of patches for this collection |

### Patch Application Logic

The `load_shapes_patch()` function in the C++ source processes patches as follows:

1. Read collection index and bit-depth from patch header
2. Look up the target collection via `get_collection_definition()`
3. Validate bit-depth compatibility (typically requires 8-bit match)
4. For each tagged block until `ENDC`:
   - `CLDF`: Replace the entire collection definition using `load_collection_definition()`
   - `HLSH`: Replace a specific high-level shape (animation sequence)
   - `LLSH`: Replace a specific low-level shape (frame)
   - `BMAP`: Replace a specific bitmap
   - `CTAB`: Replace a specific color table
5. Set `_PATCHED_BIT` flag on modified bitmaps
6. Skip unrecognized tags without error

### Important Behaviors

- Patches are applied **after** all standard collections load during `load_collections()`
- The `override_replacements` parameter controls whether the patched flag is set
- Invalid or incompatible patches are silently skipped
- Multiple patches can target the same collection (applied in plugin order)
- Patches can both replace existing data and add new entries (e.g., adding bitmaps to a collection)

## Types of Modifications

### 1. Full Collection Replacement
Replace the entire collection definition, all color tables, all shapes, and all bitmaps. Used for total conversion scenarios that completely replace a monster or texture set.

### 2. Individual Bitmap Replacement
Replace specific bitmaps within a collection while keeping all other data intact. Common for high-resolution texture packs that upgrade individual wall textures.

### 3. Color Table Override
Replace color tables (palettes) to achieve recoloring effects without modifying bitmap data. Used for things like recoloring monster sprites for team variants.

### 4. Animation Sequence Modification
Replace high-level shapes to change animation behavior (frame order, timing, number of views) while potentially reusing existing bitmaps.

### 5. Frame Metadata Modification
Replace low-level shapes to adjust spatial properties (origin, keypoint, mirroring) without changing bitmap pixel data.

## Interaction with OpenGL/MML Textures

MML's `<opengl>` section can specify external hi-res texture files that override shapes data at the rendering level:

```xml
<opengl>
  <texture coll="17" bitmap="0" normal_image="walls/water_wall_0.png"/>
</opengl>
```

This operates at a higher level than shapes patches:
- Shapes patches modify the shapes file data in memory
- OpenGL texture overrides replace the rendered texture with an external image file
- Both can coexist (OpenGL override takes precedence for rendering, but shapes data is still used for collision, animation metadata, etc.)

## Current State in Rust Rebuild

**Parser location:** `marathon-formats/src/shapes.rs`

**What the Rust code handles:**
- `ShapesFile::from_bytes()` / `::open()` -- Parse complete shapes files
- `CollectionHeader` (32 bytes) with 8-bit and 16-bit offsets
- `CollectionDefinition` (544 bytes) with version validation
- `ColorValue` -- CLUT entries with self-luminescence flag
- `HighLevelShape` -- Animation sequences with frame indices and view count calculation
- `LowLevelShape` -- Frame metadata with mirroring and keypoint flags
- `Bitmap` -- Decompressed pixel data (handles both raw and RLE formats)
- `Collection` -- Fully parsed collection with all sub-components
- `ShapesFile::collection()` / `::collection_with_depth()` -- Parse individual collections
- `actual_view_count()` -- Maps number_of_views field to actual view count

**What is missing:**
- No shapes patch file parsing (tag-based `.ShPa` format)
- No incremental patching API (replace individual components within a loaded collection)
- No ability to add new bitmaps/shapes/color tables to an existing collection
- No `_PATCHED_BIT` tracking
- No integration with the plugin system for patch file loading
- No support for creating/writing shapes patch files

## Gaps and Implementation Plan

### Phase 1: Patch Format Parser
Create a `ShapesPatch` parser that reads the tag-based binary format:

```rust
pub struct ShapesPatchEntry {
    pub collection_index: i16,
    pub bit_depth: i16,
    pub operations: Vec<PatchOperation>,
}

pub enum PatchOperation {
    ReplaceDefinition(CollectionDefinition),
    ReplaceHighLevelShape { index: usize, shape: HighLevelShape },
    ReplaceLowLevelShape { index: usize, shape: LowLevelShape },
    ReplaceBitmap { index: usize, bitmap: Bitmap },
    ReplaceColorTable { index: usize, colors: Vec<ColorValue> },
}

pub struct ShapesPatchFile {
    pub entries: Vec<ShapesPatchEntry>,
}

impl ShapesPatchFile {
    pub fn from_bytes(data: &[u8]) -> Result<Self, ShapeError> { ... }
}
```

### Phase 2: Mutable Collection API
Add methods to `Collection` for incremental modification:

```rust
impl Collection {
    pub fn apply_patch(&mut self, op: &PatchOperation) { ... }
    pub fn replace_bitmap(&mut self, index: usize, bitmap: Bitmap) { ... }
    pub fn replace_color_table(&mut self, index: usize, colors: Vec<ColorValue>) { ... }
    pub fn replace_high_level_shape(&mut self, index: usize, shape: HighLevelShape) { ... }
    pub fn replace_low_level_shape(&mut self, index: usize, shape: LowLevelShape) { ... }
    // Add new entries
    pub fn add_bitmap(&mut self, bitmap: Bitmap) -> usize { ... }
    pub fn add_color_table(&mut self, colors: Vec<ColorValue>) -> usize { ... }
}
```

### Phase 3: Patched Bitmap Tracking
Add a `patched` flag to `Bitmap` for renderer identification:

```rust
pub struct Bitmap {
    pub width: i16,
    pub height: i16,
    pub column_order: bool,
    pub transparent: bool,
    pub pixels: Vec<u8>,
    pub patched: bool,  // Set when modified by a shapes patch
}
```

### Phase 4: Integration with Plugin Pipeline
Wire shapes patch loading into the plugin system:
1. After `resolve_exclusive_resources()`, iterate active plugins
2. For each `ShapesPatch` entry, check `requires_opengl` against current renderer
3. Load and parse the `.ShPa` file
4. Apply patch operations to the in-memory shapes data

### Phase 5: OpenGL Texture Override Support
Support external image file references from MML `<opengl>` section:
1. Parse `<texture>` elements with `normal_image`, `glow_image`, etc.
2. Load external PNG/image files
3. Override rendered textures while preserving shapes metadata

## Recommended Rust Crates

- `binrw` (already used) -- Binary format parsing for patch files
- `image` -- Loading external texture images (PNG, etc.) for OpenGL overrides
- `byteorder` -- Additional binary reading utilities if needed

## Related Notes

- [[plugin-system-patching]] -- Plugins declare shapes patches in Plugin.xml
- [[mml-override-system]] -- MML `<opengl>` section provides texture overrides
- [[community-content-ecosystem]] -- HD texture packs and visual mods use shapes patches
