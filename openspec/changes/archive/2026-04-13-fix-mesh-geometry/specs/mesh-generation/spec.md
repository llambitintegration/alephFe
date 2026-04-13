## MODIFIED Requirements

### Requirement: Floor surface triangulation
The system SHALL convert each polygon's floor into a triangle mesh using fan triangulation from the first valid vertex. For an N-vertex polygon with endpoint indices that may include `-1` sentinel values, the system SHALL skip entries with index < 0 and emit vertices only for valid endpoints. The fan triangulation SHALL use the actual number of emitted vertices (not the declared `vertex_count`). If fewer than 3 valid vertices exist, no triangles SHALL be emitted for that polygon. The Y coordinate of each vertex SHALL be the polygon's `floor_height` converted to world units. UV coordinates SHALL be computed from the polygon's `floor_origin` offset and the endpoint's world XZ position.

#### Scenario: Triangulate a quad floor
- **WHEN** a polygon has 4 endpoints at positions [(0,0), (1024,0), (1024,1024), (0,1024)] with floor_height 0
- **THEN** the system produces 2 triangles: (v0,v1,v2) and (v0,v2,v3) with Y=0 for all vertices

#### Scenario: Triangulate an 8-vertex floor
- **WHEN** a polygon has 8 endpoints (maximum) with floor_height 512 and all endpoints valid
- **THEN** the system produces 6 triangles all at Y = world_distance_to_f32(512)

#### Scenario: Floor UV mapping
- **WHEN** a polygon has floor_origin (128, 256) and an endpoint at world position (640, 768)
- **THEN** the UV for that vertex is computed from the offset between the endpoint position and floor_origin, scaled by texture dimensions

#### Scenario: Polygon with sparse endpoint array
- **WHEN** a polygon has vertex_count=5 and endpoint_indexes=[10, -1, 12, 13, 14]
- **THEN** the system emits 4 vertices (skipping index -1) and produces 2 triangles from the 4 valid vertices

#### Scenario: Degenerate polygon with fewer than 3 valid endpoints
- **WHEN** a polygon has vertex_count=4 and endpoint_indexes=[5, -1, -1, 7]
- **THEN** the system emits 2 vertices and produces 0 triangles (fewer than 3 valid vertices)

### Requirement: Ceiling surface triangulation
The system SHALL convert each polygon's ceiling into a triangle mesh using the same fan triangulation as floors, with the same handling of `-1` sentinel endpoint values. The Y coordinate SHALL be the polygon's `ceiling_height`. The winding order SHALL be reversed (or face culling adjusted) so that the ceiling faces downward. UV coordinates SHALL be computed from the polygon's `ceiling_origin` offset.

#### Scenario: Ceiling faces downward
- **WHEN** a polygon has floor_height 0 and ceiling_height 2048
- **THEN** the ceiling mesh is at Y = world_distance_to_f32(2048) with normals pointing downward (reversed winding relative to floor)

#### Scenario: Ceiling with sparse endpoints
- **WHEN** a polygon has vertex_count=6 and endpoint_indexes=[0, 1, -1, 3, 4, 5]
- **THEN** the system emits 5 vertices (skipping the -1 entry) and produces 3 ceiling triangles with reversed winding

### Requirement: Full wall construction
The system SHALL construct wall geometry for sides with `side_type` = full (0). A full wall spans from the polygon's floor_height to ceiling_height. The system SHALL build a quad (two triangles) between the two line endpoints at these heights, using the side's `primary_texture` for UV mapping. If the primary texture descriptor is `0xFFFF` (none), the wall quad SHALL NOT be emitted.

#### Scenario: Full wall between floor and ceiling
- **WHEN** a line has a side with side_type=full, the polygon has floor_height=0 and ceiling_height=2048, and primary_texture is valid
- **THEN** the system creates a quad from floor to ceiling height using the primary texture

#### Scenario: Full wall with no texture
- **WHEN** a line has a side with side_type=full and primary_texture is 0xFFFF
- **THEN** the system SHALL NOT emit any wall geometry for this side

### Requirement: High wall construction
The system SHALL construct wall geometry for sides with `side_type` = high (1). A high wall spans from the adjacent polygon's ceiling_height to this polygon's ceiling_height (the portion above the opening). The side's `primary_texture` SHALL be applied to this surface. If the primary texture descriptor is `0xFFFF`, the wall quad SHALL NOT be emitted.

#### Scenario: High wall above opening
- **WHEN** a line separates polygon A (ceiling=3072) from polygon B (ceiling=2048), and the side on polygon A has side_type=high with a valid primary_texture
- **THEN** the system creates a wall quad from Y=world_distance_to_f32(2048) to Y=world_distance_to_f32(3072)

#### Scenario: High wall with no texture
- **WHEN** a line has a side with side_type=high and primary_texture is 0xFFFF
- **THEN** the system SHALL NOT emit any wall geometry for this side

### Requirement: Low wall construction
The system SHALL construct wall geometry for sides with `side_type` = low (2). A low wall spans from this polygon's floor_height to the adjacent polygon's floor_height (the portion below the opening). The side's `primary_texture` SHALL be applied. If the primary texture descriptor is `0xFFFF`, the wall quad SHALL NOT be emitted.

#### Scenario: Low wall below opening
- **WHEN** a line separates polygon A (floor=0) from polygon B (floor=1024), and the side on polygon A has side_type=low with a valid primary_texture
- **THEN** the system creates a wall quad from Y=world_distance_to_f32(0) to Y=world_distance_to_f32(1024)

#### Scenario: Low wall with no texture
- **WHEN** a line has a side with side_type=low and primary_texture is 0xFFFF
- **THEN** the system SHALL NOT emit any wall geometry for this side

### Requirement: Split wall construction
The system SHALL construct wall geometry for sides with `side_type` = split (3). A split wall has both a high section (primary_texture) and a low section (secondary_texture) with a transparent middle section (transparent_texture) where adjacent polygons overlap in height range. Each section SHALL only be emitted if its respective texture descriptor is not `0xFFFF`.

#### Scenario: Split wall with three sections
- **WHEN** a line separates polygon A (floor=0, ceiling=4096) from polygon B (floor=1024, ceiling=3072), and the side has side_type=split with all three textures valid
- **THEN** the system creates: a low quad (0 to 1024) with secondary_texture, a transparent quad (1024 to 3072) with transparent_texture, and a high quad (3072 to 4096) with primary_texture

#### Scenario: Split wall with missing transparent texture
- **WHEN** a split wall side has transparent_texture = 0xFFFF but valid primary and secondary textures
- **THEN** the system emits the high and low quads but SHALL NOT emit the transparent middle section
