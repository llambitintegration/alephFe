---
tags: [tier-2, rendering, transfer-modes, glow, shader, textures]
status: research-complete
---

# Glow and Self-Luminous Textures / Transfer Modes

How Marathon/Alephone implements transfer modes and self-luminous (glow) textures, and what the Rust rebuild needs.

## Original Alephone Implementation

### Transfer Mode System

Transfer modes are the Marathon engine's way of controlling how textures are rendered on surfaces. They are "high-level" modes defined in `map.h` that get translated into "low-level" rendering operations in the rasterizer.

### High-Level Transfer Modes (from `map.h`)

```c
enum /* object transfer modes (high-level) */
{
    _xfer_normal,                       // 0  - Standard texture rendering
    _xfer_fade_out_to_black,            // 1  - Fade to black over time
    _xfer_invisibility,                 // 2  - Fully invisible
    _xfer_subtle_invisibility,          // 3  - Partially visible (shimmer)
    _xfer_pulsate,                      // 4  - Polygons only: texture scale pulsates
    _xfer_wobble,                       // 5  - Polygons only: UV distortion
    _xfer_fast_wobble,                  // 6  - Polygons only: faster UV distortion
    _xfer_static,                       // 7  - TV static noise pattern
    _xfer_50percent_static,             // 8  - 50% static noise overlay
    _xfer_landscape,                    // 9  - View-angle-dependent texture mapping
    _xfer_smear,                        // 10 - Solid color fill (converted to _solid_transfer)
    _xfer_fade_out_static,              // 11 - Fade to static
    _xfer_pulsating_static,             // 12 - Pulsating static noise
    _xfer_fold_in,                      // 13 - Horizontal stretch (teleport in)
    _xfer_fold_out,                     // 14 - Vertical squeeze (teleport out)
    _xfer_horizontal_slide,             // 15 - Texture slides horizontally
    _xfer_fast_horizontal_slide,        // 16 - Faster horizontal slide
    _xfer_vertical_slide,               // 17 - Texture slides vertically
    _xfer_fast_vertical_slide,          // 18 - Faster vertical slide
    _xfer_wander,                       // 19 - Texture drifts randomly
    _xfer_fast_wander,                  // 20 - Faster random drift
    _xfer_big_landscape,                // 21 - Wider FOV landscape mode
    _xfer_reverse_horizontal_slide,     // 22 - Reverse horizontal slide
    _xfer_reverse_fast_horizontal_slide,// 23 - Reverse fast horizontal slide
    _xfer_reverse_vertical_slide,       // 24 - Reverse vertical slide
    _xfer_reverse_fast_vertical_slide,  // 25 - Reverse fast vertical slide
    _xfer_2x,                           // 26 - 2x texture scale
    _xfer_4x,                           // 27 - 4x texture scale
};
```

### Low-Level Transfer Modes (from `scottish_textures.h`)

The rasterizer uses a reduced set of internal modes:

```c
enum /* low-level transfer modes */
{
    _tinted_transfer,     // 0 - Apply shading table tint
    _solid_transfer,      // 1 - Flat color (single pixel repeated)
    _big_landscaped_transfer, // 2 - Wide landscape projection
    _textured_transfer,   // 3 - Standard textured rendering
    _shadeless_transfer,  // 4 - Full brightness (no lighting)
    _static_transfer,     // 5 - Random noise pattern
};
```

### High-to-Low Translation

In `RenderRasterize.cpp`, the `instantiate_polygon_transfer_mode()` function maps high-level to low-level:

| High-Level | Low-Level | Notes |
|-----------|----------|-------|
| `_xfer_normal` | `_textured_transfer` | Standard lit texture |
| `_xfer_pulsate` | `_textured_transfer` + UV scale animation | Scale oscillates via sin() |
| `_xfer_wobble` | `_textured_transfer` + UV distortion | UV offset by sin/cos waves |
| `_xfer_fast_wobble` | `_textured_transfer` + faster UV distortion | Same as wobble, faster period |
| `_xfer_static` | `_static_transfer` | Full noise replacement |
| `_xfer_50percent_static` | `_static_transfer` (50% chance) | Half pixels are noise |
| `_xfer_landscape` | `_big_landscaped_transfer` | Spherical projection |
| `_xfer_big_landscape` | `_big_landscaped_transfer` | Wider FOV version |
| `_xfer_horizontal_slide` | `_textured_transfer` + U offset | UV.u += speed * time |
| `_xfer_vertical_slide` | `_textured_transfer` + V offset | UV.v += speed * time |
| `_xfer_wander` | `_textured_transfer` + random UV offset | UV += random drift |
| `_xfer_fold_in/out` | Special render effect | Framebuffer distortion |
| `_xfer_smear` | `_solid_transfer` | Single color fill |
| `_xfer_2x` | `_textured_transfer` + `_SCALE_2X_BIT` | Texture scaled 2x |
| `_xfer_4x` | `_textured_transfer` + `_SCALE_4X_BIT` | Texture scaled 4x |

### Rendering Flags

- `_SHADELESS_BIT` (0x8000) - Render at full brightness, ignoring polygon lighting
- `_SCALE_2X_BIT` (0x0001) - Double texture coordinates
- `_SCALE_4X_BIT` (0x0002) - Quadruple texture coordinates

### Self-Luminous / Glow Textures

Glow textures are a key visual feature in Marathon. They work through a two-pass rendering system:

**Shapes File Properties:**
Each `LowLevelShape` in the shapes file has:
- `transfer_mode` - the default transfer mode for this shape
- `transfer_mode_period` - the animation period for the mode

**OpenGL Two-Pass Glow Rendering (from `OGL_Render.cpp`):**

1. **First Pass (Normal)**: `TMgr.RenderNormal()` - renders the texture with standard lighting (darkened by polygon light level)
2. **Second Pass (Glow)**: If `TMgr.IsGlowMapped()` returns true, calls `TMgr.RenderGlowing()` with **additive blending**:
   ```c
   glBlendFunc(GL_SRC_ALPHA, GL_ONE); // Additive blend
   ```
   The glow pass renders the texture at full brightness, adding light on top of the base rendering.

**Glow Intensity:**
- `MinGlowIntensity()` provides a floor value - even in dark areas, the glow is visible
- In the software renderer, glow is handled through shading table manipulation

**What Makes a Texture "Glow":**
A texture is self-luminous when it has specific transfer modes in its shape definition (from the shapes file) or when the polygon/side has certain transfer modes applied. In practice, textures with `_xfer_pulsate`, `_xfer_wobble`, or marked with self-luminous flags in the shapes data get the glow treatment.

### Landscape Mode

Landscape textures use a special projection that maps the texture based on the camera's view angles rather than world coordinates:

```
u = camera_yaw / (2 * PI)        // Horizontal position = yaw angle
v = 0.5 - camera_pitch / PI      // Vertical position = pitch angle
```

This creates the illusion of a distant background (sky, mountains) that moves with the player's view but not their position. `_xfer_big_landscape` uses a wider field of view for the projection.

In OpenGL, landscape rendering applies azimuth rotation and aspect-ratio scaling via `LandscapeRescale` factor.

### Pulsate Mode

Pulsate scales the texture coordinates around the center:
```
scale = 1.0 + amplitude * sin(time * frequency)
uv = (uv - 0.5) * scale + 0.5
```

### Wobble Mode

Wobble displaces UV coordinates using sine waves:
```
u_offset = amplitude * sin(time * freq + world_pos.y * spatial_freq)
v_offset = amplitude * cos(time * freq + world_pos.x * spatial_freq)
uv += (u_offset, v_offset)
```

`_xfer_fast_wobble` uses the same formula with a faster `freq` value.

### Slide Modes

Slide modes continuously offset UVs in one direction:
```
// Horizontal slide
uv.u += speed * time

// Vertical slide
uv.v += speed * time
```

Fast variants use a higher speed multiplier. Reverse variants negate the direction.

### Wander Mode

Wander uses a pseudo-random walk for UV offset:
```
uv += random_direction * speed * time
```

The random direction changes slowly over time, creating a drifting effect.

## Current State in Rust Rebuild

### What Exists

**marathon-viewer/transfer.rs** (`/home/llambit/0_repos/alephone-rust/marathon-viewer/src/transfer.rs`):
```rust
pub const TRANSFER_NORMAL: u32 = 0;
pub const TRANSFER_PULSATE: u32 = 1;
pub const TRANSFER_WOBBLE: u32 = 2;
pub const TRANSFER_SLIDE: u32 = 4;
pub const TRANSFER_STATIC: u32 = 6;
pub const TRANSFER_LANDSCAPE: u32 = 9;
```
Only 6 of 28 transfer modes are defined. The constant values don't match the original (Alephone's `_xfer_pulsate` = 4, not 1).

**marathon-viewer/shader.wgsl** and **marathon-game/shader.wgsl** (`/home/llambit/0_repos/alephone-rust/marathon-viewer/src/shader.wgsl`):
- `apply_transfer_mode()` function handles 5 modes:
  - `TRANSFER_PULSATE`: UV scale oscillation (correct approach, wrong enum value)
  - `TRANSFER_WOBBLE`: UV distortion with sin/cos (correct approach)
  - `TRANSFER_SLIDE`: Horizontal UV offset (correct but only one direction)
  - `TRANSFER_LANDSCAPE`: View-angle-based UV (correct approach)
  - `TRANSFER_STATIC`: Hash-based noise (correct approach)
- `hash()` function for static noise generation
- Per-polygon `floor_transfer_mode` and `ceiling_transfer_mode` uploaded to GPU

**marathon-formats/shapes.rs** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/shapes.rs` lines 204-227):
- `LowLevelShape` parses `transfer_mode` and `transfer_mode_period` from shapes data
- These are available but not used in the rendering pipeline

**marathon-formats/map.rs**:
- `SideData` parses `primary_transfer_mode`, `secondary_transfer_mode`, `transparent_transfer_mode`
- `PolygonData` parses `floor_transfer_mode`, `ceiling_transfer_mode`
- `MediaData` parses `transfer_mode`

### Gaps

1. **Missing transfer modes** - Only 6 of 28 implemented. Missing: fast_wobble, 50percent_static, fold_in/out, all slide variants (vertical, reverse, fast), wander, big_landscape, 2x, 4x, fade modes, invisibility
2. **Wrong enum values** - Constants don't match Alephone's actual values (e.g., `TRANSFER_PULSATE` should be 4, not 1)
3. **No glow/self-luminous support** - No two-pass rendering, no additive blending, no glow mapping
4. **No per-surface transfer modes** - Only floor transfer mode is used; ceiling and wall transfer modes are uploaded but the shader ignores them
5. **Transfer mode not determined by surface type** - Shader uses `poly.floor_transfer_mode` for all surfaces, not distinguishing floor/ceiling/wall
6. **No shape-level transfer modes** - `LowLevelShape.transfer_mode` not connected to the rendering pipeline
7. **No fold-in/fold-out** - Teleport visual effects require framebuffer distortion
8. **No texture scaling** - `_xfer_2x` and `_xfer_4x` not implemented

## Implementation Recommendations

### Phase 1: Fix Enum Values and Add Missing Modes

1. **Correct the transfer mode enum** to match Alephone:
   ```rust
   pub const XFER_NORMAL: u32 = 0;
   pub const XFER_FADE_OUT_TO_BLACK: u32 = 1;
   pub const XFER_INVISIBILITY: u32 = 2;
   pub const XFER_SUBTLE_INVISIBILITY: u32 = 3;
   pub const XFER_PULSATE: u32 = 4;
   pub const XFER_WOBBLE: u32 = 5;
   pub const XFER_FAST_WOBBLE: u32 = 6;
   pub const XFER_STATIC: u32 = 7;
   pub const XFER_50PERCENT_STATIC: u32 = 8;
   pub const XFER_LANDSCAPE: u32 = 9;
   pub const XFER_SMEAR: u32 = 10;
   pub const XFER_FADE_OUT_STATIC: u32 = 11;
   pub const XFER_PULSATING_STATIC: u32 = 12;
   pub const XFER_FOLD_IN: u32 = 13;
   pub const XFER_FOLD_OUT: u32 = 14;
   pub const XFER_HORIZONTAL_SLIDE: u32 = 15;
   pub const XFER_FAST_HORIZONTAL_SLIDE: u32 = 16;
   pub const XFER_VERTICAL_SLIDE: u32 = 17;
   pub const XFER_FAST_VERTICAL_SLIDE: u32 = 18;
   pub const XFER_WANDER: u32 = 19;
   pub const XFER_FAST_WANDER: u32 = 20;
   pub const XFER_BIG_LANDSCAPE: u32 = 21;
   pub const XFER_REVERSE_HORIZONTAL_SLIDE: u32 = 22;
   pub const XFER_REVERSE_FAST_HORIZONTAL_SLIDE: u32 = 23;
   pub const XFER_REVERSE_VERTICAL_SLIDE: u32 = 24;
   pub const XFER_REVERSE_FAST_VERTICAL_SLIDE: u32 = 25;
   pub const XFER_2X: u32 = 26;
   pub const XFER_4X: u32 = 27;
   ```

2. **Expand shader `apply_transfer_mode()`** to handle all slide/wander variants:
   ```wgsl
   case XFER_FAST_WOBBLE: {
       // Same as wobble but 2x frequency
       let offset_u = 0.03 * sin(time * 4.0 + world_pos.y * 4.0);
       let offset_v = 0.03 * cos(time * 5.0 + world_pos.x * 4.0);
       return uv + vec2<f32>(offset_u, offset_v);
   }
   case XFER_VERTICAL_SLIDE: {
       return uv + vec2<f32>(0.0, time * 0.5);
   }
   case XFER_FAST_HORIZONTAL_SLIDE: {
       return uv + vec2<f32>(time * 1.0, 0.0);
   }
   case XFER_WANDER: {
       let wx = sin(time * 0.3) * 0.1 + cos(time * 0.17) * 0.05;
       let wy = cos(time * 0.25) * 0.1 + sin(time * 0.13) * 0.05;
       return uv + vec2<f32>(wx, wy);
   }
   case XFER_2X: {
       return uv * 2.0;
   }
   case XFER_4X: {
       return uv * 4.0;
   }
   ```

### Phase 2: Per-Surface Transfer Modes

3. **Pass surface type to fragment shader**: Extend the vertex data or polygon data to distinguish floor/ceiling/wall surfaces, so the correct transfer mode is applied to each.

4. **Wall transfer modes from SideData**: When building wall mesh, use `side.primary_transfer_mode` etc. instead of the polygon's floor transfer mode.

### Phase 3: Glow / Self-Luminous

5. **Two-pass rendering for glow**:
   - Pass 1: Normal textured rendering with polygon lighting
   - Pass 2: For surfaces with glow, render again with additive blending at full brightness
   
   In wgpu, this means a second render pass (or draw call) with:
   ```rust
   blend: Some(wgpu::BlendState {
       color: wgpu::BlendComponent {
           src_factor: wgpu::BlendFactor::SrcAlpha,
           dst_factor: wgpu::BlendFactor::One,  // Additive
           operation: wgpu::BlendOperation::Add,
       },
       alpha: wgpu::BlendComponent::OVER,
   })
   ```

6. **Identify glow surfaces**: Check the shapes file's `LowLevelShape.transfer_mode` or specific surface transfer modes that indicate self-luminous rendering.

7. **Glow intensity floor**: Even in dark polygons, glow textures should have a minimum brightness (e.g., 0.3).

### Phase 4: Special Effects

8. **Fold-in/Fold-out**: Implement as a post-process shader that distorts the rendered framebuffer:
   ```wgsl
   // Fold-out: stretch horizontally, squeeze vertically
   let fold_progress = animation_phase; // 0.0 to 1.0
   let stretch_x = 1.0 + fold_progress * 2.0;
   let squeeze_y = 1.0 - fold_progress * 0.8;
   uv = (uv - 0.5) * vec2(1.0/stretch_x, 1.0/squeeze_y) + 0.5;
   ```

9. **50% static overlay**: Mix noise with the base texture at 50% probability per pixel.

## Related Notes

- [[liquid-surface-rendering]] - Media surfaces use transfer modes
- [[visual-effects-vfx]] - Fold-in/out used for teleport effects
- [[infravision-mode]] - Infravision sets `_SHADELESS_BIT` on all textures
- [[dynamic-lighting]] - Glow textures bypass polygon light levels
