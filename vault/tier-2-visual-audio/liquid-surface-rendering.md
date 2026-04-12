---
tags: [tier-2, rendering, media, liquid, water, lava, wgpu]
status: research-complete
---

# Liquid Surface Rendering

How Marathon/Alephone renders water, lava, sewage, goo, and Jjaro media surfaces, and what the Rust rebuild needs to implement.

## Original Alephone Implementation

### Media Types

Alephone defines 5 media (liquid) types in `media.h` / `media_definitions.h`:

| Type | Enum Value | Collection | Shape | Damage | Submerged Fade |
|------|-----------|------------|-------|--------|----------------|
| Water | `_media_water` (0) | walls1 | 19 | None | `under_water` |
| Lava | `_media_lava` (1) | walls2 | 12 | Lava damage (freq 0xF) | `under_lava` |
| Goo | `_media_goo` (2) | walls5 | 5 | Goo damage (freq 0x7) | `under_goo` |
| Sewage | `_media_sewage` (3) | walls3 | 13 | None | `under_sewage` |
| Jjaro | `_media_jjaro` (4) | walls4 | 13 | None | `under_jjaro` |

Source: `Source_Files/GameWorld/media_definitions.h`, `Source_Files/GameWorld/media.h`

### Media Data Structure (32 bytes per record)

Each media instance stores:
- `type` - which of the 5 media types
- `flags` - behavioral modifiers (e.g. `_media_sound_obstructed_by_floor`)
- `light_index` - the light whose intensity drives surface height oscillation
- `current_direction` / `current_magnitude` - flow vector for liquid current
- `low` / `high` - height boundaries (world units)
- `height` - current computed height
- `texture` - shape descriptor (collection + shape index)
- `transfer_mode` - how the surface texture is blended
- `minimum_light_intensity` - floor for surface brightness
- `origin` - texture coordinate origin

### Height Calculation

The core formula from `media.cpp`:
```c
#define CALCULATE_MEDIA_HEIGHT(m) \
    ((m)->low + FIXED_INTEGERAL_PART(((m)->high - (m)->low) * get_light_intensity((m)->light_index)))
```

Media height is driven by a **light source** - the light's intensity (0.0-1.0) interpolates between `low` and `high` bounds. This means media can oscillate, pulse, or flicker based on the linked light's animation function. See [[dynamic-lighting]].

### Surface Rendering Pipeline

In `RenderRasterize.cpp`, liquid surfaces are rendered with specific ordering:

1. **Walls** rendered first
2. **Ceilings** rendered
3. **Far-side exterior objects** (sprites behind the liquid)
4. **Media surface** rendered as a horizontal polygon at current height
5. **Near-side exterior objects** (sprites in front of liquid)
6. **Floors** rendered last

This layering ensures proper transparency compositing.

### Transparency / See-through Liquids

Alephone supports two liquid transparency modes:
- **Opaque liquids** (software renderer default): media replaces the floor surface entirely
- **Semi-transparent liquids** (OpenGL): media rendered as a translucent surface, floor visible beneath

The `SeeThruLiquids` flag is determined by renderer capability:
- OpenGL: controlled by `OGL_Flag_LiqSeeThru` preference
- Software: controlled by `_sw_alpha_off` / `_sw_alpha_fast` / `_sw_alpha_nice`

### Underwater Rendering

When the camera is submerged:
- A **fader effect** tints the screen based on media type (blue for water, red for lava, green for goo/sewage, purple for Jjaro)
- **Fog** is applied in OpenGL mode via `OGL_Fog_BelowLiquid` - linear or exponential fog colored per media type
- Visibility is reduced based on `fog_depth` parameter
- Sounds are attenuated and ambient underwater sound plays

### Media Detonation Effects

Each media type has 4 splash effects:
- `_small_media_detonation_effect` - small splash (e.g. bullet impact)
- `_medium_media_detonation_effect` - medium splash
- `_large_media_detonation_effect` - large splash (e.g. explosion)
- `_large_media_emergence_effect` - object emerging from liquid

These are sprite-based effects from the shapes file. See [[visual-effects-vfx]].

### Media Flow / Current

`update_medias()` in `media.cpp` applies directional flow using the `current_direction` and `current_magnitude` fields. This affects:
- Player movement when submerged (drag factor varies by type)
- Floating objects drift with the current
- The texture origin shifts over time to simulate surface flow

## Current State in Rust Rebuild

### What Exists

**marathon-formats** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/map.rs` lines 528-556):
- `MediaData` struct fully parsed: `media_type`, `flags`, `light_index`, `current_direction`, `current_magnitude`, `low`, `high`, `origin`, `height`, `minimum_light_intensity`, `texture`, `transfer_mode`
- `MediaTypeEnum` with all 5 types plus `Unknown`

**marathon-sim** (`/home/llambit/0_repos/alephone-rust/marathon-sim/src/world_mechanics/media.rs`):
- `compute_media_height(media, light_intensity)` - correct interpolation between `height_low` and `height_high`
- `media_deals_damage()` - correctly flags lava, goo, jjaro
- `media_drag_factor()` - per-type drag values
- Media type constants: `MEDIA_WATER` (0) through `MEDIA_JJARO` (4)

**marathon-viewer** (`/home/llambit/0_repos/alephone-rust/marathon-viewer/src/render.rs` lines 358-380):
- `PolygonGpuData` includes `media_height` and `media_transfer_mode` fields
- Media height uploaded to GPU storage buffer per polygon
- Media texture descriptors collected via `collect_texture_descriptors()`

**marathon-viewer/shader.wgsl**:
- `PolygonData` struct has `media_height` and `media_transfer_mode` but neither is used in the fragment shader

**marathon-viewer/level.rs** (`/home/llambit/0_repos/alephone-rust/marathon-viewer/src/level.rs` lines 121-144):
- `MediaState` struct with `polygon_index`, `current_height`, `low`, `high` - but unused in rendering

### Gaps

1. **No media surface geometry** - media height is uploaded to GPU but no horizontal polygon is generated at the media height level
2. **No media-specific transfer modes** - the shader does not apply any special rendering for media surfaces (ripple, wobble, transparency)
3. **No underwater rendering** - no screen tint/fog when camera is below media height
4. **No media detonation effects** - splash sprites not triggered
5. **No media flow simulation** - current direction/magnitude not applied
6. **No media-linked light animation** - `media_height` is static; not updated per-tick from linked light intensity
7. **No see-through liquid support** - no alpha blending on media surfaces

## Implementation Recommendations

### Phase 1: Static Media Surfaces

1. **Generate media surface mesh**: For each polygon with `media_index >= 0`, emit a horizontal quad at `media_height` using the polygon's vertex positions. Add these vertices to the level mesh with a special flag (e.g., `is_media_surface = true`).

2. **Media surface shader**: In the fragment shader, detect media surface vertices and apply:
   - Semi-transparent alpha (0.6-0.8 depending on media type)
   - The media's texture with appropriate transfer mode
   - A `minimum_light_intensity` floor for brightness

3. **Render ordering**: Use a two-pass approach:
   - Pass 1: Opaque geometry (walls, floors, ceilings) with depth write
   - Pass 2: Translucent geometry (media surfaces) with depth test but no depth write, rendered back-to-front

### Phase 2: Animated Media

4. **Light-driven height**: Each tick, recompute `media_height` from the linked light's current intensity using `compute_media_height()`. Update the `polygon_buffer` GPU data each frame. See [[dynamic-lighting]] for light animation.

5. **Surface ripple effect**: Add a vertex shader displacement for media surface vertices:
   ```wgsl
   // Sine-based ripple displacement
   let ripple = sin(world_pos.x * 4.0 + time * 2.0) * cos(world_pos.z * 3.5 + time * 1.7) * 0.02;
   position.y += ripple;
   ```

6. **Flow animation**: Offset the UV coordinates based on `current_direction` and `current_magnitude` over time.

### Phase 3: Underwater Effects

7. **Camera submersion detection**: Each frame, check if `camera.y < media_height` for the polygon the camera is in.

8. **Underwater fog**: Add a uniform for underwater state. In the fragment shader:
   ```wgsl
   if (is_underwater) {
       let fog_factor = exp(-distance * fog_density);
       color = mix(fog_color, color, fog_factor);
   }
   ```

9. **Screen tint**: Apply a full-screen post-process pass or blend a colored overlay:
   - Water: blue tint `(0.1, 0.2, 0.6, 0.3)`
   - Lava: red tint `(0.6, 0.1, 0.0, 0.4)`
   - Goo: green tint `(0.1, 0.5, 0.1, 0.35)`
   - Sewage: yellow-green tint `(0.3, 0.4, 0.1, 0.3)`
   - Jjaro: purple tint `(0.3, 0.1, 0.5, 0.3)`

### WebGL2 Considerations

The marathon-web crate uses WebGL2 via wgpu's GL backend. Key constraints:
- No storage buffers (already worked around with uniform buffers)
- Alpha blending works but no order-independent transparency
- Use depth peeling or sorted rendering for translucent media
- Keep shader complexity manageable for mobile GPUs

## Related Notes

- [[dynamic-lighting]] - Light animation drives media height
- [[visual-effects-vfx]] - Media detonation splash effects
- [[glow-transfer-modes]] - Transfer modes applied to media surfaces
