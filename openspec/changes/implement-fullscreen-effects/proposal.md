## Why

When the player takes damage, gets teleported, picks up invincibility, runs low on oxygen, or triggers any other game event that Marathon signals with a full-screen visual effect, there is zero visual feedback. The engine currently renders the 3D scene and HUD but has no post-process pass and no fader system. Combat feels lifeless because hits are invisible. Teleportation is instantaneous with no flash. Invincibility has no glow. Lava damage has no tint. This is a game-feel blocker that makes the game unplayable by Marathon standards — players cannot read the game state through visual feedback.

Marathon's fader system is a core part of the game's visual language: six blend modes (tint, randomize, negate, dodge, burn, soft_tint) applied as full-screen color overlays with configurable color, intensity, and duration. The MML config system already parses a `faders` section (present in `marathon-formats/src/mml.rs`), but nothing consumes it. The simulation already emits `EntityDamaged`, `LevelTeleport`, and tracks items like `ITEM_INVINCIBILITY`, `ITEM_INFRAVISION`, and `ITEM_EXTRAVISION` — but no rendering code reacts to these events.

## What Changes

- **Add a fader state manager** that tracks active full-screen effects, each with a color (RGBA), blend mode, current intensity, duration, and decay curve. The manager ticks each frame, advancing fader animations and removing expired faders. Multiple faders can be active simultaneously (e.g., damage flash + invincibility glow).
- **Add a post-process render pass** in both `marathon-game` and `marathon-web` renderers. After the 3D scene and sprite passes complete (but before HUD), a fullscreen-triangle pass reads the active fader state from a small uniform buffer and applies screen-space color blending in the fragment shader. The pass renders a single fullscreen triangle with no depth test.
- **Implement the six blend modes in WGSL**: tint (lerp toward color), randomize (per-pixel noise-modulated tint), negate (invert colors blended by intensity), dodge (additive brighten), burn (multiplicative darken), soft_tint (reduced-intensity tint for sustained effects like invincibility).
- **Wire simulation events to fader triggers**: `EntityDamaged` on the player triggers a red damage flash. `LevelTeleport` triggers a white teleport flash. Invincibility pickup triggers a cycling green/yellow glow. Low oxygen triggers a blue tint. Lava/goo submersion triggers an orange/red damage tint. Infravision triggers a green overlay. Extravision triggers a widescreen-style overlay.
- **Parse MML fader configuration**: Consume the already-parsed `faders` MML section to allow plugins to override fader colors, durations, blend modes, and intensity curves per fader type.
- **Add fader-specific shader (fader.wgsl)**: A dedicated post-process fragment shader that receives the active fader uniform data and implements all six blend modes, selected by a mode index in the uniform.

## Capabilities

### New Capabilities
- `fullscreen-effects`: Fader state manager, post-process render pass, fullscreen-triangle pipeline, fader uniform buffer, and WGSL blend-mode shader. Manages the lifecycle of screen-space visual effects triggered by game events or sustained powerup states. Supports six Marathon blend modes. MML-configurable per fader type.

### Modified Capabilities
- `level-rendering`: Render pipeline gains a post-process pass inserted between the sprite pass and HUD overlay. The 3D scene renders to the same surface, and the post-process pass blends fader colors on top before HUD compositing.
- `hud-rendering`: HUD render pass execution order is explicitly sequenced after the new post-process pass (HUD should not be tinted by faders, matching original Marathon behavior where HUD elements are drawn on top of faded gameplay).

## Impact

- **marathon-game/src/render.rs** — Add post-process pipeline creation (fullscreen triangle, fader uniform buffer, bind group). Insert fader render pass between sprite rendering and `queue.submit()`. Consume `SimEvent::EntityDamaged` and `SimEvent::LevelTeleport` to trigger faders. Query player powerup state (invincibility, infravision, extravision) each frame for sustained faders.
- **marathon-web/src/render.rs** — Same post-process pipeline addition for the WASM build. Insert fader pass between sprite rendering and `output.present()`. Wire the same sim event consumption.
- **marathon-game/src/fader.wgsl** (new) — Post-process fragment shader implementing the six blend modes. Reads a fader uniform struct (color, intensity, mode index) and blends with the framebuffer.
- **marathon-web/src/fader.wgsl** (new) — Same shader for the web build (or shared via include).
- **marathon-sim/src/world.rs** — May need new `SimEvent` variants for fader-specific triggers (e.g., `SimEvent::PlayerDamageFlash`, `SimEvent::TeleportFlash`) or the rendering layer interprets existing events. Existing `EntityDamaged` and `LevelTeleport` events are already emitted and sufficient for initial wiring.
- **marathon-sim/src/tick.rs** — Expose player powerup state queries (invincibility active, infravision active, extravision active) so the renderer can drive sustained fader effects. `player_health`/`player_shield`/`player_oxygen` queries already exist.
- **marathon-formats/src/mml.rs** — The `faders` section is already parsed into `MmlSection`. Add interpretation of fader entries to extract per-fader-type color, duration, blend mode, and intensity curve overrides.
- **No changes to marathon-viewer** — The viewer is a free-fly camera tool with no simulation, so faders do not apply.
