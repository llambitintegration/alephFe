---
tags: [tier-1, rendering, effects, faders, game-loop]
status: research-complete
---

# Full-Screen Effects

Alephone renders full-screen color overlays ("faders") for damage feedback, powerup activation, teleportation, environmental warnings, and more. These are critical for game feel.

## Original Alephone / Marathon Behavior

### Fader System Overview

The fader system is defined in `screen_drawing.h` and implemented in `screen.cpp` / `faders.cpp`. A "fader" is a full-screen color tint applied as a post-process pass over the rendered frame.

Each fader has:
- A **color** (RGBA, with R/G/B in 0-65535 and alpha intensity)
- A **type** (how the color is composited -- tint, randomize, dodge, burn, soft tint)
- A **duration** in ticks
- A **decay curve** (linear fade out)

### Fader Types (NUMBER_OF_FADER_TYPES)

The engine supports these compositing modes:

| Type | Name | Visual Effect |
|------|------|---------------|
| 0 | `_tint_fader` | Multiplicative tint: screen colors are pushed toward the fader color |
| 1 | `_randomize_fader` | Static noise effect (teleportation) |
| 2 | `_negate_fader` | Color inversion |
| 3 | `_dodge_fader` | Additive brightening (shield recharge flash) |
| 4 | `_burn_fader` | Subtractive darkening (lava damage) |
| 5 | `_soft_tint_fader` | Gentler version of tint (infravision overlay) |

### Standard Game Faders

Marathon uses these specific fader instances during gameplay:

#### Damage Flash (Red Tint)
- **Trigger:** Player takes damage (any type)
- **Color:** Red (65535, 0, 0)
- **Type:** `_tint_fader`
- **Intensity:** Proportional to damage taken (more damage = more opaque red flash)
- **Duration:** ~6-10 ticks, linear fade
- **Notes:** This is the most important feedback mechanism. The intensity scales with `damage_taken / player_max_health`. A near-death hit produces an intense red screen.

#### Teleport Effect
- **Trigger:** Player teleports between levels or within a level
- **Two phases:**
  1. **Fold-out:** Screen stretches horizontally and squeezes vertically (FOV distortion), with white flash
  2. **Fold-in:** Reverse on arrival at destination
- **Color:** White (65535, 65535, 65535)
- **Type:** `_randomize_fader` (static noise during transition)
- **Duration:** ~30 ticks total (15 out + 15 in)
- **Notes:** Also includes a geometric distortion of the viewport (not just a color overlay). The "static" effect makes entities look like they have sparkling interference.

#### Invincibility Glow
- **Trigger:** Player picks up invincibility powerup (Super Shield BCE)
- **Color:** Cycling/pulsing gold-green (the original uses a phase-shifted color cycle)
- **Type:** `_soft_tint_fader`
- **Duration:** Entire powerup duration (1500 ticks). Pulses at a regular rate.
- **Notes:** The tint is not static -- it oscillates in intensity and hue. In OpenGL mode, it also applies a "negative" color effect on the player's weapon sprite.

#### Oxygen Warning
- **Trigger:** Player's oxygen drops below a threshold (typically ~20% remaining)
- **Color:** Blue-gray tint
- **Type:** `_soft_tint_fader`
- **Intensity:** Increases as oxygen decreases. At 0 oxygen, the screen is heavily tinted.
- **Duration:** Continuous while oxygen is low
- **Notes:** Also triggers a warning sound (heavy breathing). The HUD oxygen bar flashes simultaneously.

#### Shield Recharge Flash
- **Trigger:** Player steps onto a shield recharge panel
- **Color:** Blue-white
- **Type:** `_dodge_fader` (additive brightening)
- **Duration:** Brief pulse each time shield increments

#### Lava/Goo Damage Tint
- **Trigger:** Player is submerged in damaging liquid (lava, goo, Jjaro goo)
- **Color:** Orange-red (lava), green (goo), blue-white (Jjaro)
- **Type:** `_burn_fader` or `_tint_fader`
- **Duration:** Continuous while submerged and taking damage

#### Infravision Overlay
- **Trigger:** Infravision powerup active
- **Color:** Green tint applied to the entire scene
- **Type:** `_soft_tint_fader` (or implemented as a shader modification in OpenGL mode)
- **Duration:** Entire powerup duration (1800 ticks)
- **Notes:** In the original software renderer, this was a palette remap to green tones. In OpenGL mode, it applies a green color filter. Enemies and items glow brightly against the green background.

#### Extravision Effect
- **Trigger:** Extravision powerup active
- **FOV change:** Widens the field of view to ~180 degrees (fish-eye effect)
- **Duration:** 1800 ticks
- **Notes:** This is not a color fader but a camera parameter change. The FOV is smoothly expanded and contracted on activation/deactivation.

### MML Configuration

Faders are configurable via Marathon Markup Language (MML):

```xml
<faders>
  <fader index="0" type="tint" color_red="1.0" color_green="0.0" color_blue="0.0" />
  <!-- ... -->
</faders>
```

Color channels are float 0.0-1.0. The `type` attribute maps to the fader compositing modes.

### Implementation in Original Renderer

#### Software Renderer
Faders apply a color lookup table (CLUT) modification to the frame buffer. Each pixel's color is remapped through a tint table.

#### OpenGL Renderer
Faders draw a full-screen quad with the fader color and appropriate blend mode:
- **Tint:** `GL_BLEND` with `glBlendFunc(GL_DST_COLOR, GL_ZERO)` (multiply)
- **Dodge:** `GL_BLEND` with `glBlendFunc(GL_ONE, GL_ONE)` (additive)
- **Burn:** `GL_BLEND` with `glBlendFunc(GL_ZERO, GL_ONE_MINUS_SRC_COLOR)` (subtractive)
- **Randomize:** Custom shader with noise pattern

### Rendering Order

Full-screen effects are rendered **after** the 3D scene and HUD:
1. Render 3D world (walls, floors, ceilings, entities)
2. Render first-person weapon sprite
3. Apply full-screen fader overlay(s) -- multiple can be active simultaneously
4. Render HUD elements (health/shield bars, motion sensor, inventory)

The HUD is rendered AFTER faders in Marathon 2/Infinity, so the HUD is NOT affected by damage tint. However, in Marathon 1, the HUD WAS affected.

## Current State in Rust Rebuild

### Implemented

**Nothing.** There is no fader system, no full-screen effect rendering, and no damage flash, teleport effect, or powerup overlay in the Rust codebase.

### What exists adjacent to this

- **Oxygen system:** `/marathon-sim/src/player/movement.rs` -- `apply_media_effects()` tracks oxygen depletion and drowning damage, but there is no visual warning
- **Damage system:** `/marathon-sim/src/combat/damage.rs` -- damage is calculated and applied, but there is no feedback flash
- **MML parser:** `/marathon-formats/src/mml.rs` -- parses MML files including `faders` section references, but the parsed data is not used for rendering
- **Shader:** `/marathon-game/src/shader.wgsl` -- the main fragment shader outputs colors but has no fader uniform or post-process pass
- **SimEvents:** `/marathon-sim/src/world.rs` -- `SimEvent::EntityDamaged` exists, which could be used to trigger damage flash

### Gaps

1. **No fader component or resource.** There is no ECS component or resource to track active faders (color, type, intensity, remaining duration).

2. **No post-process render pass.** The wgpu render pipeline in `render.rs` does not have a post-process pass for full-screen overlays. The frame goes directly from 3D rendering to presentation.

3. **No damage-to-visual bridge.** When `SimEvent::EntityDamaged` fires, nothing in the rendering layer responds with a visual flash.

4. **No powerup visual state.** Invincibility, infravision, and extravision have no visual representation. Even if the powerup timers were implemented (they are not), there would be no rendering effect.

5. **No teleport effect.** Level transitions have no visual feedback.

6. **No FOV manipulation for extravision.** The camera's field of view is fixed; there is no system to dynamically change it.

## Implementation Recommendations

### Priority 1: Fader resource and tick system

```rust
#[derive(Resource)]
pub struct ActiveFaders {
    pub faders: Vec<ActiveFader>,
}

pub struct ActiveFader {
    pub color: [f32; 4],  // RGBA, alpha = current intensity
    pub blend_mode: FaderBlendMode,
    pub initial_intensity: f32,
    pub remaining_ticks: u16,
    pub total_ticks: u16,
}

pub enum FaderBlendMode {
    Tint,       // Multiplicative
    Dodge,      // Additive
    Burn,       // Subtractive
    SoftTint,   // Gentle multiply
    Randomize,  // Static noise
}
```

Tick each fader: `intensity = initial_intensity * (remaining / total)`. Remove when expired.

### Priority 2: Post-process render pass

Add a second render pass in `marathon-game/src/render.rs`:
1. Render 3D scene to an intermediate texture (render target)
2. Draw a full-screen quad sampling that texture
3. For each active fader, apply its blend mode:
   - **Tint/SoftTint:** `output = lerp(scene_color, scene_color * fader_color, alpha)`
   - **Dodge:** `output = scene_color + fader_color * alpha`
   - **Burn:** `output = scene_color - fader_color * alpha`
4. Output to swapchain

Alternatively, for simplicity, draw a full-screen colored quad with appropriate wgpu blend state after the scene.

### Priority 3: Trigger faders from game events

- **Damage:** On `SimEvent::EntityDamaged` for the player entity, push a red `ActiveFader` with intensity proportional to damage
- **Teleport:** On `SimEvent::LevelTeleport`, push a white `Randomize` fader with 15-tick duration
- **Powerups:** When invincibility/infravision/extravision activate, push continuous faders that refresh each tick

### Priority 4: Oxygen warning

- When player oxygen < 120 (20%), push a blue-gray `SoftTint` fader
- Intensity: `1.0 - (oxygen / 120.0)`
- Emit warning sound event

### Priority 5: Infravision and extravision

- **Infravision:** Green tint fader OR modify the fragment shader to remap colors to green
- **Extravision:** Increase camera FOV from default (~72 degrees) to ~120 degrees. Lerp on activation/deactivation.

## Related Notes

- [[item-pickup-system]] -- Powerup pickups that trigger effects
- [[weapon-behaviors]] -- Weapon firing light (brief flash)
- [[projectile-physics]] -- Explosion screen effects
- [[platform-mechanics]] -- Crush damage triggers red flash

## Sources

- [Alephone screen_drawing.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderOther/screen_drawing.h)
- [MML Documentation](http://tst2005.github.io/alephone-doc/docs/MML.html)
- [Alephone GitHub Issue #266 - Teleport distortion effect](https://github.com/Aleph-One-Marathon/alephone/issues/266)
- [Alephone Lua Documentation](https://github.com/Aleph-One-Marathon/alephone/blob/master/docs/Lua.html)
- [Marathon Wiki - Biobus Chip Enhancements](https://marathongame.fandom.com/wiki/Biobus_Chip_Enhancements)
