// Marathon Game WGSL Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
};

struct PolygonData {
    floor_height: f32,
    ceiling_height: f32,
    floor_light: f32,
    ceiling_light: f32,
    floor_transfer_mode: u32,
    ceiling_transfer_mode: u32,
    media_height: f32,
    media_transfer_mode: u32,
    floor_tex_offset_x: f32,
    floor_tex_offset_y: f32,
    ceiling_tex_offset_x: f32,
    ceiling_tex_offset_y: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<storage, read> polygon_data: array<PolygonData>;
@group(2) @binding(0) var texture_array: texture_2d_array<f32>;
@group(2) @binding(1) var texture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) polygon_index: u32,
    @location(3) texture_descriptor: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) polygon_index: u32,
    @location(2) @interpolate(flat) texture_descriptor: u32,
    @location(3) world_position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.polygon_index = in.polygon_index;
    out.texture_descriptor = in.texture_descriptor;
    out.world_position = in.position;
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
        case TRANSFER_HORIZONTAL_SLIDE: {
            return uv + vec2<f32>(time * 0.5, 0.0);
        }
        case TRANSFER_LANDSCAPE: {
            let u = camera.camera_yaw / 6.283185;
            let v = 0.5 - camera.camera_pitch / 3.14159;
            return vec2<f32>(u, v);
        }
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

    // Read polygon light data
    let poly = polygon_data[in.polygon_index];
    let light = poly.floor_light;
    let transfer_mode = poly.floor_transfer_mode;

    var uv = in.uv;

    // Handle static mode separately (replaces texture entirely)
    if transfer_mode == TRANSFER_STATIC {
        let noise = hash(in.world_position.xz + vec2<f32>(camera.elapsed_time * 100.0));
        return vec4<f32>(vec3<f32>(noise), 1.0) * light;
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
