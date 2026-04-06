## ADDED Requirements

### Requirement: Shape collection loading
The system SHALL load Marathon shape collections from a Shapes WAD file using `ShapesFile::open()`. For each collection referenced by the level's textures, the system SHALL parse the collection via `ShapesFile::collection()` to obtain bitmaps, color tables, and shape metadata.

#### Scenario: Load collections used by level
- **WHEN** a level references textures from collections 1, 5, and 17
- **THEN** the system loads exactly those collections from the shapes file

#### Scenario: Handle missing collection
- **WHEN** a level references a collection index with status=0 (unused) in the shapes file
- **THEN** the system skips that collection and renders affected surfaces with a fallback color or checkerboard pattern

### Requirement: Bitmap to RGBA conversion
The system SHALL convert Marathon's 8-bit indexed-color bitmaps to RGBA8 format by applying the appropriate color lookup table (CLUT). For each pixel in the bitmap, the system SHALL look up the ColorValue at that index in the CLUT and produce an (R, G, B, A) pixel where A=255 for opaque pixels and A=0 for transparent pixels (index 0 in transparent bitmaps).

#### Scenario: Opaque bitmap conversion
- **WHEN** a bitmap has transparent=false and pixel value 42
- **THEN** the RGBA output for that pixel is (clut[42].red >> 8, clut[42].green >> 8, clut[42].blue >> 8, 255)

#### Scenario: Transparent bitmap conversion
- **WHEN** a bitmap has transparent=true and a pixel with the transparent marker
- **THEN** the RGBA output for that pixel has A=0

### Requirement: Column-major to row-major transposition
The system SHALL transpose Marathon bitmaps from column-major to row-major order during loading. Marathon stores bitmaps column-by-column (x varies slowest), but GPU textures expect row-major layout.

#### Scenario: Transpose bitmap layout
- **WHEN** a bitmap has width=128, height=128, column_order=true
- **THEN** the system transposes the pixel data so that row-major iteration produces the correct image

### Requirement: Texture array creation per collection
The system SHALL create one wgpu 2D texture array per shape collection. Each bitmap in the collection becomes one layer of the array. Bitmaps within a collection that differ in dimensions SHALL be resized (padded or scaled) to match the largest dimensions in that collection.

#### Scenario: Uniform-sized bitmaps
- **WHEN** a collection has 12 bitmaps all at 128x128
- **THEN** the system creates a single texture array with 12 layers at 128x128

#### Scenario: Mixed-size bitmaps
- **WHEN** a collection has bitmaps at 128x128 and 64x64
- **THEN** the system creates a texture array at the maximum dimension (128x128) and pads or scales smaller bitmaps to fit

### Requirement: ShapeDescriptor to texture lookup
The system SHALL resolve `ShapeDescriptor` values to GPU texture references. Given a ShapeDescriptor, the system SHALL extract the collection index (bits 12:8), CLUT index (bits 15:13), and shape index (bits 7:0). The shape index maps to a `LowLevelShape` which contains the `bitmap_index` identifying the texture array layer.

#### Scenario: Resolve wall texture
- **WHEN** a side's primary_texture has ShapeDescriptor with collection=5, clut=0, shape_index=12
- **THEN** the system looks up collection 5's LowLevelShape[12].bitmap_index to find the texture array layer

#### Scenario: None descriptor
- **WHEN** a ShapeDescriptor's `is_none()` returns true
- **THEN** the surface is not textured (skip rendering or use fallback)

### Requirement: CLUT selection
The system SHALL support selecting which CLUT to apply when converting bitmaps. The default CLUT index is 0. When a ShapeDescriptor specifies a non-zero CLUT index, the system SHALL use that CLUT for the texture lookup, producing a differently-colored variant of the bitmap.

#### Scenario: Alternate CLUT
- **WHEN** a ShapeDescriptor has clut=2 and the collection has 4 CLUTs
- **THEN** the system applies CLUT index 2 when converting that bitmap to RGBA

### Requirement: Texture bind group management
The system SHALL create wgpu bind groups for each loaded texture array so the fragment shader can sample textures. The bind group SHALL include the texture array view and a sampler configured for nearest-neighbor filtering (to match Marathon's pixel art style) with repeating address mode (for tiling textures).

#### Scenario: Bind group for rendering
- **WHEN** the render pipeline needs to draw surfaces from collection 5
- **THEN** a bind group exists with collection 5's texture array and a nearest-neighbor repeating sampler
