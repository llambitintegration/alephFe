## 1. Fix Floor/Ceiling Triangulation

- [x] 1.1 In marathon-web/src/mesh.rs `build_floor()`: track actual emitted vertex count separately; use it (not vert_count) for fan triangulation loop; skip polygon if actual_verts < 3
- [x] 1.2 Apply same fix to `build_ceiling()` in marathon-web/src/mesh.rs
- [x] 1.3 Apply same floor fix to marathon-game/src/mesh.rs
- [x] 1.4 Apply same ceiling fix to marathon-game/src/mesh.rs
- [x] 1.5 Apply same floor fix to marathon-viewer/src/mesh.rs
- [x] 1.6 Apply same ceiling fix to marathon-viewer/src/mesh.rs

## 2. Filter None-Texture Walls

- [x] 2.1 In marathon-web/src/mesh.rs wall builder: add `is_none()` check on primary_texture before emitting full (type 0), high (type 1), and low (type 2) wall quads
- [x] 2.2 In marathon-web/src/mesh.rs: add `is_none()` check on secondary_texture for split wall low section, and on transparent_texture for split wall transparent section (primary already checked in 2.1)
- [x] 2.3 Apply same wall texture filtering to marathon-game/src/mesh.rs
- [x] 2.4 Apply same wall texture filtering to marathon-viewer/src/mesh.rs

## 3. Unit Tests

- [x] 3.1 Add test in marathon-web: polygon with 5 declared vertices, one endpoint_index=-1 → produces correct triangle count (2 triangles from 4 valid verts)
- [x] 3.2 Add test: polygon with vertex_count=4, two endpoints -1 → produces 0 triangles (only 2 valid verts)
- [x] 3.3 Add test: wall side with primary_texture=0xFFFF produces 0 wall vertices
- [x] 3.4 Add test: wall side with valid primary_texture produces expected wall quad (4 verts, 6 indices)

## 4. Integration Testing

- [x] 4.1 Run full cargo test suite in Docker and verify all existing + new tests pass
- [x] 4.2 Deploy to marathon.llambit.io and verify Waterloo Waterpark renders with no visible geometry gaps
