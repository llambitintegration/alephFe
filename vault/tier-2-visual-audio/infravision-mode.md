---
tags: [tier-2, rendering, infravision, shader, post-process]
status: research-complete
---

# Infravision Mode

How Marathon/Alephone implements infravision (alien thermal vision), and what the Rust rebuild needs.

## Original Alephone Implementation

### What is Infravision?

Infravision is a power-up in Marathon that gives the player "alien vision" -- a thermal/infrared view mode that makes entities visible in dark areas. It is activated by picking up the `ITEM_INFRAVISION` item (item type 28) and lasts for a configurable duration (`infravision_duration` ticks).

### Visual Characteristics

When infravision is active:
- **All textures render at full brightness** (shadeless) -- dark areas become visible
- **Entity sprites are highlighted** with collection-specific tinting
- **Fog color is adjusted** for the infravision palette
- **The world takes on a monochrome/tinted appearance** based on per-collection color assignments

### Implementation Architecture

The infravision system touches multiple rendering layers:

**Activation Check** (`OGL_Render.cpp`):
```c
bool IsInfravisionActive();  // Set in OGL_SetView()
```

**Shading Mode** (`render.h`):
```c
enum {
    _shading_normal,      // 0 - Standard lighting
    _shading_infravision  // 1 - Infravision active
};
```

When `view->shading_mode == _shading_infravision`, the renderer switches behavior.

### Texture Rendering Under Infravision

**Software Renderer:**
- Uses different shading tables -- infravision shading tables map pixel colors to their tinted equivalents
- The `ModifyCLUT()` function in the texture manager replaces the normal color lookup table with the infravision version

**OpenGL Renderer** (`OGL_Render.cpp`, `OGL_Textures.h`):
- All textures get `_SHADELESS_BIT` set in their polygon flags, eliminating lighting calculations
- Textures render at full brightness
- `FindInfravisionVersionRGBA()` applies per-collection color transformation to texture pixels
- Sprites marked with `_SHADELESS_BIT` render at full white intensity
- Landscape textures receive infravision-adjusted shading tables

### Per-Collection Tinting

The infravision system assigns different tint colors to different texture collections. This is how the engine creates the "thermal" look -- organic entities (monsters, players) might glow warm colors while architecture remains cool-toned.

From the MML documentation, infravision colors are configured as:
```xml
<infravision>
    <assignment coll="..." color="..."/>
</infravision>
```

There are 4 colors available (indexed 0-3), and each collection is assigned one.

Key methods:
- `SetInfravisionTint(collection, tint_color)` - configure tint per collection
- `FindInfravisionVersionRGBA()` - transform texture RGBA data to infravision palette
- `FindSilhouetteVersion()` - related: renders entities as flat silhouettes

### Fog Adjustments

When infravision is active:
- `CurrFogColor` is modified using `FindInfravisionVersionRGBA()` with collection-specific tints
- This ensures fog blends correctly with the infravision palette
- The effect makes underwater areas (which use fog) appear with the infravision tinting

### Glow Interaction

Under infravision:
- Glow textures still render their glow pass
- But since the base pass is already shadeless (full brightness), the glow adds less perceptible difference
- Self-luminous textures look similar with or without infravision active

### Duration and Timing

- Duration is measured in game ticks
- The Lua API exposes `player.infravision_duration` for reading/writing remaining time
- When duration reaches 0, rendering reverts to `_shading_normal`
- The infravision can be reactivated by picking up another infravision item

## Current State in Rust Rebuild

### What Exists

**marathon-formats/mml.rs** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/mml.rs`):
- `infravision` MML section is recognized and parsed (line 39)
- The section data is stored but not interpreted

**marathon-sim/world_mechanics/items.rs** (`/home/llambit/0_repos/alephone-rust/marathon-sim/src/world_mechanics/items.rs`):
- `ITEM_INFRAVISION: i16 = 28` constant defined

**Shader pipeline**:
- No infravision support in any shader (`shader.wgsl`, `sprite_shader.wgsl`)
- No `_SHADELESS_BIT` or shading mode uniform
- No per-collection tinting

### Gaps

1. **No infravision state tracking** - player has no `infravision_duration` field
2. **No shading mode uniform** - shader has no way to know if infravision is active
3. **No shadeless rendering mode** - can't disable polygon lighting
4. **No per-collection color tinting** - no infravision color map
5. **No MML infravision parsing** - section recognized but assignments not extracted
6. **No fog color adjustment** - fog (when implemented) won't react to infravision
7. **No item pickup integration** - picking up ITEM_INFRAVISION doesn't activate anything
8. **No duration countdown** - no per-tick decrement of infravision time

## Implementation Recommendations

### Phase 1: Basic Infravision

1. **Player state extension** in marathon-sim:
   ```rust
   pub struct PlayerState {
       // ... existing fields ...
       pub infravision_duration: i32,  // ticks remaining, 0 = inactive
   }
   ```

2. **Item pickup integration**: When `ITEM_INFRAVISION` is collected, set `infravision_duration` to the standard duration (e.g., 900 ticks = 30 seconds at 30 ticks/sec).

3. **Per-tick countdown**: In the simulation tick, decrement `infravision_duration` if > 0.

### Phase 2: Shader Implementation

4. **Shading mode uniform**: Add to the camera uniform buffer:
   ```rust
   struct CameraUniform {
       view_proj: [f32; 16],
       camera_yaw: f32,
       camera_pitch: f32,
       elapsed_time: f32,
       shading_mode: u32,  // 0 = normal, 1 = infravision
   }
   ```

5. **Shadeless rendering in fragment shader**:
   ```wgsl
   @fragment
   fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
       // ... existing texture sampling ...
       
       var light = poly.floor_light;
       if (camera.shading_mode == 1u) {
           light = 1.0;  // Full brightness under infravision
       }
       
       var final_color = color.rgb * light;
       
       if (camera.shading_mode == 1u) {
           // Apply infravision tint - monochrome green with slight warmth
           let luminance = dot(final_color, vec3<f32>(0.299, 0.587, 0.114));
           final_color = vec3<f32>(
               luminance * 0.2,   // Reduced red
               luminance * 1.0,   // Full green
               luminance * 0.3    // Slight blue
           );
       }
       
       return vec4<f32>(final_color, color.a);
   }
   ```

6. **Entity highlighting in sprite shader**:
   ```wgsl
   @fragment
   fn fs_sprite(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
       let color = textureSample(sprite_texture, sprite_sampler, in.uv, in.tex_index);
       if (color.a < 0.01) { discard; }
       
       var final_color = color.rgb * in.tint;
       
       if (camera.shading_mode == 1u) {
           // Entities glow warm under infravision
           let luminance = dot(final_color, vec3<f32>(0.299, 0.587, 0.114));
           final_color = vec3<f32>(
               luminance * 1.2,   // Warm red/orange
               luminance * 0.8,
               luminance * 0.2
           );
       }
       
       return vec4<f32>(final_color, color.a);
   }
   ```

### Phase 3: Per-Collection Tinting

7. **Collection tint map**: Parse the MML infravision section to build a collection-to-color mapping:
   ```rust
   pub struct InfravisionConfig {
       pub collection_colors: HashMap<u16, [f32; 3]>,
       pub default_color: [f32; 3],  // For unmapped collections
   }
   ```

8. **Pass collection info to shader**: The existing vertex data includes `texture_descriptor` which encodes the collection index. Use this to look up the infravision tint color from a uniform or storage buffer.

9. **4-color palette**: Implement the 4-color system from the MML spec:
   ```rust
   pub const INFRAVISION_COLORS: [[f32; 3]; 4] = [
       [0.2, 1.0, 0.3],   // Color 0: Cool green (architecture)
       [1.0, 0.6, 0.1],   // Color 1: Warm orange (organic)
       [0.8, 0.2, 0.2],   // Color 2: Hot red (dangerous)
       [0.3, 0.3, 1.0],   // Color 3: Cool blue (tech)
   ];
   ```

### Alternative Approach: Post-Process

Instead of modifying the main shader, infravision could be implemented as a post-process pass:

1. Render the scene normally to a framebuffer texture
2. Apply a full-screen quad shader that:
   - Converts to grayscale
   - Applies the green/thermal tint
   - Brightens dark areas (gamma adjustment)

This is simpler but loses per-collection tinting. The per-shader approach is more faithful to Alephone.

### WebGL2 Considerations

- Adding a `shading_mode` to the camera uniform is straightforward
- Per-collection tint colors could be passed as a small uniform array (32 collections x 3 floats = 96 floats = 384 bytes)
- Avoid texture reads for the tint lookup; use uniforms instead

## Related Notes

- [[glow-transfer-modes]] - Glow textures interact with infravision
- [[dynamic-lighting]] - Infravision overrides polygon light levels
- [[overhead-map-automap]] - Infravision could enhance automap visibility
- [[liquid-surface-rendering]] - Underwater fog adjusts for infravision
