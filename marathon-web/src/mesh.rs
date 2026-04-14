use bytemuck::{Pod, Zeroable};
use marathon_formats::MapData;

/// Per-polygon lighting and transfer data, precomputed for baking into vertices.
pub struct PolygonInfo {
    pub floor_light: f32,
    pub floor_transfer_mode: u32,
    pub ceiling_light: f32,
    pub ceiling_transfer_mode: u32,
}

/// GPU vertex format: position + UV + texture descriptor + light + transfer mode.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub texture_descriptor: u32,
    pub light: f32,
    pub transfer_mode: u32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
        3 => Float32,
        4 => Uint32,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// A batch of triangles sharing the same texture collection.
pub struct DrawBatch {
    pub collection_index: u16,
    /// Range into the index buffer (start..end).
    pub index_start: u32,
    pub index_count: u32,
}

/// Result of converting a level's geometry to GPU-ready mesh data.
pub struct LevelMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Draw batches grouped by texture collection, sorted by collection index.
    pub batches: Vec<DrawBatch>,
}

/// Convert Marathon world distance (i16, 1024 = 1 world unit) to f32.
fn world_to_f32(v: i16) -> f32 {
    v as f32 / 1024.0
}

/// Build all geometry for a level: floors, ceilings, and walls.
pub fn build_level_mesh(map: &MapData, poly_info: &[PolygonInfo]) -> LevelMesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (poly_idx, polygon) in map.polygons.iter().enumerate() {
        let vert_count = polygon.vertex_count as usize;
        if vert_count < 3 {
            continue;
        }

        let info = &poly_info[poly_idx];

        build_floor(&mut vertices, &mut indices, map, polygon, info, vert_count);
        build_ceiling(&mut vertices, &mut indices, map, polygon, info, vert_count);

        if polygon.media_index >= 0 {
            if let Some(media) = map.media.get(polygon.media_index as usize) {
                build_media_surface(
                    &mut vertices, &mut indices, map, polygon, info, vert_count, media,
                );
            }
        }
    }

    for line in &map.lines {
        build_walls_for_line(&mut vertices, &mut indices, map, line, poly_info);
    }

    // Group triangles by collection for batched rendering.
    // Each triangle's collection is determined by the first vertex's texture_descriptor.
    let batches = build_draw_batches(&vertices, &mut indices);

    LevelMesh { vertices, indices, batches }
}

/// Sort triangle indices by texture collection and return draw batches.
fn build_draw_batches(vertices: &[Vertex], indices: &mut Vec<u32>) -> Vec<DrawBatch> {
    if indices.is_empty() {
        return Vec::new();
    }

    // Extract collection from texture_descriptor: bits[12:8]
    let collection_of = |idx: u32| -> u16 {
        let desc = vertices[idx as usize].texture_descriptor;
        ((desc >> 8) & 0x1F) as u16
    };

    // Sort triangles (groups of 3 indices) by collection
    let mut triangles: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    triangles.sort_by_key(|tri| collection_of(tri[0]));

    // Rebuild sorted index buffer and record batches
    indices.clear();
    let mut batches = Vec::new();
    let mut current_coll = collection_of(triangles[0][0]);
    let mut batch_start = 0u32;

    for tri in &triangles {
        let coll = collection_of(tri[0]);
        if coll != current_coll {
            let count = indices.len() as u32 - batch_start;
            if count > 0 {
                batches.push(DrawBatch {
                    collection_index: current_coll,
                    index_start: batch_start,
                    index_count: count,
                });
            }
            current_coll = coll;
            batch_start = indices.len() as u32;
        }
        indices.extend_from_slice(tri);
    }

    // Final batch
    let count = indices.len() as u32 - batch_start;
    if count > 0 {
        batches.push(DrawBatch {
            collection_index: current_coll,
            index_start: batch_start,
            index_count: count,
        });
    }

    batches
}

fn build_floor(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    polygon: &marathon_formats::Polygon,
    info: &PolygonInfo,
    vert_count: usize,
) {
    let base = vertices.len() as u32;
    let floor_y = world_to_f32(polygon.floor_height);
    let tex_desc = polygon.floor_texture.0 as u32;

    let mut actual_verts = 0u32;
    for i in 0..vert_count {
        let ep_idx = polygon.endpoint_indexes[i];
        if ep_idx < 0 {
            continue;
        }
        let ep = &map.endpoints[ep_idx as usize];
        let wx = world_to_f32(ep.vertex.x);
        let wz = -world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(polygon.floor_origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(polygon.floor_origin.y)) as f32 / 1024.0;

        vertices.push(Vertex {
            position: [wx, floor_y, wz],
            uv: [u, v],
            texture_descriptor: tex_desc,
            light: info.floor_light,
            transfer_mode: info.floor_transfer_mode,
        });
        actual_verts += 1;
    }

    if actual_verts >= 3 {
        for i in 1..actual_verts - 1 {
            indices.push(base);
            indices.push(base + i + 1);
            indices.push(base + i);
        }
    }
}

fn build_ceiling(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    polygon: &marathon_formats::Polygon,
    info: &PolygonInfo,
    vert_count: usize,
) {
    let base = vertices.len() as u32;
    let ceil_y = world_to_f32(polygon.ceiling_height);
    let tex_desc = polygon.ceiling_texture.0 as u32;

    let mut actual_verts = 0u32;
    for i in 0..vert_count {
        let ep_idx = polygon.endpoint_indexes[i];
        if ep_idx < 0 {
            continue;
        }
        let ep = &map.endpoints[ep_idx as usize];
        let wx = world_to_f32(ep.vertex.x);
        let wz = -world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(polygon.ceiling_origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(polygon.ceiling_origin.y)) as f32 / 1024.0;

        vertices.push(Vertex {
            position: [wx, ceil_y, wz],
            uv: [u, v],
            texture_descriptor: tex_desc,
            light: info.ceiling_light,
            transfer_mode: info.ceiling_transfer_mode,
        });
        actual_verts += 1;
    }

    if actual_verts >= 3 {
        for i in 1..actual_verts - 1 {
            indices.push(base);
            indices.push(base + i);
            indices.push(base + i + 1);
        }
    }
}

fn build_media_surface(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    polygon: &marathon_formats::Polygon,
    info: &PolygonInfo,
    vert_count: usize,
    media: &marathon_formats::MediaData,
) {
    let base = vertices.len() as u32;
    let media_y = world_to_f32(media.height);
    let tex_desc = media.texture.0 as u32;

    let mut actual_verts = 0u32;
    for i in 0..vert_count {
        let ep_idx = polygon.endpoint_indexes[i];
        if ep_idx < 0 {
            continue;
        }
        let ep = &map.endpoints[ep_idx as usize];
        let wx = world_to_f32(ep.vertex.x);
        let wz = -world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(media.origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(media.origin.y)) as f32 / 1024.0;

        vertices.push(Vertex {
            position: [wx, media_y, wz],
            uv: [u, v],
            texture_descriptor: tex_desc,
            light: info.floor_light,
            transfer_mode: media.transfer_mode as u32,
        });
        actual_verts += 1;
    }

    if actual_verts >= 3 {
        for i in 1..actual_verts - 1 {
            indices.push(base);
            indices.push(base + i + 1);
            indices.push(base + i);
        }
    }
}

fn build_walls_for_line(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    line: &marathon_formats::Line,
    poly_info: &[PolygonInfo],
) {
    if line.clockwise_polygon_side_index >= 0 && line.clockwise_polygon_owner >= 0 {
        let side_idx = line.clockwise_polygon_side_index as usize;
        let poly_idx = line.clockwise_polygon_owner as usize;
        if let Some(side) = map.sides.get(side_idx) {
            let adjacent_poly_idx = if line.counterclockwise_polygon_owner >= 0 {
                Some(line.counterclockwise_polygon_owner as usize)
            } else {
                None
            };
            build_wall_side(vertices, indices, map, line, side, poly_idx, adjacent_poly_idx, false, poly_info);
        }
    }

    if line.counterclockwise_polygon_side_index >= 0 && line.counterclockwise_polygon_owner >= 0 {
        let side_idx = line.counterclockwise_polygon_side_index as usize;
        let poly_idx = line.counterclockwise_polygon_owner as usize;
        if let Some(side) = map.sides.get(side_idx) {
            let adjacent_poly_idx = if line.clockwise_polygon_owner >= 0 {
                Some(line.clockwise_polygon_owner as usize)
            } else {
                None
            };
            build_wall_side(vertices, indices, map, line, side, poly_idx, adjacent_poly_idx, true, poly_info);
        }
    }
}

fn build_wall_side(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    line: &marathon_formats::Line,
    side: &marathon_formats::Side,
    poly_idx: usize,
    adjacent_poly_idx: Option<usize>,
    reverse_endpoints: bool,
    poly_info: &[PolygonInfo],
) {
    let polygon = &map.polygons[poly_idx];
    let info = &poly_info[poly_idx];

    let (ep0_idx, ep1_idx) = if reverse_endpoints {
        (line.endpoint_indexes[1], line.endpoint_indexes[0])
    } else {
        (line.endpoint_indexes[0], line.endpoint_indexes[1])
    };

    let ep0 = &map.endpoints[ep0_idx as usize];
    let ep1 = &map.endpoints[ep1_idx as usize];

    let x0 = world_to_f32(ep0.vertex.x);
    let z0 = -world_to_f32(ep0.vertex.y);
    let x1 = world_to_f32(ep1.vertex.x);
    let z1 = -world_to_f32(ep1.vertex.y);

    let wall_len = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt();

    match side.side_type {
        0 => {
            if !side.primary_texture.texture.is_none() {
                let bottom = world_to_f32(polygon.floor_height);
                let top = world_to_f32(polygon.ceiling_height);
                let tex = &side.primary_texture;
                emit_wall_quad(
                    vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                    tex.texture.0 as u32, info.floor_light, side.primary_transfer_mode as u32,
                );
            }
        }
        1 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];
                let bottom = world_to_f32(adj.ceiling_height);
                let top = world_to_f32(polygon.ceiling_height);
                if top > bottom && !side.primary_texture.texture.is_none() {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                        tex.texture.0 as u32, info.floor_light, side.primary_transfer_mode as u32,
                    );
                }
            }
        }
        2 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];
                let bottom = world_to_f32(polygon.floor_height);
                let top = world_to_f32(adj.floor_height);
                if top > bottom && !side.primary_texture.texture.is_none() {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                        tex.texture.0 as u32, info.floor_light, side.primary_transfer_mode as u32,
                    );
                }
            }
        }
        3 | 4 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];

                let low_bottom = world_to_f32(polygon.floor_height);
                let low_top = world_to_f32(adj.floor_height);
                if low_top > low_bottom && !side.secondary_texture.texture.is_none() {
                    let tex = &side.secondary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, low_bottom, low_top, wall_len, tex,
                        tex.texture.0 as u32, info.floor_light, side.secondary_transfer_mode as u32,
                    );
                }

                let trans_bottom = world_to_f32(adj.floor_height);
                let trans_top = world_to_f32(adj.ceiling_height);
                if trans_top > trans_bottom && !side.transparent_texture.texture.is_none() {
                    let tex = &side.transparent_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, trans_bottom, trans_top, wall_len,
                        tex, tex.texture.0 as u32, info.floor_light, side.transparent_transfer_mode as u32,
                    );
                }

                let high_bottom = world_to_f32(adj.ceiling_height);
                let high_top = world_to_f32(polygon.ceiling_height);
                if high_top > high_bottom && !side.primary_texture.texture.is_none() {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, high_bottom, high_top, wall_len,
                        tex, tex.texture.0 as u32, info.floor_light, side.primary_transfer_mode as u32,
                    );
                }
            }
        }
        _ => {}
    }
}

fn emit_wall_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    bottom: f32,
    top: f32,
    wall_len: f32,
    side_tex: &marathon_formats::SideTexture,
    tex_desc: u32,
    light: f32,
    transfer_mode: u32,
) {
    let base = vertices.len() as u32;
    let height = top - bottom;
    let u_off = side_tex.x0 as f32 / 1024.0;
    let v_off = side_tex.y0 as f32 / 1024.0;

    vertices.push(Vertex {
        position: [x0, bottom, z0],
        uv: [u_off, v_off + height],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
    });
    vertices.push(Vertex {
        position: [x0, top, z0],
        uv: [u_off, v_off],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
    });
    vertices.push(Vertex {
        position: [x1, top, z1],
        uv: [u_off + wall_len, v_off],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
    });
    vertices.push(Vertex {
        position: [x1, bottom, z1],
        uv: [u_off + wall_len, v_off + height],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
    });

    indices.push(base);
    indices.push(base + 1);
    indices.push(base + 2);
    indices.push(base);
    indices.push(base + 2);
    indices.push(base + 3);
}

#[cfg(test)]
mod tests {
    use super::*;
    use marathon_formats::{Endpoint, Line, Polygon, Side, WorldPoint2d, ShapeDescriptor, SideTexture};
    use marathon_formats::map::LightData;

    fn make_endpoint(x: i16, y: i16) -> Endpoint {
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 0,
            vertex: WorldPoint2d { x, y },
            transformed: WorldPoint2d { x, y },
            supporting_polygon_index: -1,
        }
    }

    fn make_polygon(vertex_count: u16, endpoint_indexes: [i16; 8]) -> Polygon {
        Polygon {
            polygon_type: 0,
            flags: 0,
            permutation: 0,
            vertex_count,
            endpoint_indexes,
            line_indexes: [-1; 8],
            floor_texture: ShapeDescriptor(0x0100),
            ceiling_texture: ShapeDescriptor(0x0100),
            floor_height: 0,
            ceiling_height: 1024,
            floor_lightsource_index: 0,
            ceiling_lightsource_index: 0,
            area: 0,
            floor_transfer_mode: 0,
            ceiling_transfer_mode: 0,
            adjacent_polygon_indexes: [-1; 8],
            center: WorldPoint2d { x: 0, y: 0 },
            side_indexes: [-1; 8],
            floor_origin: WorldPoint2d { x: 0, y: 0 },
            ceiling_origin: WorldPoint2d { x: 0, y: 0 },
            media_index: -1,
            media_lightsource_index: -1,
            sound_source_indexes: -1,
            ambient_sound_image_index: -1,
            random_sound_image_index: -1,
        }
    }

    fn make_side_texture(descriptor: u16) -> SideTexture {
        SideTexture {
            x0: 0,
            y0: 0,
            texture: ShapeDescriptor(descriptor),
        }
    }

    fn make_side(side_type: i16, primary_desc: u16) -> Side {
        Side {
            side_type,
            flags: 0,
            primary_texture: make_side_texture(primary_desc),
            secondary_texture: make_side_texture(0xFFFF),
            transparent_texture: make_side_texture(0xFFFF),
            exclusion_zone: [WorldPoint2d { x: 0, y: 0 }; 4],
            control_panel_type: 0,
            control_panel_permutation: 0,
            primary_transfer_mode: 0,
            secondary_transfer_mode: 0,
            transparent_transfer_mode: 0,
            polygon_index: 0,
            line_index: 0,
            primary_lightsource_index: 0,
            secondary_lightsource_index: 0,
            transparent_lightsource_index: 0,
            ambient_delta: 0,
        }
    }

    fn make_map_data(endpoints: Vec<Endpoint>, polygons: Vec<Polygon>, lines: Vec<Line>, sides: Vec<Side>) -> MapData {
        MapData {
            endpoints,
            lines,
            sides,
            polygons,
            objects: vec![],
            lights: LightData::None,
            platforms: vec![],
            media: vec![],
            annotations: vec![],
            terminals: vec![],
            ambient_sounds: vec![],
            random_sounds: vec![],
            map_info: None,
            item_placement: vec![],
            guard_paths: None,
        }
    }

    fn make_info() -> PolygonInfo {
        PolygonInfo {
            floor_light: 1.0,
            floor_transfer_mode: 0,
            ceiling_light: 1.0,
            ceiling_transfer_mode: 0,
        }
    }

    #[test]
    fn world_to_f32_conversion() {
        assert_eq!(world_to_f32(0), 0.0);
        assert_eq!(world_to_f32(1024), 1.0);
        assert_eq!(world_to_f32(-1024), -1.0);
        assert_eq!(world_to_f32(512), 0.5);
    }

    #[test]
    fn vertex_size_matches_gpu_layout() {
        // 3 floats (pos) + 2 floats (uv) + 1 u32 (tex_desc) + 1 float (light) + 1 u32 (transfer) = 8 * 4 = 32 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
    }

    #[test]
    fn vertex_is_pod() {
        // Ensure Vertex can be safely cast to bytes for GPU upload
        let v = Vertex {
            position: [1.0, 2.0, 3.0],
            uv: [0.5, 0.5],
            texture_descriptor: 42,
            light: 0.8,
            transfer_mode: 0,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn floor_triangulation_skips_negative_one_endpoints() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ];
        let polygon = make_polygon(5, [0, 1, -1, 2, 3, -1, -1, -1]);
        let map = make_map_data(endpoints, vec![polygon], vec![], vec![]);
        let info = make_info();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_floor(&mut vertices, &mut indices, &map, &map.polygons[0], &info, 5);

        assert_eq!(vertices.len(), 4, "should emit 4 vertices (skipping -1)");
        assert_eq!(indices.len(), 6, "should emit 2 triangles (6 indices) from 4 verts");
    }

    #[test]
    fn floor_triangulation_too_few_valid_verts_produces_nothing() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
        ];
        let polygon = make_polygon(4, [0, -1, 1, -1, -1, -1, -1, -1]);
        let map = make_map_data(endpoints, vec![polygon], vec![], vec![]);
        let info = make_info();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_floor(&mut vertices, &mut indices, &map, &map.polygons[0], &info, 4);

        assert_eq!(vertices.len(), 2, "should emit 2 vertices");
        assert_eq!(indices.len(), 0, "should emit 0 triangles (not enough verts)");
    }

    #[test]
    fn wall_none_texture_produces_no_geometry() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
        ];
        let polygon = make_polygon(4, [0, 1, -1, -1, -1, -1, -1, -1]);
        let side = make_side(0, 0xFFFF);
        let line = Line {
            endpoint_indexes: [0, 1],
            flags: 0,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 1024,
            clockwise_polygon_side_index: 0,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        };
        let map = make_map_data(endpoints, vec![polygon], vec![line], vec![side]);
        let poly_info = vec![make_info()];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_walls_for_line(&mut vertices, &mut indices, &map, &map.lines[0], &poly_info);

        assert_eq!(vertices.len(), 0, "none-texture wall should produce 0 vertices");
        assert_eq!(indices.len(), 0, "none-texture wall should produce 0 indices");
    }

    #[test]
    fn wall_valid_texture_produces_quad() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
        ];
        let polygon = make_polygon(4, [0, 1, -1, -1, -1, -1, -1, -1]);
        let side = make_side(0, 0x0100);
        let line = Line {
            endpoint_indexes: [0, 1],
            flags: 0,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 1024,
            clockwise_polygon_side_index: 0,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        };
        let map = make_map_data(endpoints, vec![polygon], vec![line], vec![side]);
        let poly_info = vec![make_info()];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_walls_for_line(&mut vertices, &mut indices, &map, &map.lines[0], &poly_info);

        assert_eq!(vertices.len(), 4, "valid-texture wall should produce 4 vertices");
        assert_eq!(indices.len(), 6, "valid-texture wall should produce 6 indices");
    }
}
