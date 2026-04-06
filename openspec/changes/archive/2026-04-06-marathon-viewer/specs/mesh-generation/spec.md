## ADDED Requirements

### Requirement: Floor surface triangulation
The system SHALL convert each polygon's floor into a triangle mesh using fan triangulation from vertex 0. For an N-vertex polygon with endpoint indices [v0, v1, ..., vN-1], the system SHALL emit triangles (v0, v1, v2), (v0, v2, v3), ..., (v0, vN-2, vN-1). The Y coordinate of each vertex SHALL be the polygon's `floor_height` converted to world units. UV coordinates SHALL be computed from the polygon's `floor_origin` offset and the endpoint's world XZ position.

#### Scenario: Triangulate a quad floor
- **WHEN** a polygon has 4 endpoints at positions [(0,0), (1024,0), (1024,1024), (0,1024)] with floor_height 0
- **THEN** the system produces 2 triangles: (v0,v1,v2) and (v0,v2,v3) with Y=0 for all vertices

#### Scenario: Triangulate an 8-vertex floor
- **WHEN** a polygon has 8 endpoints (maximum) with floor_height 512
- **THEN** the system produces 6 triangles all at Y = world_distance_to_f32(512)

#### Scenario: Floor UV mapping
- **WHEN** a polygon has floor_origin (128, 256) and an endpoint at world position (640, 768)
- **THEN** the UV for that vertex is computed from the offset between the endpoint position and floor_origin, scaled by texture dimensions

### Requirement: Ceiling surface triangulation
The system SHALL convert each polygon's ceiling into a triangle mesh using the same fan triangulation as floors. The Y coordinate SHALL be the polygon's `ceiling_height`. The winding order SHALL be reversed (or face culling adjusted) so that the ceiling faces downward. UV coordinates SHALL be computed from the polygon's `ceiling_origin` offset.

#### Scenario: Ceiling faces downward
- **WHEN** a polygon has floor_height 0 and ceiling_height 2048
- **THEN** the ceiling mesh is at Y = world_distance_to_f32(2048) with normals pointing downward (reversed winding relative to floor)

### Requirement: Full wall construction
The system SHALL construct wall geometry for sides with `side_type` = full (0). A full wall spans from the polygon's floor_height to ceiling_height. The system SHALL build a quad (two triangles) between the two line endpoints at these heights, using the side's `primary_texture` for UV mapping.

#### Scenario: Full wall between floor and ceiling
- **WHEN** a line has a side with side_type=full, the polygon has floor_height=0 and ceiling_height=2048
- **THEN** the system creates a quad from floor to ceiling height using the primary texture

### Requirement: High wall construction
The system SHALL construct wall geometry for sides with `side_type` = high (1). A high wall spans from the adjacent polygon's ceiling_height to this polygon's ceiling_height (the portion above the opening). The side's `primary_texture` SHALL be applied to this surface.

#### Scenario: High wall above opening
- **WHEN** a line separates polygon A (ceiling=3072) from polygon B (ceiling=2048), and the side on polygon A has side_type=high
- **THEN** the system creates a wall quad from Y=world_distance_to_f32(2048) to Y=world_distance_to_f32(3072)

### Requirement: Low wall construction
The system SHALL construct wall geometry for sides with `side_type` = low (2). A low wall spans from this polygon's floor_height to the adjacent polygon's floor_height (the portion below the opening). The side's `primary_texture` SHALL be applied.

#### Scenario: Low wall below opening
- **WHEN** a line separates polygon A (floor=0) from polygon B (floor=1024), and the side on polygon A has side_type=low
- **THEN** the system creates a wall quad from Y=world_distance_to_f32(0) to Y=world_distance_to_f32(1024)

### Requirement: Split wall construction
The system SHALL construct wall geometry for sides with `side_type` = split (3). A split wall has both a high section (primary_texture) and a low section (secondary_texture) with a transparent middle section (transparent_texture) where adjacent polygons overlap in height range.

#### Scenario: Split wall with three sections
- **WHEN** a line separates polygon A (floor=0, ceiling=4096) from polygon B (floor=1024, ceiling=3072), and the side has side_type=split
- **THEN** the system creates: a low quad (0 to 1024) with secondary_texture, a transparent quad (1024 to 3072) with transparent_texture, and a high quad (3072 to 4096) with primary_texture

### Requirement: Wall UV computation
The system SHALL compute wall UV coordinates from the side's texture offsets (x0, y0) and the wall's world-space dimensions. The U coordinate SHALL map along the wall's horizontal length. The V coordinate SHALL map along the wall's vertical height. Texture offsets SHALL shift the UV origin.

#### Scenario: Wall with texture offset
- **WHEN** a wall quad spans 2 world units horizontally and 1 world unit vertically, with side texture offset (64, 128)
- **THEN** the UV origin is shifted by the offset values, and U spans the wall length while V spans the wall height

### Requirement: Vertex buffer construction
The system SHALL produce GPU-ready vertex buffers containing position (vec3), UV (vec2), polygon index (u32), and texture descriptor (u32) per vertex. The system SHALL also produce an index buffer for indexed drawing. All floor vertices, ceiling vertices, and wall vertices SHALL be combined into buffers suitable for batched draw calls.

#### Scenario: Combined vertex buffer
- **WHEN** a level has 100 polygons and 200 walls
- **THEN** the system produces vertex and index buffers containing all floor triangles, ceiling triangles, and wall quads, each vertex tagged with its polygon index for storage buffer lookup

### Requirement: Platform geometry animation
The system SHALL support animating platform polygons by updating their floor_height and/or ceiling_height in the per-polygon storage buffer each frame. The mesh vertices SHALL remain static — only the height values in the uniform data change. The vertex shader SHALL read current heights from the storage buffer and apply them to offset vertex Y positions.

#### Scenario: Elevator platform moves
- **WHEN** a platform polygon's height changes from 0 to 1024 over time
- **THEN** the per-polygon storage buffer entry is updated with the new height, and the vertex shader applies this offset without modifying the vertex buffer

### Requirement: Media surface geometry
The system SHALL render media (liquid) surfaces as flat quads at the media's current height within the containing polygon. Media surfaces SHALL use the media's texture descriptor and transfer mode. The media height in the storage buffer SHALL be updated each frame for animated media.

#### Scenario: Water surface rendering
- **WHEN** a polygon has media_index pointing to a media entry with height=512 and texture=water
- **THEN** the system renders a surface at Y=world_distance_to_f32(512) within that polygon using the water texture
