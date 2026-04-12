## MODIFIED Requirements

### Requirement: Media surface geometry
The system SHALL render media (liquid) surfaces as flat quads at the media's current height within the containing polygon. Media surfaces SHALL use the media's texture descriptor with bit 31 set to flag them as media vertices. The media height in the storage buffer SHALL be updated each frame for animated media. The mesh builder SHALL emit all opaque geometry (floors, ceilings, walls) first, then all media geometry, and SHALL return an `opaque_index_count` value marking the boundary between the two ranges in the index buffer.

#### Scenario: Water surface rendering
- **WHEN** a polygon has media_index pointing to a media entry with height=512 and texture=water
- **THEN** the system renders a surface at Y=world_distance_to_f32(512) within that polygon using the water texture with bit 31 set on the texture descriptor

#### Scenario: Media vertex texture descriptor flagging
- **WHEN** `build_media_surface()` emits a vertex for a media surface with base texture descriptor 0x0013
- **THEN** the vertex's `texture_descriptor` SHALL be 0x80000013 (bit 31 set to flag as media)

#### Scenario: Index buffer ordering
- **WHEN** a level has 500 opaque triangles and 20 media triangles
- **THEN** the index buffer SHALL contain opaque indices at positions 0..1500, media indices at positions 1500..1560, and `opaque_index_count` SHALL be 1500

### Requirement: LevelMesh opaque/media index split
The `LevelMesh` struct SHALL include an `opaque_index_count` field (u32) indicating how many indices in the index buffer belong to opaque geometry. Indices from 0 to `opaque_index_count` SHALL be opaque geometry. Indices from `opaque_index_count` to the total index count SHALL be media surface geometry. This enables the renderer to issue two separate draw calls with different pipeline states.

#### Scenario: Level with no media
- **WHEN** a level has no polygons with media_index >= 0
- **THEN** `opaque_index_count` SHALL equal the total index count, and no media draw call is needed

#### Scenario: Level with media
- **WHEN** a level has some polygons with media
- **THEN** `opaque_index_count` SHALL be less than the total index count, and the media draw call covers the remaining indices

### Requirement: Vertex buffer construction
The system SHALL produce GPU-ready vertex buffers containing position (vec3), UV (vec2), polygon index (u32), and texture descriptor (u32) per vertex. The system SHALL also produce an index buffer for indexed drawing. All floor vertices, ceiling vertices, and wall vertices SHALL be combined into buffers suitable for batched draw calls. Media surface vertices SHALL follow opaque vertices and SHALL have bit 31 set in their texture descriptor.

#### Scenario: Combined vertex buffer
- **WHEN** a level has 100 polygons and 200 walls, some polygons with media
- **THEN** the system produces vertex and index buffers containing all floor triangles, ceiling triangles, and wall quads first, then media surface triangles, each vertex tagged with its polygon index for storage buffer lookup
