use bytemuck::{Pod, Zeroable};
use marathon_formats::MapData;

/// Per-polygon lighting and transfer data, precomputed for baking into vertices.
pub struct PolygonInfo {
    pub floor_light: f32,
    pub floor_transfer_mode: u32,
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

/// Result of converting a level's geometry to GPU-ready mesh data.
pub struct LevelMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
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

    LevelMesh { vertices, indices }
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
            texture_descriptor: tex_desc,
            light: info.floor_light,
            transfer_mode: info.floor_transfer_mode,
        });
    }

    for i in 1..(vert_count as u32 - 1) {
        indices.push(base);
        indices.push(base + i);
        indices.push(base + i + 1);
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
            texture_descriptor: tex_desc,
            light: info.floor_light,
            transfer_mode: info.floor_transfer_mode,
        });
    }

    for i in 1..(vert_count as u32 - 1) {
        indices.push(base);
        indices.push(base + i + 1);
        indices.push(base + i);
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

        vertices.push(Vertex {
            position: [wx, media_y, wz],
            uv: [u, v],
            texture_descriptor: tex_desc,
            light: info.floor_light,
            transfer_mode: media.transfer_mode as u32,
        });
    }

    for i in 1..(vert_count as u32 - 1) {
        indices.push(base);
        indices.push(base + i);
        indices.push(base + i + 1);
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
    let z0 = world_to_f32(ep0.vertex.y);
    let x1 = world_to_f32(ep1.vertex.x);
    let z1 = world_to_f32(ep1.vertex.y);

    let wall_len = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt();

    match side.side_type {
        0 => {
            let bottom = world_to_f32(polygon.floor_height);
            let top = world_to_f32(polygon.ceiling_height);
            let tex = &side.primary_texture;
            emit_wall_quad(
                vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                tex.texture.0 as u32, info,
            );
        }
        1 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];
                let bottom = world_to_f32(adj.ceiling_height);
                let top = world_to_f32(polygon.ceiling_height);
                if top > bottom {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                        tex.texture.0 as u32, info,
                    );
                }
            }
        }
        2 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];
                let bottom = world_to_f32(polygon.floor_height);
                let top = world_to_f32(adj.floor_height);
                if top > bottom {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, bottom, top, wall_len, tex,
                        tex.texture.0 as u32, info,
                    );
                }
            }
        }
        3 => {
            if let Some(adj_idx) = adjacent_poly_idx {
                let adj = &map.polygons[adj_idx];

                let low_bottom = world_to_f32(polygon.floor_height);
                let low_top = world_to_f32(adj.floor_height);
                if low_top > low_bottom {
                    let tex = &side.secondary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, low_bottom, low_top, wall_len, tex,
                        tex.texture.0 as u32, info,
                    );
                }

                let trans_bottom = world_to_f32(adj.floor_height);
                let trans_top = world_to_f32(adj.ceiling_height);
                if trans_top > trans_bottom && !side.transparent_texture.texture.is_none() {
                    let tex = &side.transparent_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, trans_bottom, trans_top, wall_len,
                        tex, tex.texture.0 as u32, info,
                    );
                }

                let high_bottom = world_to_f32(adj.ceiling_height);
                let high_top = world_to_f32(polygon.ceiling_height);
                if high_top > high_bottom {
                    let tex = &side.primary_texture;
                    emit_wall_quad(
                        vertices, indices, x0, z0, x1, z1, high_bottom, high_top, wall_len,
                        tex, tex.texture.0 as u32, info,
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
    info: &PolygonInfo,
) {
    let base = vertices.len() as u32;
    let height = top - bottom;
    let u_off = side_tex.x0 as f32 / 1024.0;
    let v_off = side_tex.y0 as f32 / 1024.0;

    vertices.push(Vertex {
        position: [x0, bottom, z0],
        uv: [u_off, v_off + height],
        texture_descriptor: tex_desc,
        light: info.floor_light,
        transfer_mode: info.floor_transfer_mode,
    });
    vertices.push(Vertex {
        position: [x0, top, z0],
        uv: [u_off, v_off],
        texture_descriptor: tex_desc,
        light: info.floor_light,
        transfer_mode: info.floor_transfer_mode,
    });
    vertices.push(Vertex {
        position: [x1, top, z1],
        uv: [u_off + wall_len, v_off],
        texture_descriptor: tex_desc,
        light: info.floor_light,
        transfer_mode: info.floor_transfer_mode,
    });
    vertices.push(Vertex {
        position: [x1, bottom, z1],
        uv: [u_off + wall_len, v_off + height],
        texture_descriptor: tex_desc,
        light: info.floor_light,
        transfer_mode: info.floor_transfer_mode,
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
}
