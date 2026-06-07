//! Per-polygon dynamic data, packed for upload into a GPU data texture.
//!
//! # Texel packing layout (resolves design Open Question 1)
//!
//! WebGL2 cannot bind a shader storage buffer (SSBO is GLES 3.1 / WebGPU only),
//! and a uniform-block array caps at ~640 polygons (64 KB / ~10 f32). So the web
//! renderer encodes per-polygon dynamic data into rows of an `Rgba32Float`
//! texture that the shader samples by `polygon_index`.
//!
//! A single RGBA32F texel holds only 4 floats, which is insufficient for the 5
//! per-polygon values (floor height, ceiling height, media height, floor light,
//! ceiling light). **Decision: 2 RGBA32F texels per polygon** (8 float slots; 5
//! used, 3 reserved for future expansion such as fog density / transfer
//! animation phase). The data texture is laid out as a 2-wide,
//! `polygon_count`-tall texture (`width = 2`, `height = N`), row-major:
//!
//! ```text
//!   row p, texel 0 (x=0): [ floor_h, ceiling_h, media_h, floor_light ]
//!   row p, texel 1 (x=1): [ ceiling_light, reserved, reserved, reserved ]
//! ```
//!
//! Rationale for 2-wide-by-N over a flat `2*N`-wide single row:
//! - Keeps texture dimensions well within `downlevel_webgl2_defaults`
//!   (`max_texture_dimension_2d = 2048`): a 2-wide texture supports up to 2048
//!   polygons in height; if a level exceeds that, height can grow by tiling
//!   without ever approaching the width limit. A flat single-row layout would
//!   hit the 2048 width cap at only 1024 polygons.
//! - The vertex/fragment shader recovers a polygon's two texels with a trivial
//!   `(x = 0 | 1, y = polygon_index)` lookup — no division/modulo.
//!
//! `FLOATS_PER_POLYGON` is the packed stride (8 = 2 texels * 4 channels).

/// Number of f32 slots written per polygon (2 RGBA32F texels).
pub const FLOATS_PER_POLYGON: usize = 8;

/// Texture width in texels for the per-polygon data texture (2 RGBA32F texels
/// per polygon row).
pub const DATA_TEXTURE_WIDTH: u32 = 2;

/// `downlevel_webgl2_defaults` guarantees at least this for
/// `max_texture_dimension_2d`. The data texture height (= polygon count) must
/// not exceed it.
pub const WEBGL2_MAX_TEXTURE_DIMENSION_2D: u32 = 2048;

/// Compute the `Rgba32Float` data-texture size for a level with `polygon_count`
/// polygons: 2 texels wide (see module docs), one row per polygon.
///
/// Returns `None` if `polygon_count` exceeds the WebGL2-guaranteed
/// `max_texture_dimension_2d`, in which case the caller must tile (out of scope
/// here) rather than silently truncate. `polygon_count == 0` yields a 2x1
/// texture (wgpu rejects zero-sized textures) that is simply never sampled.
pub fn data_texture_extent(polygon_count: usize) -> Option<wgpu::Extent3d> {
    let height = polygon_count.max(1) as u32;
    if height > WEBGL2_MAX_TEXTURE_DIMENSION_2D {
        return None;
    }
    Some(wgpu::Extent3d {
        width: DATA_TEXTURE_WIDTH,
        height,
        depth_or_array_layers: 1,
    })
}

/// The texture format used for the per-polygon data texture.
pub const DATA_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Build the [`wgpu::TextureDescriptor`] for the per-polygon data texture.
pub fn data_texture_descriptor(extent: wgpu::Extent3d) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("poly_data_texture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DATA_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    }
}

/// Bind-group-layout entries for the per-polygon data texture: an unfilterable
/// float texture sampled in both vertex (height offset) and fragment (light)
/// stages, plus a non-filtering sampler. WebGL2 cannot linearly filter
/// `Rgba32Float`, and per-polygon data must not be interpolated anyway, so the
/// sample type is `Float { filterable: false }` with a `NonFiltering` sampler.
pub fn data_texture_bgl_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        },
    ]
}

/// Dynamic per-polygon state that the shader needs each frame.
///
/// Heights are in render units (Marathon world units / 1024). Light values are
/// 0.0..=1.0 intensity multipliers.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PolyDynData {
    pub floor_h: f32,
    pub ceiling_h: f32,
    pub media_h: f32,
    pub floor_light: f32,
    pub ceiling_light: f32,
}

/// Pack per-polygon dynamic data into a flat `f32` buffer laid out as 2 RGBA32F
/// texels per polygon (see module docs for the layout).
///
/// The returned buffer has `data.len() * FLOATS_PER_POLYGON` elements and can be
/// uploaded directly via `queue.write_texture` into a `width = 2`,
/// `height = data.len()` `Rgba32Float` texture.
pub fn pack_poly_data(data: &[PolyDynData]) -> Vec<f32> {
    let mut out = vec![0.0f32; data.len() * FLOATS_PER_POLYGON];
    for (i, d) in data.iter().enumerate() {
        let base = i * FLOATS_PER_POLYGON;
        // texel 0
        out[base] = d.floor_h;
        out[base + 1] = d.ceiling_h;
        out[base + 2] = d.media_h;
        out[base + 3] = d.floor_light;
        // texel 1
        out[base + 4] = d.ceiling_light;
        // out[base + 5..base + 8] reserved (left zero)
    }
    out
}

/// Inverse of [`pack_poly_data`]: recover the `PolyDynData` array from a packed
/// buffer. Used by round-trip tests and by any CPU-side readback path.
pub fn unpack_poly_data(packed: &[f32]) -> Vec<PolyDynData> {
    let count = packed.len() / FLOATS_PER_POLYGON;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * FLOATS_PER_POLYGON;
        out.push(PolyDynData {
            floor_h: packed[base],
            ceiling_h: packed[base + 1],
            media_h: packed[base + 2],
            floor_light: packed[base + 3],
            ceiling_light: packed[base + 4],
        });
    }
    out
}

/// Bytes per texture row for the packed buffer: `DATA_TEXTURE_WIDTH` RGBA32F
/// texels = 2 * 4 channels * 4 bytes = 32 bytes.
pub const DATA_TEXTURE_BYTES_PER_ROW: u32 = DATA_TEXTURE_WIDTH * 4 * 4;

/// The `TexelCopyBufferLayout` describing how [`pack_poly_data`]'s output maps
/// onto the data texture rows.
pub fn data_texture_copy_layout(poly_count: usize) -> wgpu::TexelCopyBufferLayout {
    wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(DATA_TEXTURE_BYTES_PER_ROW),
        rows_per_image: Some(poly_count.max(1) as u32),
    }
}

/// Upload per-polygon dynamic data into the data texture via
/// `queue.write_texture`. The packed buffer is laid out 2 RGBA32F texels per
/// polygon, one texture row per polygon (see module docs).
///
/// `texture` must have been created with [`data_texture_descriptor`] for the
/// same `data.len()`.
pub fn write_poly_data_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[PolyDynData]) {
    if data.is_empty() {
        return;
    }
    let packed = pack_poly_data(data);
    let extent = data_texture_extent(data.len())
        .expect("polygon count exceeds WebGL2 max texture dimension");
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&packed),
        data_texture_copy_layout(data.len()),
        extent,
    );
}

/// Convert a Marathon world distance (i16, 1024 = 1 world unit) to render
/// units. Mirrors `mesh::world_to_f32` so the data texture's heights match the
/// scale the shader uses for X/Z.
fn world_to_render(v: i16) -> f32 {
    v as f32 / 1024.0
}

/// Build the initial per-polygon dynamic data at level load: floor/ceiling/
/// media heights (render units) plus floor/ceiling light intensities from the
/// same `evaluate_light_intensity` path the renderer used before un-baking.
///
/// This reproduces the values that `build_floor`/`build_ceiling`/
/// `build_media_surface` previously baked into vertices, so a fully static
/// scene renders identically once the shader (boxes 3.1/3.2) applies them.
pub fn build_poly_dyn_data(map: &marathon_formats::MapData) -> Vec<PolyDynData> {
    map.polygons
        .iter()
        .map(|p| {
            let media_h = if p.media_index >= 0 {
                map.media
                    .get(p.media_index as usize)
                    .map(|m| world_to_render(m.height))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            PolyDynData {
                floor_h: world_to_render(p.floor_height),
                ceiling_h: world_to_render(p.ceiling_height),
                media_h,
                floor_light: crate::level::evaluate_light_intensity(
                    &map.lights,
                    p.floor_lightsource_index,
                ),
                ceiling_light: crate::level::evaluate_light_intensity(
                    &map.lights,
                    p.ceiling_lightsource_index,
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_poly_dyn_data_reproduces_prechange_baked_values() {
        use marathon_formats::map::LightData;
        use marathon_formats::*;

        // Pre-change, build_floor baked position.y = floor_height/1024,
        // build_ceiling = ceiling_height/1024, build_media = media.height/1024,
        // and light = evaluate_light_intensity(lights, *_lightsource_index).
        // With LightData::None evaluate_light_intensity returns 1.0.
        let mut p0 = mk_poly();
        p0.floor_height = 0;
        p0.ceiling_height = 1024;
        p0.media_index = -1;
        let mut p1 = mk_poly();
        p1.floor_height = 2048; // 2.0 render units
        p1.ceiling_height = 3072; // 3.0
        p1.media_index = 0;

        let media = MediaData {
            media_type: 0,
            flags: 0,
            light_index: 0,
            current_direction: 0,
            current_magnitude: 0,
            low: 0,
            high: 0,
            origin: WorldPoint2d { x: 0, y: 0 },
            height: 512, // 0.5 render units
            minimum_light_intensity: 0.0,
            texture: ShapeDescriptor(0x0100),
            transfer_mode: 0,
        };

        let map = MapData {
            endpoints: vec![],
            lines: vec![],
            sides: vec![],
            polygons: vec![p0, p1],
            objects: vec![],
            lights: LightData::None,
            platforms: vec![],
            media: vec![media],
            annotations: vec![],
            terminals: vec![],
            ambient_sounds: vec![],
            random_sounds: vec![],
            map_info: None,
            item_placement: vec![],
            guard_paths: None,
        };

        let data = build_poly_dyn_data(&map);
        assert_eq!(data.len(), 2);

        // Polygon 0 — matches pre-change baked floor/ceiling Y, no media.
        assert_eq!(data[0].floor_h, 0.0);
        assert_eq!(data[0].ceiling_h, 1.0);
        assert_eq!(data[0].media_h, 0.0);
        assert_eq!(data[0].floor_light, 1.0); // LightData::None -> 1.0
        assert_eq!(data[0].ceiling_light, 1.0);

        // Polygon 1 — different heights + media.
        assert_eq!(data[1].floor_h, 2.0);
        assert_eq!(data[1].ceiling_h, 3.0);
        assert_eq!(data[1].media_h, 0.5);
        assert_eq!(data[1].floor_light, 1.0);
        assert_eq!(data[1].ceiling_light, 1.0);
    }

    fn mk_poly() -> marathon_formats::Polygon {
        use marathon_formats::*;
        Polygon {
            polygon_type: 0,
            flags: 0,
            permutation: 0,
            vertex_count: 4,
            endpoint_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
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

    #[test]
    fn pack_two_polygons_has_expected_offsets() {
        let input = vec![
            PolyDynData {
                floor_h: 1.0,
                ceiling_h: 2.0,
                media_h: 3.0,
                floor_light: 0.5,
                ceiling_light: 0.25,
            },
            PolyDynData {
                floor_h: -4.0,
                ceiling_h: 8.0,
                media_h: 0.0,
                floor_light: 1.0,
                ceiling_light: 0.75,
            },
        ];

        let packed = pack_poly_data(&input);

        // 2 polygons * 8 floats each.
        assert_eq!(packed.len(), 16);

        // Polygon 0, texel 0.
        assert_eq!(packed[0], 1.0); // floor_h
        assert_eq!(packed[1], 2.0); // ceiling_h
        assert_eq!(packed[2], 3.0); // media_h
        assert_eq!(packed[3], 0.5); // floor_light
                                    // Polygon 0, texel 1.
        assert_eq!(packed[4], 0.25); // ceiling_light
        assert_eq!(packed[5], 0.0); // reserved
        assert_eq!(packed[6], 0.0); // reserved
        assert_eq!(packed[7], 0.0); // reserved

        // Polygon 1, texel 0.
        assert_eq!(packed[8], -4.0); // floor_h
        assert_eq!(packed[9], 8.0); // ceiling_h
        assert_eq!(packed[10], 0.0); // media_h
        assert_eq!(packed[11], 1.0); // floor_light
                                     // Polygon 1, texel 1.
        assert_eq!(packed[12], 0.75); // ceiling_light
    }

    #[test]
    fn pack_empty_yields_empty() {
        assert!(pack_poly_data(&[]).is_empty());
    }

    #[test]
    fn data_texture_extent_matches_polygon_count_within_webgl2_limits() {
        // A realistic Marathon level (>1000 polygons) fits.
        let ext = data_texture_extent(1500).expect("1500 polys must fit");
        assert_eq!(ext.width, DATA_TEXTURE_WIDTH);
        assert_eq!(ext.height, 1500, "one texture row per polygon");
        assert_eq!(ext.depth_or_array_layers, 1);
        assert!(ext.width <= WEBGL2_MAX_TEXTURE_DIMENSION_2D);
        assert!(ext.height <= WEBGL2_MAX_TEXTURE_DIMENSION_2D);

        // At the limit.
        let at = data_texture_extent(WEBGL2_MAX_TEXTURE_DIMENSION_2D as usize).unwrap();
        assert_eq!(at.height, WEBGL2_MAX_TEXTURE_DIMENSION_2D);

        // Empty level still produces a valid (non-zero) 2x1 texture.
        let empty = data_texture_extent(0).unwrap();
        assert_eq!(empty.width, 2);
        assert_eq!(empty.height, 1);

        // Exceeding the WebGL2 guarantee is reported, not silently truncated.
        assert!(data_texture_extent(WEBGL2_MAX_TEXTURE_DIMENSION_2D as usize + 1).is_none());
    }

    #[test]
    fn data_texture_descriptor_is_rgba32float_bindable_copydst() {
        let ext = data_texture_extent(10).unwrap();
        let desc = data_texture_descriptor(ext);
        assert_eq!(desc.format, wgpu::TextureFormat::Rgba32Float);
        assert_eq!(desc.size.width, 2);
        assert_eq!(desc.size.height, 10);
        assert_eq!(desc.dimension, wgpu::TextureDimension::D2);
        assert!(desc
            .usage
            .contains(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST));
    }

    #[test]
    fn data_texture_bgl_entries_are_vertex_and_fragment_visible() {
        let entries = data_texture_bgl_entries();
        // Texture entry visible to both stages (vertex offsets Y, fragment uses light).
        assert!(entries[0]
            .visibility
            .contains(wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT));
        match entries[0].ty {
            wgpu::BindingType::Texture { sample_type, .. } => {
                assert_eq!(
                    sample_type,
                    wgpu::TextureSampleType::Float { filterable: false }
                );
            }
            _ => panic!("entry 0 must be a texture binding"),
        }
        match entries[1].ty {
            wgpu::BindingType::Sampler(s) => {
                assert_eq!(s, wgpu::SamplerBindingType::NonFiltering);
            }
            _ => panic!("entry 1 must be a sampler binding"),
        }
    }

    #[test]
    fn copy_layout_bytes_per_row_matches_packed_buffer() {
        // bytes_per_row must equal one polygon row's byte size, and the packed
        // buffer length must be rows_per_image * bytes_per_row.
        let input = vec![PolyDynData::default(); 7];
        let packed = pack_poly_data(&input);
        let layout = data_texture_copy_layout(input.len());

        assert_eq!(layout.bytes_per_row, Some(32)); // 2 texels * 4 ch * 4 bytes
        assert_eq!(layout.rows_per_image, Some(7));

        let packed_bytes = packed.len() * std::mem::size_of::<f32>();
        let expected =
            layout.bytes_per_row.unwrap() as usize * layout.rows_per_image.unwrap() as usize;
        assert_eq!(packed_bytes, expected);
    }

    #[test]
    fn round_trip_through_texel_byte_layout_yields_input() {
        // Simulate the GPU path: pack -> bytes (what write_texture uploads) ->
        // reinterpret as f32 -> unpack. Must reproduce the input exactly.
        let input = vec![
            PolyDynData {
                floor_h: 12.5,
                ceiling_h: 30.0,
                media_h: 7.25,
                floor_light: 0.6,
                ceiling_light: 0.9,
            },
            PolyDynData {
                floor_h: -1.0,
                ceiling_h: 0.0,
                media_h: 100.0,
                floor_light: 0.0,
                ceiling_light: 1.0,
            },
        ];
        let packed = pack_poly_data(&input);
        let bytes: &[u8] = bytemuck::cast_slice(&packed);
        let back: &[f32] = bytemuck::cast_slice(bytes);
        let restored = unpack_poly_data(back);
        assert_eq!(restored, input);
    }

    #[test]
    fn pack_unpack_round_trips() {
        let input = vec![
            PolyDynData {
                floor_h: 0.123,
                ceiling_h: 9.5,
                media_h: -2.0,
                floor_light: 0.8,
                ceiling_light: 0.1,
            },
            PolyDynData::default(),
            PolyDynData {
                floor_h: 42.0,
                ceiling_h: 43.0,
                media_h: 44.0,
                floor_light: 0.0,
                ceiling_light: 1.0,
            },
        ];
        let packed = pack_poly_data(&input);
        let restored = unpack_poly_data(&packed);
        assert_eq!(restored, input);
    }
}
