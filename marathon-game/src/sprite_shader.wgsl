// Marathon Game Sprite Billboard Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var sprite_texture: texture_2d_array<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct SpriteVertexInput {
    @location(0) position: vec3<f32>,   // World position of quad corner
    @location(1) uv: vec2<f32>,         // Texture coordinates
    @location(2) tex_index: u32,        // Texture array layer index
    @location(3) tint: f32,             // Light/tint multiplier
};

struct SpriteVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) tex_index: u32,
    @location(2) tint: f32,
};

@vertex
fn vs_sprite(in: SpriteVertexInput) -> SpriteVertexOutput {
    var out: SpriteVertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.tex_index = in.tex_index;
    out.tint = in.tint;
    return out;
}

@fragment
fn fs_sprite(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(sprite_texture, sprite_sampler, in.uv, in.tex_index);

    // Alpha test: discard transparent pixels
    if color.a < 0.01 {
        discard;
    }

    return vec4<f32>(color.rgb * in.tint, color.a);
}
