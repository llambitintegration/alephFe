## MODIFIED Requirements

### Requirement: Ceiling surface triangulation
The system SHALL convert each polygon's ceiling into a triangle mesh using the same fan triangulation as floors, with the same handling of `-1` sentinel endpoint values. The Y coordinate SHALL be the polygon's `ceiling_height`. The winding order SHALL be reversed (or face culling adjusted) so that the ceiling faces downward. UV coordinates SHALL be computed from the polygon's `ceiling_origin` offset. Each ceiling vertex SHALL carry the polygon's ceiling transfer mode and ceiling light intensity, independent of the floor's values.

#### Scenario: Ceiling faces downward
- **WHEN** a polygon has floor_height 0 and ceiling_height 2048
- **THEN** the ceiling mesh is at Y = world_distance_to_f32(2048) with normals pointing downward (reversed winding relative to floor)

#### Scenario: Ceiling with sparse endpoints
- **WHEN** a polygon has vertex_count=6 and endpoint_indexes=[0, 1, -1, 3, 4, 5]
- **THEN** the system emits 5 vertices (skipping the -1 entry) and produces 3 ceiling triangles with reversed winding

#### Scenario: Ceiling uses own transfer mode
- **WHEN** a polygon has floor_transfer_mode=0 (normal) and ceiling_transfer_mode=9 (landscape)
- **THEN** floor vertices have transfer_mode=0 and ceiling vertices have transfer_mode=9

#### Scenario: Ceiling uses own light intensity
- **WHEN** a polygon has floor_light=1.0 and ceiling_light=0.5
- **THEN** floor vertices have light=1.0 and ceiling vertices have light=0.5

### Requirement: Full wall construction
The system SHALL construct wall geometry for sides with `side_type` = full (0). A full wall spans from the polygon's floor_height to ceiling_height. The system SHALL build a quad (two triangles) between the two line endpoints at these heights, using the side's `primary_texture` for UV mapping. If the primary texture descriptor is `0xFFFF` (none), the wall quad SHALL NOT be emitted. Wall vertices SHALL carry the side's primary transfer mode, not the polygon's floor transfer mode.

#### Scenario: Full wall between floor and ceiling
- **WHEN** a line has a side with side_type=full, the polygon has floor_height=0 and ceiling_height=2048, and primary_texture is valid
- **THEN** the system creates a quad from floor to ceiling height using the primary texture

#### Scenario: Full wall with no texture
- **WHEN** a line has a side with side_type=full and primary_texture is 0xFFFF
- **THEN** the system SHALL NOT emit any wall geometry for this side

#### Scenario: Wall uses side transfer mode
- **WHEN** a side has primary_transfer_mode=4 (slide) and the polygon has floor_transfer_mode=0 (normal)
- **THEN** wall vertices carry transfer_mode=4, not transfer_mode=0
