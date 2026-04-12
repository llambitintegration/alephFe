---
tags: [tier-2, rendering, lighting, animation, shader]
status: research-complete
---

# Dynamic Lighting

How Marathon/Alephone's light system works: stationary lights with animation functions, light state machines, and how lights affect polygon rendering.

## Original Alephone Implementation

### Light System Overview

Marathon uses a per-polygon lighting model where each polygon has a floor and ceiling light source index. Lights are not point lights or directional lights -- they are **ambient light levels** attached to polygons that can animate over time through various functions.

Source files: `lightsource.h`, `lightsource.cpp`

### Light Types

```c
enum /* default light types */
{
    _normal_light,   // 0 - Standard animated light
    _strobe_light,   // 1 - Rapid on/off cycling
    _media_light,    // 2 - Light that drives media height
    NUMBER_OF_LIGHT_TYPES  // 3
};
```

`_media_light` is special: it controls the height of liquid surfaces (see [[liquid-surface-rendering]]). The light's intensity directly drives the media height interpolation.

### Lighting Animation Functions (6 types)

```c
enum /* lighting functions */
{
    _constant_lighting_function,     // 0 - Hold at final intensity
    _linear_lighting_function,       // 1 - Linear ramp between initial/final
    _smooth_lighting_function,       // 2 - Cosine interpolation (smooth ease)
    _flicker_lighting_function,      // 3 - Smooth base + random variation
    _random_lighting_function,       // 4 - Pure random between initial/final
    _fluorescent_lighting_function,  // 5 - Random on/off toggle
    NUMBER_OF_LIGHTING_FUNCTIONS     // 6
};
```

**Mathematical Details:**

**Constant**: Returns `final_intensity` unchanged, ignoring phase. Used for static light levels.

**Linear**: Linear interpolation over the period:
```
intensity = initial + ((final - initial) * phase) / period
```
Creates triangle-wave oscillation when cycling through states.

**Smooth**: Cosine-based easing:
```
intensity = initial + ((final - initial) * (cos(phase * PI / period + PI) + 1)) / 2
```
Produces a sinusoidal fade between initial and final intensities.

**Flicker**: Combines smooth interpolation with random variation:
```
smooth_value = smooth_lighting_function(phase, period, initial, final)
intensity = smooth_value + random() * (final - smooth_value)
```
This creates a base oscillation with random jitter on top.

**Random**: Pure random value each tick:
```
intensity = initial + random() * (final - initial)
```

**Fluorescent**: Binary random toggle with 50% probability each frame:
```
intensity = (random() > 0.5) ? final : initial
```
Simulates a flickering fluorescent tube.

### Light State Machine (6 states)

Each light cycles through a state machine:

```
_light_becoming_active       -> _light_primary_active
_light_primary_active        -> _light_secondary_active
_light_secondary_active      -> (loops back or) _light_becoming_inactive
_light_becoming_inactive     -> _light_primary_inactive
_light_primary_inactive      -> _light_secondary_inactive
_light_secondary_inactive    -> (loops back or) _light_becoming_active
```

Each state has its own `lighting_function_specification`:
- **function type** (one of the 6 functions above)
- **period** (ticks for one cycle)
- **delta_period** (random variation added to period)
- **intensity** (target intensity for this state, fixed-point)
- **delta_intensity** (random variation added to intensity)

### Static Light Data Structure (100 bytes)

From `lightsource.h`:

```c
struct static_light_data {
    int16 type;       // _normal_light, _strobe_light, _media_light
    uint16 flags;     // _light_is_initially_active, _light_has_slaved_intensities, _light_is_stateless
    int16 phase;      // Initial phase offset (for synchronization)
    
    // Six function specifications (14 bytes each = 84 bytes):
    lighting_function_specification primary_active;
    lighting_function_specification secondary_active;
    lighting_function_specification becoming_active;
    lighting_function_specification primary_inactive;
    lighting_function_specification secondary_inactive;
    lighting_function_specification becoming_inactive;
    
    int16 tag;        // Script/trigger tag
};
```

### Static Light Flags

- `_light_is_initially_active` - Start in the active state
- `_light_has_slaved_intensities` - Secondary functions use primary's intensity values
- `_light_is_stateless` - Don't remember on/off state across saves
- `NUMBER_OF_STATIC_LIGHT_FLAGS` (max 16, stored as bitfield)

### Lighting Function Specification (14 bytes)

```c
struct lighting_function_specification {
    int16 function;        // Which of the 6 function types
    int16 period;          // Base period in ticks
    int16 delta_period;    // Random addition to period
    fixed intensity;       // Target intensity (fixed-point)
    fixed delta_intensity; // Random addition to intensity
};
```

### Runtime Light Data (128 bytes)

```c
struct light_data {
    uint32 flags;           // Active/inactive state flags
    int16 state;            // Current state machine state
    fixed intensity;        // Current computed intensity
    int16 phase;            // Current phase within period
    int16 period;           // Current state's period (with delta applied)
    fixed initial_intensity;// Intensity at state entry
    fixed final_intensity;  // Target intensity for current state
    // ... plus the full static_light_data
};
```

### Update Loop

`update_lights()` is called each game tick:
1. For each active light:
   - Increment `phase` by 1
   - If `phase >= period`, call `rephase_light()` to transition to next state
   - Call the current state's lighting function to compute `intensity`
   - Each state transition randomizes period and final intensity using delta values:
     ```
     period = base_period + random() % (delta_period + 1)
     final_intensity = base_intensity + random() * delta_intensity
     ```

### How Lights Affect Polygon Rendering

Each polygon references two light indices:
- `floor_lightsource_index` - light for the floor and lower walls
- `ceiling_lightsource_index` - light for the ceiling and upper walls

During rendering, the light's current `intensity` (0.0 to 1.0) is used as a brightness multiplier for the polygon's textures. In the fragment shader, this becomes:
```
final_color = texture_color * light_intensity
```

### Marathon 1 Legacy Light Types

Marathon 1 used a simpler system:
```c
_light_is_normal, _light_is_rheostat, _light_is_flourescent,
_light_is_strobe, _light_flickers, _light_pulsates,
_light_is_annoying, _light_is_energy_efficient
```
These are converted to the M2/Infinity format on load.

## Current State in Rust Rebuild

### What Exists

**marathon-formats/map.rs** (lines 19, 100-byte `StaticLightData`):
- `StaticLightData` struct parsed from WAD
- `LightData` enum with `Static(Vec<StaticLightData>)`, `Old(Vec<OldLightData>)`, and `None`
- Old light format also parsed for Marathon 1 compatibility

**marathon-viewer/level.rs** (`/home/llambit/0_repos/alephone-rust/marathon-viewer/src/level.rs` lines 147-178):
- `evaluate_light_intensity()` function that reads light data
- For static lights: uses `primary_active.intensity` as a static value
- For old lights: uses `intensity` field directly
- Returns clamped 0.0-1.0 value

**marathon-sim/world_mechanics/lights.rs** (`/home/llambit/0_repos/alephone-rust/marathon-sim/src/world_mechanics/lights.rs`):
- `Light` component with `function`, `period`, `phase`, `intensity_min`, `intensity_max`, `current_intensity`
- `LightFunction` enum: `Constant`, `Linear`, `Smooth`, `Flicker` (4 of 6 types)
- `compute_light_intensity()` correctly implements:
  - **Constant**: returns `intensity_max`
  - **Linear**: triangle wave over period
  - **Smooth**: cosine wave over period
  - **Flicker**: random value in range
- Phase offset support
- Tests for all 4 function types

**Shader pipeline** (all 3 crates):
- `PolygonData` struct includes `floor_light` and `ceiling_light` fields
- `floor_light` is used in the fragment shader: `return vec4(color.rgb * light, color.a)`
- `ceiling_light` is available but not used (all surfaces use `floor_light`)

### Gaps

1. **Static light evaluation** - `evaluate_light_intensity()` only reads `primary_active.intensity` once. No tick-by-tick animation.
2. **Missing lighting functions** - `_random_lighting_function` and `_fluorescent_lighting_function` not implemented
3. **No state machine** - The 6-state transition system is not implemented. Lights don't transition between becoming_active, primary_active, secondary_active, etc.
4. **No delta randomization** - `delta_period` and `delta_intensity` not used for variation
5. **No per-tick light updates** - Light intensities are computed once at level load, not updated each tick
6. **No GPU buffer updates** - Even if lights animated, the `polygon_buffer` is not updated per-frame for light changes
7. **Ceiling light unused** - Shader uses `floor_light` for everything; ceiling surfaces should use `ceiling_light`
8. **No light type distinction** - `_normal_light`, `_strobe_light`, `_media_light` not differentiated
9. **No media-light coupling** - Media lights don't drive liquid surface height in the renderer
10. **No firing light** - Weapon discharge doesn't temporarily increase polygon lighting
11. **No light tags** - Script/trigger tag on lights not connected

## Implementation Recommendations

### Phase 1: Per-Tick Light Animation

1. **Full state machine in marathon-sim**: Extend the `Light` component:
   ```rust
   pub struct Light {
       pub light_type: LightType,       // Normal, Strobe, Media
       pub state: LightState,           // 6 states
       pub flags: u16,
       pub phase: u32,                  // Current phase in ticks
       pub period: u32,                 // Current state's period
       pub intensity: f32,              // Current computed intensity
       pub initial_intensity: f32,      // Intensity at state entry
       pub final_intensity: f32,        // Target for current state
       pub functions: [LightFunctionSpec; 6], // One per state
       pub tag: i16,
   }
   
   pub struct LightFunctionSpec {
       pub function: LightFunction,
       pub period: u16,
       pub delta_period: u16,
       pub intensity: f32,
       pub delta_intensity: f32,
   }
   ```

2. **Add missing functions**:
   ```rust
   pub enum LightFunction {
       Constant,    // 0
       Linear,      // 1
       Smooth,      // 2
       Flicker,     // 3
       Random,      // 4 - NEW
       Fluorescent, // 5 - NEW
   }
   ```
   
   Random: `intensity_min + rng.gen::<f32>() * range`
   Fluorescent: `if rng.gen::<bool>() { intensity_max } else { intensity_min }`

3. **State transitions**: Implement `rephase_light()` that advances to the next state when phase >= period, randomizing the new period and target intensity.

### Phase 2: GPU Updates

4. **Per-frame polygon buffer update**: After all lights are updated each tick, recompute `floor_light` and `ceiling_light` for each polygon and write the changes to the GPU buffer:
   ```rust
   for (i, poly) in polygons.iter().enumerate() {
       let floor_light = lights.get_intensity(poly.floor_lightsource_index);
       let ceiling_light = lights.get_intensity(poly.ceiling_lightsource_index);
       let offset = i * std::mem::size_of::<PolygonGpuData>();
       queue.write_buffer(&polygon_buffer, (offset + 8) as u64,
           bytemuck::bytes_of(&floor_light));
       queue.write_buffer(&polygon_buffer, (offset + 12) as u64,
           bytemuck::bytes_of(&ceiling_light));
   }
   ```

5. **Use correct light per surface**: In the shader, distinguish floor/ceiling/wall surfaces and apply the appropriate light value from `PolygonData`.

### Phase 3: Media Lights and Special Lighting

6. **Media-light coupling**: For lights with `_media_light` type, use their intensity to drive media height:
   ```rust
   for media in &mut media_instances {
       let intensity = lights.get_intensity(media.light_index);
       media.current_height = media.low + (media.high - media.low) * intensity;
   }
   ```
   Then update the polygon buffer's `media_height` field.

7. **Firing light**: When a weapon fires, temporarily boost the player's polygon light:
   ```rust
   pub fn apply_firing_light(polygon_index: usize, intensity: f32, decay_ticks: u16) {
       // Add a temporary light boost that decays over decay_ticks frames
       // This modifies the polygon's effective light intensity
   }
   ```

8. **Light-activated triggers**: Connect light tags to the trigger system for script-activated lights.

### Performance Considerations

- **Partial buffer updates**: Only update polygons whose lights actually changed this tick. Track dirty flags per light.
- **Light grouping**: Polygons sharing the same light source can be batched for a single intensity lookup.
- **WebGL2**: The polygon data is already in a uniform/storage buffer. Writing partial updates is efficient with `queue.write_buffer()` at specific offsets.
- **Interpolation**: For smooth rendering between ticks, interpolate light intensities: `lerp(prev_intensity, curr_intensity, alpha)` where alpha is the sub-tick fraction.

## Related Notes

- [[liquid-surface-rendering]] - Media lights drive liquid surface height
- [[glow-transfer-modes]] - Glow textures bypass polygon lighting
- [[infravision-mode]] - Infravision overrides all lighting to full brightness
- [[visual-effects-vfx]] - Weapon firing light is a temporary polygon light boost
