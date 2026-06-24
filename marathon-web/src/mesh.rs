use bytemuck::{Pod, Zeroable};
use marathon_formats::MapData;

/// Per-polygon lighting and transfer data, precomputed for baking into vertices.
pub struct PolygonInfo {
    pub floor_light: f32,
    pub floor_transfer_mode: u32,
    pub ceiling_light: f32,
    pub ceiling_transfer_mode: u32,
}

/// Wall vertex height-source surface selector (high bit of `Vertex::height_source`).
///
/// A wall vertex's top/bottom Y can be driven by either the floor or the ceiling
/// of its *source* polygon (which may be the owning polygon or an adjacent
/// neighbor — see design.md "Wall height-source representation (box 5.1)"). The
/// 1-bit selector lives in bit 31; the source polygon index is the low 31 bits.
/// Mirrors the native twin in `marathon-game/src/mesh.rs` (box 6.1).
pub const WALL_HEIGHT_FROM_FLOOR: u32 = 0;
pub const WALL_HEIGHT_FROM_CEILING: u32 = 1;

/// High bit of `Vertex::texture_descriptor` flagging a media-surface vertex.
/// Media vertices set this bit so the renderer/shader can distinguish them from
/// opaque geometry (apply media height override + alpha-blended visuals). The
/// real texture id lives in the low 31 bits; the shader masks this off before
/// sampling. Collection extraction (bits[12:8]) is unaffected by bit 31.
pub const MEDIA_VERTEX_FLAG: u32 = 0x8000_0000;

/// Bit position of the floor/ceiling selector inside `Vertex::height_source`.
const HEIGHT_SOURCE_SELECTOR_SHIFT: u32 = 31;
/// Mask covering the source-polygon-index bits of `Vertex::height_source`.
pub const HEIGHT_SOURCE_INDEX_MASK: u32 = 0x7FFF_FFFF;

/// Pack a height-source descriptor: which polygon's floor/ceiling drives this
/// vertex's Y. Mirrors the floor/ceiling/media surface-discriminator pattern but
/// uses a dedicated attribute because a wall vertex's `polygon_index` (kept for
/// light/transfer-mode sampling = the owning polygon) is *not* necessarily the
/// source polygon. (Box 5.3 resolves wall Y from this in the shader.)
pub fn pack_height_source(selector: u32, source_polygon_index: u32) -> u32 {
    (selector << HEIGHT_SOURCE_SELECTOR_SHIFT) | (source_polygon_index & HEIGHT_SOURCE_INDEX_MASK)
}

/// Extract the floor/ceiling selector from a packed `height_source`.
#[allow(dead_code)]
pub fn height_source_selector(height_source: u32) -> u32 {
    height_source >> HEIGHT_SOURCE_SELECTOR_SHIFT
}

/// Extract the source polygon index from a packed `height_source`.
#[allow(dead_code)]
pub fn height_source_index(height_source: u32) -> u32 {
    height_source & HEIGHT_SOURCE_INDEX_MASK
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
    /// Index of the source polygon. The shader uses this to sample the
    /// per-polygon data texture for the dynamic height offset and light
    /// (see `poly_data`). For wall quads this is the owning polygon's index.
    pub polygon_index: u32,
    /// Packed wall height-source descriptor: high bit = floor/ceiling selector,
    /// low 31 bits = source polygon index (see `pack_height_source`). For wall
    /// quads this names the polygon (and surface) whose animated height drives
    /// this vertex's Y; box 5.3 resolves Y from it in the shader instead of the
    /// baked `position.y`. Floor/ceiling/media vertices carry their own
    /// surface+polygon as a forward-consistent default (unused for them).
    pub height_source: u32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
        3 => Float32,
        4 => Uint32,
        5 => Uint32,
        6 => Uint32,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Surface discriminators stored in `Vertex::position[1]` (Y) for height-zero
/// reference geometry. The vertex shader (box 3.1) reads this to choose which
/// per-polygon height (floor / ceiling / media) from the data texture to apply.
/// Walls are not un-baked by this scheme (they span two polygons' heights).
pub const SURFACE_FLOOR: f32 = 0.0;
pub const SURFACE_CEILING: f32 = 1.0;
pub const SURFACE_MEDIA: f32 = 2.0;

/// Sentinel light value for un-baked floor/ceiling/media vertices. The fragment
/// shader (box 3.2) replaces this with the per-polygon light from the data
/// texture; 1.0 keeps appearance neutral until then.
pub const UNBAKED_LIGHT: f32 = 1.0;

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
    /// Opaque batches come first (covering `0..opaque_index_count`), followed by
    /// media batches (covering `opaque_index_count..indices.len()`).
    pub batches: Vec<DrawBatch>,
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
pub fn build_level_mesh(map: &MapData, poly_info: &[PolygonInfo]) -> LevelMesh {
    let mut vertices = Vec::new();

    // Opaque and media geometry are accumulated into separate index buffers so
    // they can be batched independently and concatenated opaque-then-media; the
    // boundary between them becomes `opaque_index_count`.
    let mut opaque_indices = Vec::new();
    let mut media_indices = Vec::new();

    // --- Opaque pass: floors + ceilings for every polygon, then all walls. ---
    for (poly_idx, polygon) in map.polygons.iter().enumerate() {
        let vert_count = polygon.vertex_count as usize;
        if vert_count < 3 {
            continue;
        }

        let info = &poly_info[poly_idx];

        build_floor(
            &mut vertices,
            &mut opaque_indices,
            map,
            polygon,
            info,
            vert_count,
            poly_idx,
        );
        build_ceiling(
            &mut vertices,
            &mut opaque_indices,
            map,
            polygon,
            info,
            vert_count,
            poly_idx,
        );
    }

    for line in &map.lines {
        build_walls_for_line(&mut vertices, &mut opaque_indices, map, line, poly_info);
    }

    // --- Media pass: alpha-blended media surfaces for every polygon. ---
    for (poly_idx, polygon) in map.polygons.iter().enumerate() {
        let vert_count = polygon.vertex_count as usize;
        if vert_count < 3 {
            continue;
        }

        if polygon.media_index >= 0 {
            if let Some(media) = map.media.get(polygon.media_index as usize) {
                let info = &poly_info[poly_idx];
                build_media_surface(
                    &mut vertices,
                    &mut media_indices,
                    map,
                    polygon,
                    info,
                    vert_count,
                    media,
                    poly_idx,
                );
            }
        }
    }

    // Group triangles by collection for batched rendering, opaque first then
    // media. Each triangle's collection is determined by the first vertex's
    // texture_descriptor. Batching opaque and media separately preserves the
    // opaque-then-media ordering despite the per-batch collection sort.
    let mut opaque_batches = build_draw_batches(&vertices, &mut opaque_indices);
    let opaque_index_count = opaque_indices.len() as u32;

    let media_batches = build_draw_batches(&vertices, &mut media_indices);

    // Concatenate: opaque indices first, then media indices (offsetting media
    // batch starts past the opaque range).
    let mut indices = opaque_indices;
    for mut batch in media_batches {
        batch.index_start += opaque_index_count;
        opaque_batches.push(batch);
    }
    indices.extend_from_slice(&media_indices);
    let batches = opaque_batches;

    LevelMesh {
        vertices,
        indices,
        batches,
        opaque_index_count,
    }
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
    poly_idx: usize,
) {
    let base = vertices.len() as u32;
    // Height is NOT baked here — the shader adds the per-polygon floor height
    // from the data texture (box 3.1). `position.y` instead carries a surface
    // discriminator the vertex shader reads to pick which data-texture height
    // to apply: 0.0 = floor, 1.0 = ceiling, 2.0 = media.
    let floor_y = SURFACE_FLOOR;
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
            // Light is NOT baked — the fragment shader applies the per-polygon
            // floor light from the data texture (box 3.2). Sentinel 1.0 keeps
            // the pre-shader-change appearance neutral.
            light: UNBAKED_LIGHT,
            transfer_mode: info.floor_transfer_mode,
            polygon_index: poly_idx as u32,
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
    info: &PolygonInfo,
    vert_count: usize,
    poly_idx: usize,
) {
    let base = vertices.len() as u32;
    // Height un-baked; Y carries the ceiling surface discriminator (see build_floor).
    let ceil_y = SURFACE_CEILING;
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
            light: UNBAKED_LIGHT,
            transfer_mode: info.ceiling_transfer_mode,
            polygon_index: poly_idx as u32,
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

#[allow(clippy::too_many_arguments)]
fn build_media_surface(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    map: &MapData,
    polygon: &marathon_formats::Polygon,
    _info: &PolygonInfo,
    vert_count: usize,
    media: &marathon_formats::MediaData,
    poly_idx: usize,
) {
    let base = vertices.len() as u32;
    // Height un-baked; Y carries the media surface discriminator (see build_floor).
    let media_y = SURFACE_MEDIA;
    // Flag this as a media-surface vertex by setting bit 31 of the texture
    // descriptor (low bits stay the real texture id). The shader masks bit 31
    // off before sampling and uses it to apply media height/visual overrides.
    // Collection extraction (bits[12:8]) is unaffected by bit 31.
    let tex_desc = (media.texture.0 as u32) | MEDIA_VERTEX_FLAG;

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
            light: UNBAKED_LIGHT,
            transfer_mode: media.transfer_mode as u32,
            polygon_index: poly_idx as u32,
            // Media keeps baked Y; tag with the owning polygon's floor selector
            // for forward consistency (unused by the shader for media).
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
            build_wall_side(
                vertices,
                indices,
                map,
                line,
                side,
                poly_idx,
                adjacent_poly_idx,
                false,
                poly_info,
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
                poly_info,
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
                    info.floor_light,
                    side.primary_transfer_mode as u32,
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
                        info.floor_light,
                        side.primary_transfer_mode as u32,
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
                        info.floor_light,
                        side.primary_transfer_mode as u32,
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
                        info.floor_light,
                        side.secondary_transfer_mode as u32,
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
                        info.floor_light,
                        side.transparent_transfer_mode as u32,
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
                        info.floor_light,
                        side.primary_transfer_mode as u32,
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
    light: f32,
    transfer_mode: u32,
    poly_idx: usize,
    bottom_source: u32,
    top_source: u32,
) {
    let base = vertices.len() as u32;
    let height = top - bottom;
    let u_off = side_tex.x0 as f32 / 1024.0;
    let v_off = side_tex.y0 as f32 / 1024.0;
    let polygon_index = poly_idx as u32;

    vertices.push(Vertex {
        position: [x0, bottom, z0],
        uv: [u_off, v_off + height],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
        polygon_index,
        height_source: bottom_source,
    });
    vertices.push(Vertex {
        position: [x0, top, z0],
        uv: [u_off, v_off],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
        polygon_index,
        height_source: top_source,
    });
    vertices.push(Vertex {
        position: [x1, top, z1],
        uv: [u_off + wall_len, v_off],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
        polygon_index,
        height_source: top_source,
    });
    vertices.push(Vertex {
        position: [x1, bottom, z1],
        uv: [u_off + wall_len, v_off + height],
        texture_descriptor: tex_desc,
        light,
        transfer_mode,
        polygon_index,
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
        // 3 floats (pos) + 2 floats (uv) + 1 u32 (tex_desc) + 1 float (light)
        // + 1 u32 (transfer) + 1 u32 (polygon_index) + 1 u32 (height_source)
        // = 10 * 4 = 40 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 40);
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
            polygon_index: 7,
            height_source: pack_height_source(WALL_HEIGHT_FROM_FLOOR, 7),
        };
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 40);
    }

    #[test]
    fn vertex_layout_includes_polygon_index_attribute() {
        let layout = Vertex::layout();
        assert_eq!(layout.array_stride, 40);
        // 7 attributes: pos, uv, tex_desc, light, transfer, polygon_index,
        // height_source.
        assert_eq!(layout.attributes.len(), 7);
        let pi = layout.attributes[5];
        assert_eq!(pi.shader_location, 5);
        assert_eq!(pi.format, wgpu::VertexFormat::Uint32);
        let hs = layout.attributes[6];
        assert_eq!(hs.shader_location, 6);
        assert_eq!(hs.format, wgpu::VertexFormat::Uint32);
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

    /// Box 5.2 (red-green): a high wall (side_type 1) sources its BOTTOM edge
    /// from the *adjacent* (neighbor) polygon's ceiling, not the owning polygon.
    /// This proves wall vertices carry the packed height-source discriminator +
    /// source polygon index (the neighbor) instead of relying solely on baked
    /// absolute Y — and that the source polygon is not always `polygon_index`.
    /// Mirrors the native twin `high_wall_bottom_sources_neighbor_polygon`.
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
        let line = Line {
            endpoint_indexes: [0, 1],
            flags: 0,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 1024,
            clockwise_polygon_side_index: 0,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: 1,
        };
        let map = make_map_data(endpoints, vec![p0, p1], vec![line], vec![side]);
        let poly_info = vec![make_info(), make_info()];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_walls_for_line(&mut vertices, &mut indices, &map, &map.lines[0], &poly_info);

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
                assert_eq!(
                    height_source_selector(v.height_source),
                    WALL_HEIGHT_FROM_CEILING
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

    /// Box 5.2: a full wall (side_type 0) carries the discriminator + owning
    /// polygon index — bottom from floor of poly 0, top from ceiling of poly 0.
    #[test]
    fn full_wall_vertices_carry_height_source_discriminator() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
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

        assert_eq!(vertices.len(), 4, "full wall emits one quad (4 vertices)");

        let expect_bottom = pack_height_source(WALL_HEIGHT_FROM_FLOOR, 0);
        let expect_top = pack_height_source(WALL_HEIGHT_FROM_CEILING, 0);
        for v in &vertices {
            if v.position[1] == 0.0 {
                assert_eq!(v.height_source, expect_bottom);
                assert_eq!(
                    height_source_selector(v.height_source),
                    WALL_HEIGHT_FROM_FLOOR
                );
            } else {
                assert_eq!(v.height_source, expect_top);
                assert_eq!(
                    height_source_selector(v.height_source),
                    WALL_HEIGHT_FROM_CEILING
                );
            }
        }
    }

    #[test]
    fn floor_height_no_longer_baked_into_vertex_y() {
        // Two square polygons with DIFFERENT floor heights. After un-baking,
        // their floor vertices must have identical Y (the height-zero floor
        // discriminant) and differ only by polygon_index.
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ];
        let mut p0 = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        p0.floor_height = 0;
        let mut p1 = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        p1.floor_height = 2048; // very different absolute height
        let map = make_map_data(endpoints, vec![p0, p1], vec![], vec![]);
        let poly_info = vec![make_info(), make_info()];

        let mut v0 = Vec::new();
        let mut i0 = Vec::new();
        build_floor(
            &mut v0,
            &mut i0,
            &map,
            &map.polygons[0],
            &poly_info[0],
            4,
            0,
        );
        let mut v1 = Vec::new();
        let mut i1 = Vec::new();
        build_floor(
            &mut v1,
            &mut i1,
            &map,
            &map.polygons[1],
            &poly_info[1],
            4,
            1,
        );

        assert_eq!(v0.len(), v1.len());
        for (a, b) in v0.iter().zip(v1.iter()) {
            assert_eq!(
                a.position[1], SURFACE_FLOOR,
                "floor Y must be the height-zero discriminator"
            );
            assert_eq!(b.position[1], SURFACE_FLOOR);
            assert_eq!(
                a.position[1], b.position[1],
                "Y identical despite different floor_height"
            );
            // Differ only by polygon_index.
            assert_eq!(a.polygon_index, 0);
            assert_eq!(b.polygon_index, 1);
            assert_ne!(a.polygon_index, b.polygon_index);
            // X/Z geometry unchanged.
            assert_eq!(a.position[0], b.position[0]);
            assert_eq!(a.position[2], b.position[2]);
        }
    }

    #[test]
    fn floor_ceiling_media_light_is_unbaked_sentinel() {
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ];
        let polygon = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        let map = make_map_data(endpoints, vec![polygon], vec![], vec![]);
        // info carries non-1.0 light to prove it is NOT baked in.
        let info = PolygonInfo {
            floor_light: 0.3,
            floor_transfer_mode: 0,
            ceiling_light: 0.7,
            ceiling_transfer_mode: 0,
        };

        let mut vf = Vec::new();
        let mut idx = Vec::new();
        build_floor(&mut vf, &mut idx, &map, &map.polygons[0], &info, 4, 0);
        for v in &vf {
            assert_eq!(
                v.light, UNBAKED_LIGHT,
                "floor light must be the unbaked sentinel, not info.floor_light"
            );
        }

        let mut vc = Vec::new();
        let mut idc = Vec::new();
        build_ceiling(&mut vc, &mut idc, &map, &map.polygons[0], &info, 4, 0);
        for v in &vc {
            assert_eq!(v.position[1], SURFACE_CEILING);
            assert_eq!(v.light, UNBAKED_LIGHT);
        }
    }

    #[test]
    fn build_level_mesh_assigns_source_polygon_index_to_every_vertex() {
        // Two stacked square polygons; every emitted floor/ceiling vertex must
        // carry the index of its source polygon.
        let endpoints = vec![
            make_endpoint(0, 0),
            make_endpoint(1024, 0),
            make_endpoint(1024, 1024),
            make_endpoint(0, 1024),
        ];
        let mut p0 = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        p0.floor_height = 0;
        p0.ceiling_height = 1024;
        let mut p1 = make_polygon(4, [0, 1, 2, 3, -1, -1, -1, -1]);
        p1.floor_height = 512;
        p1.ceiling_height = 2048;
        let map = make_map_data(endpoints, vec![p0, p1], vec![], vec![]);
        let poly_info = vec![make_info(), make_info()];

        let mesh = build_level_mesh(&map, &poly_info);

        assert!(!mesh.vertices.is_empty());
        // Both polygon indices must appear, and no vertex may reference a
        // polygon outside [0, 2).
        let mut seen0 = false;
        let mut seen1 = false;
        for v in &mesh.vertices {
            assert!(
                (v.polygon_index as usize) < map.polygons.len(),
                "polygon_index {} out of range",
                v.polygon_index
            );
            match v.polygon_index {
                0 => seen0 = true,
                1 => seen1 = true,
                other => panic!("unexpected polygon_index {other}"),
            }
        }
        assert!(
            seen0 && seen1,
            "vertices from both polygons must be present"
        );
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
        build_floor(
            &mut vertices,
            &mut indices,
            &map,
            &map.polygons[0],
            &info,
            5,
            0,
        );

        assert_eq!(vertices.len(), 4, "should emit 4 vertices (skipping -1)");
        assert_eq!(
            indices.len(),
            6,
            "should emit 2 triangles (6 indices) from 4 verts"
        );
    }

    #[test]
    fn floor_triangulation_too_few_valid_verts_produces_nothing() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
        let polygon = make_polygon(4, [0, -1, 1, -1, -1, -1, -1, -1]);
        let map = make_map_data(endpoints, vec![polygon], vec![], vec![]);
        let info = make_info();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_floor(
            &mut vertices,
            &mut indices,
            &map,
            &map.polygons[0],
            &info,
            4,
            0,
        );

        assert_eq!(vertices.len(), 2, "should emit 2 vertices");
        assert_eq!(
            indices.len(),
            0,
            "should emit 0 triangles (not enough verts)"
        );
    }

    #[test]
    fn wall_none_texture_produces_no_geometry() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
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

        assert_eq!(
            vertices.len(),
            0,
            "none-texture wall should produce 0 vertices"
        );
        assert_eq!(
            indices.len(),
            0,
            "none-texture wall should produce 0 indices"
        );
    }

    #[test]
    fn wall_valid_texture_produces_quad() {
        let endpoints = vec![make_endpoint(0, 0), make_endpoint(1024, 0)];
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

        assert_eq!(
            vertices.len(),
            4,
            "valid-texture wall should produce 4 vertices"
        );
        assert_eq!(
            indices.len(),
            6,
            "valid-texture wall should produce 6 indices"
        );
    }
}
