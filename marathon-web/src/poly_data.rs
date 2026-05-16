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

#[cfg(test)]
mod tests {
    use super::*;

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
