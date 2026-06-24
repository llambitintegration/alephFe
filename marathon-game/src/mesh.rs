use bytemuck::{Pod, Zeroable};
use marathon_formats::MapData;

/// Wall vertex height-source surface selector (high bit of `Vertex::height_source`).
///
/// A wall vertex's top/bottom Y can be driven by either the floor or the ceiling
/// of its *source* polygon (which may be the owning polygon or an adjacent
/// neighbor — see design.md "Wall height-source representation (box 5.1)"). The
/// 1-bit selector lives in bit 31; the source polygon index is the low 31 bits.
pub const WALL_HEIGHT_FROM_FLOOR: u32 = 0;
pub const WALL_HEIGHT_FROM_CEILING: u32 = 1;

/// High bit of `Vertex::texture_descriptor` flagging a media-surface vertex.
/// Media vertices set this bit so the renderer/shader can distinguish them from
/// opaque geometry (apply media height override + alpha-blended visuals). The
/// real texture id lives in the low 31 bits; the shader masks this off before
/// sampling.
pub const MEDIA_VERTEX_FLAG: u32 = 0x8000_0000;

/// Bit position of the floor/ceiling selector inside `Vertex::height_source`.
const HEIGHT_SOURCE_SELECTOR_SHIFT: u32 = 31;
/// Mask covering the source-polygon-index bits of `Vertex::height_source`.
pub const HEIGHT_SOURCE_INDEX_MASK: u32 = 0x7FFF_FFFF;

/// Pack a height-source descriptor: which polygon's floor/ceiling drives this
/// vertex's Y. Mirrors the floor/ceiling/media surface-discriminator pattern but
/// uses a dedicated attribute because a wall vertex's `polygon_index` (kept for
/// light/transfer-mode sampling = the owning polygon) is *not* necessarily the
/// source polygon.
pub fn pack_height_source(selector: u32, source_polygon_index: u32) -> u32 {
    (selector << HEIGHT_SOURCE_SELECTOR_SHIFT) | (source_polygon_index & HEIGHT_SOURCE_INDEX_MASK)
}

/// Extract the floor/ceiling selector from a packed `height_source`.
/// Used by tests and (structurally) mirrors the shader-side unpack in box 6.2.
#[allow(dead_code)]
pub fn height_source_selector(height_source: u32) -> u32 {
    height_source >> HEIGHT_SOURCE_SELECTOR_SHIFT
}

/// Extract the source polygon index from a packed `height_source`.
/// Used by tests and (structurally) mirrors the shader-side unpack in box 6.2.
#[allow(dead_code)]
pub fn height_source_index(height_source: u32) -> u32 {
    height_source & HEIGHT_SOURCE_INDEX_MASK
}

/// GPU vertex format: position + UV + polygon index + texture descriptor +
/// height source. `height_source` names the polygon (and floor-vs-ceiling
/// surface) whose animated height drives this vertex's Y; box 6.2 resolves Y
/// from `polygon_data[source]` in the shader instead of the baked `position.y`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub polygon_index: u32,
    pub texture_descriptor: u32,
    pub height_source: u32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
        3 => Uint32,
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

/// Result of converting a level's geometry to GPU-ready mesh data.
pub struct LevelMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Number of indices belonging to opaque geometry (floors, ceilings, walls),
    /// which are emitted FIRST. Indices in `0..opaque_index_count` are opaque;
    /// indices in `opaque_index_count..indices.len()` are alpha-blended media
    /// surfaces. Lets the renderer draw opaque-then-media in two sub-passes.
    pub opaque_index_count: u32,
}

/// Convert Marathon world distance (i16, 1024 = 1 world unit) to f32.
fn world_to_f32(v: i16) -> f32 {
    v as f32 / 1024.0
}

/// Build all geometry for a level: floors, ceilings, and walls.
pub fn build_level_mesh(map: &MapData) -> LevelMesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // --- Opaque pass: floors + ceilings for every polygon, then all walls. ---
    for (poly_idx, polygon) in map.polygons.iter().enumerate() {
        let vert_count = polygon.vertex_count as usize;
        if vert_count < 3 {
            continue;
        }

        build_floor(
            &mut vertices,
            &mut indices,
            map,
            polygon,
            poly_idx,
            vert_count,
        );
        build_ceiling(
            &mut vertices,
            &mut indices,
            map,
            polygon,
            poly_idx,
            vert_count,
        );
    }

    for line in &map.lines {
        build_walls_for_line(&mut vertices, &mut indices, map, line);
    }

    // Boundary between opaque and media geometry: every index emitted so far is
    // opaque; media surfaces are appended after this point.
    let opaque_index_count = indices.len() as u32;

    // --- Media pass: alpha-blended media surfaces for every polygon. ---
    for (poly_idx, polygon) in map.polygons.iter().enumerate() {
        let vert_count = polygon.vertex_count as usize;
        if vert_count < 3 {
            continue;
        }

        if polygon.media_index >= 0 {
            if let Some(media) = map.media.get(polygon.media_index as usize) {
                build_media_surface(
                    &mut vertices,
                    &mut indices,
                    map,
                    polygon,
                    poly_idx,
                    vert_count,
                    media,
                );
            }
        }
    }

    LevelMesh {
        vertices,
        indices,
        opaque_index_count,
    }
}

fn build_floor(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    polygon: &marathon_formats::Polygon,
    poly_idx: usize,
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
        let wz = world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(polygon.floor_origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(polygon.floor_origin.y)) as f32 / 1024.0;

        vertices.push(Vertex {
            position: [wx, floor_y, wz],
            uv: [u, v],
            polygon_index: poly_idx as u32,
            texture_descriptor: tex_desc,
            height_source: pack_height_source(WALL_HEIGHT_FROM_FLOOR, poly_idx as u32),
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
    poly_idx: usize,
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
        let wz = world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(polygon.ceiling_origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(polygon.ceiling_origin.y)) as f32 / 1024.0;

        vertices.push(Vertex {
            position: [wx, ceil_y, wz],
            uv: [u, v],
            polygon_index: poly_idx as u32,
            texture_descriptor: tex_desc,
            height_source: pack_height_source(WALL_HEIGHT_FROM_CEILING, poly_idx as u32),
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
    poly_idx: usize,
    vert_count: usize,
    media: &marathon_formats::MediaData,
) {
    let base = vertices.len() as u32;
    let media_y = world_to_f32(media.height);
    // Flag this as a media-surface vertex by setting bit 31 of the texture
    // descriptor (low bits stay the real texture id). The shader masks bit 31
    // off before sampling and uses it to apply media height/visual overrides.
    let tex_desc = (media.texture.0 as u32) | MEDIA_VERTEX_FLAG;

    let mut actual_verts = 0u32;
    for i in 0..vert_count {
        let ep_idx = polygon.endpoint_indexes[i];
        if ep_idx < 0 {
            continue;
        }
        let ep = &map.endpoints[ep_idx as usize];
        let wx = world_to_f32(ep.vertex.x);
        let wz = world_to_f32(ep.vertex.y);
        let u = (ep.vertex.x.wrapping_sub(media.origin.x)) as f32 / 1024.0;
        let v = (ep.vertex.y.wrapping_sub(media.origin.y)) as f32 / 1024.0;

        // Media surfaces keep baked Y; box 6.2 only un-bakes walls. Tag with the
        // owning polygon's floor selector for forward consistency (unused by the
        // shader for media).
        vertices.push(Vertex {
            position: [wx, media_y, wz],
            uv: [u, v],
            polygon_index: poly_idx as u32,
            texture_descriptor: tex_desc,
            height_source: pack_height_source(WALL_HEIGHT_FROM_FLOOR, poly_idx as u32),
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
            build_wall_side(
                vertices,
                indices,
                map,
                line,
                side,
                poly_idx,
                adjacent_poly_idx,
                false,
            );
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
            build_wall_side(
                vertices,
                indices,
                map,
                line,
                side,
                poly_idx,
                adjacent_poly_idx,
                true,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_wall_side(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    line: &marathon_formats::Line,
    side: &marathon_formats::Side,
    poly_idx: usize,
    adjacent_poly_idx: Option<usize>,
    reverse_endpoints: bool,
) {
    let polygon = &map.polygons[poly_idx];

    let (ep0_idx, ep1_idx) = if reverse_endpoints {
        (line.endpoint_indexes[1], line.endpoint_indexes[0])
    } else {
        (line.endpoint_indexes[0], line.endpoint_indexes[1])
    };

    let ep0 = &map.endpoints[ep0_idx as usize];
    let ep1 = &map.endpoints[ep1_idx as usize];

    let x0 = world_to_f32(ep0.vertex.x);
    let z0 = world_to_f32(ep0.vertex.y);
    let x1 = world_to_f32(ep1.vertex.x);
    let z1 = world_to_f32(ep1.vertex.y);

    let wall_len = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt();

    match side.side_type {
        0 => {
            if !side.primary_texture.texture.is_none() {
                let bottom = world_to_f32(polygon.floor_height);
                let top = world_to_f32(polygon.ceiling_height);
                let tex = &side.primary_texture;
                // Full wall: bottom = own floor, top = own ceiling.
                emit_wall_quad(
                    vertices,
                    indices,
                    x0,
                    z0,
                    x1,
                    z1,
                    bottom,
                    top,
                    wall_len,
                    tex,
                    tex.texture.0 as u32,
                    poly_idx,
                    pack_height_source(WALL_HEIGHT_FROM_FLOOR, poly_idx as u32),
                    pack_height_source(WALL_HEIGHT_FROM_CEILING, poly_idx as u32),
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
                    // High wall: bottom = neighbor ceiling, top = own ceiling.
                    emit_wall_quad(
                        vertices,
                        indices,
                        x0,
                        z0,
                        x1,
                        z1,
                        bottom,
                        top,
                        wall_len,
                        tex,
                        tex.texture.0 as u32,
                        poly_idx,
                        pack_height_source(WALL_HEIGHT_FROM_CEILING, adj_idx as u32),
                        pack_height_source(WALL_HEIGHT_FROM_CEILING, poly_idx as u32),
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
                    // Low wall: bottom = own floor, top = neighbor floor.
                    emit_wall_quad(
                        vertices,
                        indices,
                        x0,
                        z0,
                        x1,
                        z1,
                        bottom,
                        top,
                        wall_len,
                        tex,
                        tex.texture.0 as u32,
                        poly_idx,
                        pack_height_source(WALL_HEIGHT_FROM_FLOOR, poly_idx as u32),
                        pack_height_source(WALL_HEIGHT_FROM_FLOOR, adj_idx as u32),
                    );
                }
            }
        }
        3 | 4 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];

                // Split wall, low quad: bottom = own floor, top = neighbor floor.
                let low_bottom = world_to_f32(polygon.floor_height);
                let low_top = world_to_f32(adj.floor_height);
                if low_top > low_bottom && !side.secondary_texture.texture.is_none() {
                    let tex = &side.secondary_texture;
                    emit_wall_quad(
                        vertices,
                        indices,
                        x0,
                        z0,
                        x1,
                        z1,
                        low_bottom,
                        low_top,
                        wall_len,
                        tex,
                        tex.texture.0 as u32,
                        poly_idx,
                        pack_height_source(WALL_HEIGHT_FROM_FLOOR, poly_idx as u32),
                        pack_height_source(WALL_HEIGHT_FROM_FLOOR, adj_idx as u32),
                    );
                }

                // Split wall, transparent (middle) quad: bottom = neighbor floor,
                // top = neighbor ceiling.
                let trans_bottom = world_to_f32(adj.floor_height);
                let trans_top = world_to_f32(adj.ceiling_height);
                if trans_top > trans_bottom && !side.transparent_texture.texture.is_none() {
                    let tex = &side.transparent_texture;
                    emit_wall_quad(
                        vertices,
                        indices,
                        x0,
                        z0,
                        x1,
                        z1,
                        trans_bottom,
                        trans_top,
                        wall_len,
                        tex,
                        tex.texture.0 as u32,
                        poly_idx,
                        pack_height_source(WALL_HEIGHT_FROM_FLOOR, adj_idx as u32),
                        pack_height_source(WALL_HEIGHT_FROM_CEILING, adj_idx as u32),
                    );
                }

                // Split wall, high quad: bottom = neighbor ceiling, top = own ceiling.
                let high_bottom = world_to_f32(adj.ceiling_height);
                let high_top = world_to_f32(polygon.ceiling_height);
                if high_top > high_bottom && !side.primary_texture.texture.is_none() {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices,
                        indices,
                        x0,
                        z0,
                        x1,
                        z1,
                        high_bottom,
                        high_top,
                        wall_len,
                        tex,
                        tex.texture.0 as u32,
                        poly_idx,
                        pack_height_source(WALL_HEIGHT_FROM_CEILING, adj_idx as u32),
                        pack_height_source(WALL_HEIGHT_FROM_CEILING, poly_idx as u32),
                    );
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
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
    poly_idx: usize,
    bottom_source: u32,
    top_source: u32,
) {
    let base = vertices.len() as u32;
    let height = top - bottom;
    let u_off = side_tex.x0 as f32 / 1024.0;
    let v_off = side_tex.y0 as f32 / 1024.0;

    vertices.push(Vertex {
        position: [x0, bottom, z0],
        uv: [u_off, v_off + height],
        polygon_index: poly_idx as u32,
        texture_descriptor: tex_desc,
        height_source: bottom_source,
    });
    vertices.push(Vertex {
        position: [x0, top, z0],
        uv: [u_off, v_off],
        polygon_index: poly_idx as u32,
        texture_descriptor: tex_desc,
        height_source: top_source,
    });
    vertices.push(Vertex {
        position: [x1, top, z1],
        uv: [u_off + wall_len, v_off],
        polygon_index: poly_idx as u32,
        texture_descriptor: tex_desc,
        height_source: top_source,
    });
    vertices.push(Vertex {
        position: [x1, bottom, z1],
        uv: [u_off + wall_len, v_off + height],
        polygon_index: poly_idx as u32,
        texture_descriptor: tex_desc,
        height_source: bottom_source,
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
    use marathon_formats::map::LightData;
    use marathon_formats::{
        Endpoint, Line, Polygon, ShapeDescriptor, Side, SideTexture, WorldPoint2d,
    };

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

    fn make_line(
        endpoint_indexes: [i16; 2],
        cw_side: i16,
        ccw_side: i16,
        cw_owner: i16,
        ccw_owner: i16,
    ) -> Line {
        Line {
            endpoint_indexes,
            flags: 0,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 1024,
            clockwise_polygon_side_index: cw_side,
            counterclockwise_polygon_side_index: ccw_side,
            clockwise_polygon_owner: cw_owner,
            counterclockwise_polygon_owner: ccw_owner,
        }
    }

    fn make_map_data(
        endpoints: Vec<Endpoint>,
        polygons: Vec<Polygon>,
        lines: Vec<Line>,
        sides: Vec<Side>,
    ) -> MapData {
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

    #[test]
    fn height_source_packing_round_trips() {
        let f = pack_height_source(WALL_HEIGHT_FROM_FLOOR, 7);
        assert_eq!(height_source_selector(f), WALL_HEIGHT_FROM_FLOOR);
        assert_eq!(height_source_index(f), 7);

        let c = pack_height_source(WALL_HEIGHT_FROM_CEILING, 12345);
        assert_eq!(height_source_selector(c), WALL_HEIGHT_FROM_CEILING);
        assert_eq!(height_source_index(c), 12345);
    }

    /// Box 6.1 (red-green): a full wall (side_type 0) must emit vertices that
    /// carry the height-source discriminator + the owning polygon's index — the
    /// bottom edge sourced from the floor of polygon 0, the top edge from the
    /// ceiling of polygon 0 — rather than relying solely on baked absolute Y.
    #[test]
    fn full_wall_vertices_carry_height_source_discriminator() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
        let polygon = make_polygon(4, [0, 1, -1, -1, -1, -1, -1, -1]);
        let side = make_side(0, 0x0100);
        let line = make_line([0, 1], 0, -1, 0, -1);
        let map = make_map_data(endpoints, vec![polygon], vec![line], vec![side]);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_walls_for_line(&mut vertices, &mut indices, &map, &map.lines[0]);

        assert_eq!(vertices.len(), 4, "full wall emits one quad (4 vertices)");

        let expect_bottom = pack_height_source(WALL_HEIGHT_FROM_FLOOR, 0);
        let expect_top = pack_height_source(WALL_HEIGHT_FROM_CEILING, 0);

        for v in &vertices {
            // Bottom vertices sit at floor_height (0.0), top at ceiling (1.0).
            if v.position[1] == 0.0 {
                assert_eq!(
                    v.height_source, expect_bottom,
                    "bottom wall vertex must source polygon 0's floor"
                );
                assert_eq!(
                    height_source_selector(v.height_source),
                    WALL_HEIGHT_FROM_FLOOR
                );
                assert_eq!(height_source_index(v.height_source), 0);
            } else {
                assert_eq!(
                    v.height_source, expect_top,
                    "top wall vertex must source polygon 0's ceiling"
                );
                assert_eq!(
                    height_source_selector(v.height_source),
                    WALL_HEIGHT_FROM_CEILING
                );
                assert_eq!(height_source_index(v.height_source), 0);
            }
        }
    }

    /// Box 5.1 / 6.1 (red-green): a high wall (side_type 1) sources its BOTTOM
    /// edge from the *adjacent* (neighbor) polygon's ceiling, not the owning
    /// polygon. This proves the neighbor index is carried explicitly (a wall
    /// vertex's source polygon is not always `polygon_index`).
    #[test]
    fn high_wall_bottom_sources_neighbor_polygon() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
        // Owning polygon 0: ceiling at 2048. Neighbor polygon 1: ceiling at 1024.
        let mut p0 = make_polygon(4, [0, 1, -1, -1, -1, -1, -1, -1]);
        p0.ceiling_height = 2048;
        let mut p1 = make_polygon(4, [0, 1, -1, -1, -1, -1, -1, -1]);
        p1.ceiling_height = 1024;
        let side = make_side(1, 0x0100);
        // cw side 0 owned by poly 0; ccw owner is the neighbor poly 1.
        let line = make_line([0, 1], 0, -1, 0, 1);
        let map = make_map_data(endpoints, vec![p0, p1], vec![line], vec![side]);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_walls_for_line(&mut vertices, &mut indices, &map, &map.lines[0]);

        assert_eq!(vertices.len(), 4, "high wall emits one quad (4 vertices)");

        // bottom = neighbor (poly 1) ceiling; top = own (poly 0) ceiling.
        let expect_bottom = pack_height_source(WALL_HEIGHT_FROM_CEILING, 1);
        let expect_top = pack_height_source(WALL_HEIGHT_FROM_CEILING, 0);
        let bottom_y = world_to_f32(1024); // neighbor ceiling
        for v in &vertices {
            // polygon_index always names the owner (for light), never the source.
            assert_eq!(v.polygon_index, 0, "owner polygon_index unchanged");
            if (v.position[1] - bottom_y).abs() < 1e-6 {
                assert_eq!(
                    v.height_source, expect_bottom,
                    "high-wall bottom must source NEIGHBOR polygon 1's ceiling"
                );
                assert_eq!(height_source_index(v.height_source), 1);
            } else {
                assert_eq!(
                    v.height_source, expect_top,
                    "high-wall top must source OWN polygon 0's ceiling"
                );
                assert_eq!(height_source_index(v.height_source), 0);
            }
        }
    }

    /// The new attribute must not disturb floor/ceiling emission: those vertices
    /// carry their own surface selector + own polygon index.
    #[test]
    fn floor_and_ceiling_carry_own_height_source() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ];
        let polygon = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        let map = make_map_data(endpoints, vec![polygon], vec![], vec![]);

        let mut vf = Vec::new();
        let mut idx = Vec::new();
        build_floor(&mut vf, &mut idx, &map, &map.polygons[0], 0, 4);
        for v in &vf {
            assert_eq!(
                v.height_source,
                pack_height_source(WALL_HEIGHT_FROM_FLOOR, 0)
            );
        }

        let mut vc = Vec::new();
        let mut idc = Vec::new();
        build_ceiling(&mut vc, &mut idc, &map, &map.polygons[0], 0, 4);
        for v in &vc {
            assert_eq!(
                v.height_source,
                pack_height_source(WALL_HEIGHT_FROM_CEILING, 0)
            );
        }
    }

    fn make_media(texture: u16, height: i16) -> marathon_formats::MediaData {
        marathon_formats::MediaData {
            media_type: 0,
            flags: 0,
            light_index: 0,
            current_direction: 0,
            current_magnitude: 0,
            low: 0,
            high: 0,
            origin: WorldPoint2d { x: 0, y: 0 },
            height,
            minimum_light_intensity: 0.0,
            texture: ShapeDescriptor(texture),
            transfer_mode: 0,
        }
    }

    /// A square polygon (endpoints 0..4) referencing the given media index.
    fn make_square_polygon(media_index: i16) -> Polygon {
        let mut p = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        p.media_index = media_index;
        p
    }

    fn square_endpoints() -> Vec<Endpoint> {
        vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ]
    }

    /// Box 1.4: a level with 2 opaque polygons and 1 media polygon must emit all
    /// opaque indices first, with `opaque_index_count` marking the boundary; the
    /// media triangles follow AFTER that boundary and bring the total higher.
    #[test]
    fn opaque_index_count_marks_media_boundary() {
        // Three square polygons. Polygon 2 also has a media surface (media 0).
        let p0 = make_square_polygon(-1);
        let p1 = make_square_polygon(-1);
        let p2 = make_square_polygon(0);
        let mut map = make_map_data(square_endpoints(), vec![p0, p1, p2], vec![], vec![]);
        map.media = vec![make_media(0x0005, 512)];

        let mesh = build_level_mesh(&map);

        // Each square emits floor (2 tris = 6 idx) + ceiling (6 idx) = 12 opaque
        // indices per polygon. 3 polygons => 36 opaque indices, no walls.
        let expected_opaque = 36u32;
        assert_eq!(
            mesh.opaque_index_count, expected_opaque,
            "opaque_index_count must equal the opaque floor+ceiling index total"
        );

        // The media surface (1 square = 2 tris = 6 indices) is appended after the
        // boundary, so the total exceeds the opaque count by exactly the media
        // triangle indices.
        assert_eq!(
            mesh.indices.len() as u32,
            expected_opaque + 6,
            "media indices must follow the opaque boundary"
        );
        assert!(
            mesh.opaque_index_count < mesh.indices.len() as u32,
            "media triangles must come AFTER opaque_index_count"
        );
    }

    /// Box 1.5: media vertices carry bit 31 set on `texture_descriptor`; opaque
    /// (floor/ceiling/wall) vertices do NOT.
    #[test]
    fn media_vertices_flagged_with_bit31() {
        let p0 = make_square_polygon(-1);
        let p1 = make_square_polygon(0);
        let mut map = make_map_data(square_endpoints(), vec![p0, p1], vec![], vec![]);
        map.media = vec![make_media(0x0007, 256)];

        let mesh = build_level_mesh(&map);

        let media_verts: Vec<&Vertex> = mesh
            .vertices
            .iter()
            .filter(|v| v.texture_descriptor & MEDIA_VERTEX_FLAG != 0)
            .collect();
        let opaque_verts: Vec<&Vertex> = mesh
            .vertices
            .iter()
            .filter(|v| v.texture_descriptor & MEDIA_VERTEX_FLAG == 0)
            .collect();

        // One media square => 4 flagged vertices; both polygons' floors+ceilings
        // are opaque (unflagged).
        assert_eq!(media_verts.len(), 4, "exactly the media square is flagged");
        assert!(!opaque_verts.is_empty(), "opaque vertices must remain");

        for v in &media_verts {
            assert_ne!(
                v.texture_descriptor & MEDIA_VERTEX_FLAG,
                0,
                "media vertex must set bit 31"
            );
            // Low 31 bits preserve the real texture id.
            assert_eq!(v.texture_descriptor & 0x7FFF_FFFF, 0x0007);
        }
        for v in &opaque_verts {
            assert_eq!(
                v.texture_descriptor & MEDIA_VERTEX_FLAG,
                0,
                "opaque vertex must NOT set bit 31"
            );
        }
    }
}
