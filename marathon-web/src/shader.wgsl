// Marathon Game WGSL Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
    camera_position: vec3<f32>,
    _padding2: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var texture_array: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

// Per-polygon dynamic data texture (Rgba32Float, 2 texels wide, one row per
// polygon — see marathon-web::poly_data). Row p:
//   texel (0,p) = (floor_h, ceiling_h, media_h, floor_light)
//   texel (1,p) = (ceiling_light, _, _, _)
@group(2) @binding(0) var poly_data_tex: texture_2d<f32>;
@group(2) @binding(1) var poly_data_sampler: sampler;

// Surface discriminators carried in the un-baked vertex's position.y
// (matches marathon-web::mesh SURFACE_FLOOR/CEILING/MEDIA).
const SURFACE_FLOOR: f32 = 0.0;
const SURFACE_CEILING: f32 = 1.0;
const SURFACE_MEDIA: f32 = 2.0;

// Fetch the two packed texels of per-polygon dynamic data.
fn poly_texel0(poly_index: u32) -> vec4<f32> {
    return textureLoad(poly_data_tex, vec2<i32>(0, i32(poly_index)), 0);
}
fn poly_texel1(poly_index: u32) -> vec4<f32> {
    return textureLoad(poly_data_tex, vec2<i32>(1, i32(poly_index)), 0);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) texture_descriptor: u32,
    @location(3) light: f32,
    @location(4) transfer_mode: u32,
    @location(5) polygon_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) texture_descriptor: u32,
    @location(2) world_position: vec3<f32>,
    @location(3) @interpolate(flat) light: f32,
    @location(4) @interpolate(flat) transfer_mode: u32,
    @location(5) @interpolate(flat) polygon_index: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Resolve the per-polygon dynamic height. For un-baked floor/ceiling/media
    // surfaces, position.y is a surface discriminator (0/1/2) and the real
    // world height comes from the data texture. Wall vertices are still baked
    // with absolute Y (boxes 2.2 scope) and any other y value passes through.
    let pd0 = poly_texel0(in.polygon_index); // (floor_h, ceiling_h, media_h, floor_light)
    let pd1 = poly_texel1(in.polygon_index); // (ceiling_light, _, _, _)
    var world_y = in.position.y;
    // Light is no longer baked for floor/ceiling/media — take it per-polygon
    // from the data texture (box 3.2). Walls keep their baked in.light.
    var resolved_light = in.light;
    if in.position.y == SURFACE_FLOOR {
        world_y = pd0.x; // floor_h
        resolved_light = pd0.w; // floor_light
    } else if in.position.y == SURFACE_CEILING {
        world_y = pd0.y; // ceiling_h
        resolved_light = pd1.x; // ceiling_light
    } else if in.position.y == SURFACE_MEDIA {
        world_y = pd0.z; // media_h
        resolved_light = pd0.w; // media uses floor_light (matches pre-change baking)
    }

    let world_pos = vec3<f32>(in.position.x, world_y, in.position.z);
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = in.uv;
    out.texture_descriptor = in.texture_descriptor;
    out.world_position = world_pos;
    out.light = resolved_light;
    out.transfer_mode = in.transfer_mode;
    out.polygon_index = in.polygon_index;
    return out;
}

// Transfer mode constants (mirror Alephone map.h enum; all 28 modes)
const TRANSFER_NORMAL: u32 = 0u;
const TRANSFER_FADE_OUT_TO_BLACK: u32 = 1u;
const TRANSFER_INVISIBILITY: u32 = 2u;
const TRANSFER_SUBTLE_INVISIBILITY: u32 = 3u;
const TRANSFER_PULSATE: u32 = 4u;
const TRANSFER_WOBBLE: u32 = 5u;
const TRANSFER_FAST_WOBBLE: u32 = 6u;
const TRANSFER_STATIC: u32 = 7u;
const TRANSFER_FIFTY_PERCENT_STATIC: u32 = 8u;
const TRANSFER_LANDSCAPE: u32 = 9u;
const TRANSFER_SMEAR: u32 = 10u;
const TRANSFER_FADE_OUT_STATIC: u32 = 11u;
const TRANSFER_PULSATING_STATIC: u32 = 12u;
const TRANSFER_FOLD_IN: u32 = 13u;
const TRANSFER_FOLD_OUT: u32 = 14u;
const TRANSFER_HORIZONTAL_SLIDE: u32 = 15u;
const TRANSFER_FAST_HORIZONTAL_SLIDE: u32 = 16u;
const TRANSFER_VERTICAL_SLIDE: u32 = 17u;
const TRANSFER_FAST_VERTICAL_SLIDE: u32 = 18u;
const TRANSFER_WANDER: u32 = 19u;
const TRANSFER_FAST_WANDER: u32 = 20u;
const TRANSFER_BIG_LANDSCAPE: u32 = 21u;
const TRANSFER_REVERSE_HORIZONTAL_SLIDE: u32 = 22u;
const TRANSFER_REVERSE_FAST_HORIZONTAL_SLIDE: u32 = 23u;
const TRANSFER_REVERSE_VERTICAL_SLIDE: u32 = 24u;
const TRANSFER_REVERSE_FAST_VERTICAL_SLIDE: u32 = 25u;
const TRANSFER_2X: u32 = 26u;
const TRANSFER_4X: u32 = 27u;

// Simple hash for static noise
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn apply_transfer_mode(uv: vec2<f32>, mode: u32, time: f32, world_pos: vec3<f32>) -> vec2<f32> {
    switch mode {
        case TRANSFER_PULSATE: {
            let scale = 1.0 + 0.1 * sin(time * 3.0);
            return (uv - vec2<f32>(0.5)) * scale + vec2<f32>(0.5);
        }
        case TRANSFER_WOBBLE: {
            let offset_u = 0.03 * sin(time * 2.0 + world_pos.y * 4.0);
            let offset_v = 0.03 * cos(time * 2.5 + world_pos.x * 4.0);
            return uv + vec2<f32>(offset_u, offset_v);
        }
        case TRANSFER_FAST_WOBBLE: {
            // Same distortion as WOBBLE at 2x frequency.
            let offset_u = 0.03 * sin(time * 4.0 + world_pos.y * 4.0);
            let offset_v = 0.03 * cos(time * 5.0 + world_pos.x * 4.0);
            return uv + vec2<f32>(offset_u, offset_v);
        }
        case TRANSFER_HORIZONTAL_SLIDE: {
            return uv + vec2<f32>(time * 0.5, 0.0);
        }
        case TRANSFER_FAST_HORIZONTAL_SLIDE: {
            return uv + vec2<f32>(time * 1.0, 0.0);
        }
        case TRANSFER_VERTICAL_SLIDE: {
            return uv + vec2<f32>(0.0, time * 0.5);
        }
        case TRANSFER_FAST_VERTICAL_SLIDE: {
            return uv + vec2<f32>(0.0, time * 1.0);
        }
        case TRANSFER_REVERSE_HORIZONTAL_SLIDE: {
            return uv + vec2<f32>(time * -0.5, 0.0);
        }
        case TRANSFER_REVERSE_FAST_HORIZONTAL_SLIDE: {
            return uv + vec2<f32>(time * -1.0, 0.0);
        }
        case TRANSFER_REVERSE_VERTICAL_SLIDE: {
            return uv + vec2<f32>(0.0, time * -0.5);
        }
        case TRANSFER_REVERSE_FAST_VERTICAL_SLIDE: {
            return uv + vec2<f32>(0.0, time * -1.0);
        }
        case TRANSFER_WANDER: {
            // Layered incommensurate sine drift.
            let offset_u = 0.02 * sin(time * 1.3) + 0.01 * sin(time * 2.7);
            let offset_v = 0.02 * cos(time * 1.1) + 0.01 * cos(time * 3.1);
            return uv + vec2<f32>(offset_u, offset_v);
        }
        case TRANSFER_FAST_WANDER: {
            // Same as WANDER at 2x speed.
            let offset_u = 0.02 * sin(time * 2.6) + 0.01 * sin(time * 5.4);
            let offset_v = 0.02 * cos(time * 2.2) + 0.01 * cos(time * 6.2);
            return uv + vec2<f32>(offset_u, offset_v);
        }
        case TRANSFER_LANDSCAPE: {
            // Marathon landscape textures: V = azimuth (wraps horizon), U = elevation
            let dir = normalize(world_pos - camera.camera_position);
            let azimuth = atan2(dir.z, dir.x) / 6.283185 + 0.5;
            let elevation = 0.5 - asin(clamp(dir.y, -1.0, 1.0)) / 3.14159;
            return vec2<f32>(elevation, azimuth);
        }
        case TRANSFER_BIG_LANDSCAPE: {
            // Same projection as LANDSCAPE but widened FOV on the U/elevation term.
            let dir = normalize(world_pos - camera.camera_position);
            let azimuth = atan2(dir.z, dir.x) / 6.283185 + 0.5;
            let elevation = (0.5 - asin(clamp(dir.y, -1.0, 1.0)) / 3.14159) * 0.5;
            return vec2<f32>(elevation, azimuth);
        }
        case TRANSFER_2X: {
            return uv * 2.0;
        }
        case TRANSFER_4X: {
            return uv * 4.0;
        }
        // The following modes are not yet implemented and intentionally fall
        // through to the default (normal) case below:
        //   TRANSFER_FADE_OUT_TO_BLACK (1)   - fade surface to black over time
        //   TRANSFER_INVISIBILITY (2)         - render surface invisible
        //   TRANSFER_SUBTLE_INVISIBILITY (3)  - partial/subtle invisibility
        //   TRANSFER_SMEAR (10)               - smear/blur texture
        //   TRANSFER_FADE_OUT_STATIC (11)     - static that fades out over time
        //   TRANSFER_FOLD_IN (13)             - fold-in transition effect
        //   TRANSFER_FOLD_OUT (14)            - fold-out transition effect
        // (Static-noise modes TRANSFER_STATIC/FIFTY_PERCENT_STATIC/PULSATING_STATIC
        //  are handled in fs_main before this function is called.)
        default: {
            return uv;
        }
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let desc = in.texture_descriptor;

    // Decode ShapeDescriptor: shape_index = bits[7:0]
    let shape_index = desc & 0xFFu;

    let light = in.light;
    let transfer_mode = in.transfer_mode;

    var uv = in.uv;

    // Handle static mode separately (replaces texture entirely)
    if transfer_mode == TRANSFER_STATIC {
        let noise = hash(in.world_position.xz + vec2<f32>(camera.elapsed_time * 100.0));
        return vec4<f32>(vec3<f32>(noise), 1.0) * light;
    }

    // 50% static: per-pixel coin flip between noise and the normal texture.
    if transfer_mode == TRANSFER_FIFTY_PERCENT_STATIC {
        let coin = hash(in.world_position.xz * 7.13 + vec2<f32>(camera.elapsed_time * 13.0));
        if coin < 0.5 {
            let noise = hash(in.world_position.xz + vec2<f32>(camera.elapsed_time * 100.0));
            return vec4<f32>(vec3<f32>(noise), 1.0) * light;
        }
        let base = textureSample(texture_array, texture_sampler, in.uv, shape_index);
        if base.a < 0.01 {
            discard;
        }
        return vec4<f32>(base.rgb * light, base.a);
    }

    // Pulsating static: noise intensity modulated sinusoidally over time.
    if transfer_mode == TRANSFER_PULSATING_STATIC {
        let noise = hash(in.world_position.xz + vec2<f32>(camera.elapsed_time * 100.0));
        let intensity = 0.5 + 0.5 * sin(camera.elapsed_time * 3.0);
        return vec4<f32>(vec3<f32>(noise * intensity), 1.0) * light;
    }

    uv = apply_transfer_mode(uv, transfer_mode, camera.elapsed_time, in.world_position);

    // Sample texture array
    let color = textureSample(texture_array, texture_sampler, uv, shape_index);

    // Discard fully transparent pixels
    if color.a < 0.01 {
        discard;
    }

    return vec4<f32>(color.rgb * light, color.a);
}
