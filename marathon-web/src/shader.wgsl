// Marathon Game WGSL Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var texture_array: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) texture_descriptor: u32,
    @location(3) light: f32,
    @location(4) transfer_mode: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) texture_descriptor: u32,
    @location(2) world_position: vec3<f32>,
    @location(3) @interpolate(flat) light: f32,
    @location(4) @interpolate(flat) transfer_mode: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.texture_descriptor = in.texture_descriptor;
    out.world_position = in.position;
    out.light = in.light;
    out.transfer_mode = in.transfer_mode;
    return out;
}

// Transfer mode constants
const TRANSFER_NORMAL: u32 = 0u;
const TRANSFER_PULSATE: u32 = 1u;
const TRANSFER_WOBBLE: u32 = 2u;
const TRANSFER_SLIDE: u32 = 4u;
const TRANSFER_STATIC: u32 = 6u;
const TRANSFER_LANDSCAPE: u32 = 9u;

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
        case TRANSFER_SLIDE: {
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

    let light = in.light;
    let transfer_mode = in.transfer_mode;

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
