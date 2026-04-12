## Context

The Rust Marathon engine renders the 3D scene and HUD but has no post-process pass and no fader system. When the player takes damage, teleports, picks up invincibility, runs low on oxygen, or walks through lava, there is zero visual feedback. Original Marathon uses a fader system -- full-screen color overlays with six compositing modes -- as a core part of its visual language. The MML parser in `marathon-formats/src/mml.rs` already parses a `faders` section, but nothing consumes it. The simulation already emits `SimEvent::EntityDamaged` and `SimEvent::LevelTeleport`, but no rendering code reacts.

The rendering pipeline currently goes: level geometry pass -> entity sprite pass -> HUD overlay pass -> present. There is no intermediate pass for screen-space effects.

## Goals / Non-Goals

**Goals:**
- Full-screen color overlay system matching Marathon's six blend modes
- Fader state manager that ticks per frame, supports multiple simultaneous faders, and handles duration/decay
- Post-process render pass inserted between the sprite pass and HUD overlay
- Game event wiring: damage flash, teleport static, invincibility glow, oxygen warning, shield recharge, lava/goo tint
- MML configuration consumed for per-fader-type color, duration, blend mode, and intensity curve overrides
- Render ordering preserved: 3D scene -> faders -> HUD (HUD is not affected by faders, matching Marathon 2/Infinity behavior)

**Non-Goals:**
- Teleport geometric distortion (FOV warp) -- that is a camera parameter change, not a fader
- Extravision FOV widening -- this is a camera parameter change, separate from the fader system
- Infravision as a shader color remap (the original OpenGL renderer did this) -- we implement it as a soft tint overlay, which is sufficient and matches the software renderer behavior
- Advanced decay curves (exponential, ease-in/out) -- linear decay matches original Marathon
- Per-pixel noise for randomize mode using a noise texture -- a hash-based WGSL implementation is sufficient

## Decisions

### Decision 1: Fader state manager as a standalone struct, not ECS components

**Choice:** Implement `FaderManager` as a plain Rust struct with a `Vec<ActiveFader>` that is owned by the renderer (or passed as a resource). Each `ActiveFader` holds color, blend mode, initial intensity, remaining ticks, and total ticks.

**Alternative considered:** ECS components (one entity per active fader) -- rejected because faders are purely a rendering concern, not simulation entities. They have no position, no collision, no interaction with the game world. A simple Vec with per-frame tick-down is the right abstraction.

**Rationale:** Faders are ephemeral visual state. A flat Vec scanned each frame is cache-friendly and trivially debuggable. The manager exposes `trigger(fader_type, color, blend_mode, intensity, duration)` and `tick(dt)` methods. Expired faders are removed by `tick()`.

### Decision 2: Six blend modes implemented in a single WGSL fragment shader

**Choice:** A dedicated `fader.wgsl` shader implements all six blend modes (tint, randomize, negate, dodge, burn, soft_tint) selected by a `mode` index in the fader uniform buffer. The shader renders a fullscreen triangle (3 vertices, no vertex buffer, positions computed from `vertex_index`).

**Alternative considered:** Using wgpu blend state configuration (e.g., `BlendFactor::Dst`, `BlendFactor::One`) to achieve tint/dodge/burn without a shader -- rejected because randomize (per-pixel noise) and negate (color inversion) cannot be expressed as fixed-function blend state. A unified shader approach handles all six modes consistently.

**Rationale:** One pipeline and one shader for all fader types simplifies the render code. The uniform buffer contains: `color: vec4<f32>`, `intensity: f32`, `mode: u32`, `time: f32` (for randomize animation), and `padding: f32`. Multiple active faders are rendered as multiple draw calls with the same pipeline but different uniform data.

### Decision 3: Fullscreen triangle instead of fullscreen quad

**Choice:** The post-process pass draws a single triangle with vertices at (-1,-1), (3,-1), (-1,3) in clip space, covering the entire screen. Vertex positions are computed from `vertex_index` in the vertex shader with no vertex buffer.

**Alternative considered:** Fullscreen quad (4 vertices, 2 triangles) -- rejected because the triangle approach uses fewer vertices, no index buffer, and no vertex buffer. It is the standard modern approach for fullscreen passes.

**Rationale:** Fewer GPU resources, no buffer allocation, and the rasterizer naturally clips the oversized triangle to the viewport.

### Decision 4: Faders render between sprite pass and HUD, reading from the framebuffer directly

**Choice:** The fader pass writes to the same swapchain surface as the 3D scene. It uses alpha blending to composite the fader color on top of the rendered scene. The HUD pass executes after the fader pass, so HUD elements are not affected by faders.

**Alternative considered:** Rendering the 3D scene to an intermediate texture, then sampling it in the fader shader -- rejected because this requires an extra render target allocation and a texture copy. Since all six blend modes can be expressed as blending a solid color onto the existing framebuffer (with the right blend factors or shader math), we do not need to read back the scene pixels.

**Correction on randomize and negate:** These modes do need to read the scene color. For randomize, the noise modulates intensity but the blend is still a tint. For negate, the shader inverts: `output = mix(scene, 1.0 - scene, intensity)`. This requires reading the scene, which means for negate mode specifically, we need to either: (a) render the scene to a texture and sample it, or (b) use a two-pass approach. **Decision: Use a render-to-texture approach for the fader pass.** The 3D scene and sprites render to an intermediate color attachment. The fader pass samples this texture and outputs to the swapchain. When no faders are active, the intermediate texture is blitted directly (or the scene renders to the swapchain with no fader pass).

**Rationale:** Reading the scene texture is required for negate and randomize. The intermediate texture is only allocated once (at surface-resolution) and reused every frame. The cost is one additional texture sample per pixel per active fader, which is negligible.

### Decision 5: Sim events trigger faders via a bridge function

**Choice:** After each `sim.tick()`, the game loop queries `sim.pending_events()` and calls `fader_manager.trigger()` for relevant events. For sustained effects (invincibility, infravision, low oxygen), the game loop checks player state each frame and either refreshes or removes the corresponding fader.

**Alternative considered:** Having the sim directly emit fader commands -- rejected because the sim should not know about rendering. The sim emits semantic events (`EntityDamaged`, `LevelTeleport`) and the game loop translates them to visual effects.

**Rationale:** Clean separation between simulation and rendering. The game loop is the only place that knows about both sim events and the fader manager.

### Decision 6: MML fader configuration consumed at level load time

**Choice:** When a level loads, the MML `faders` section (already parsed by `marathon-formats`) is read to build a `FaderConfig` table. This table maps fader type indices to `(color, blend_mode, duration, intensity)` defaults. When a fader is triggered, the config table provides the defaults, which the trigger call can override (e.g., damage intensity scales with damage amount).

**Alternative considered:** Hardcoding all fader parameters -- rejected because MML configurability is a core Marathon feature that plugins rely on.

**Rationale:** The MML parser already handles the `faders` section. We just need to interpret the parsed data into a config struct and use it when triggering faders.

## Blend Mode Specifications

| Mode | Index | Shader Operation | Visual Effect |
|------|-------|------------------|---------------|
| Tint | 0 | `mix(scene, scene * color, intensity)` | Push colors toward fader color (multiplicative) |
| Randomize | 1 | `mix(scene, scene * color, intensity * noise)` | Per-pixel noise-modulated tint (static/interference) |
| Negate | 2 | `mix(scene, 1.0 - scene, intensity)` | Color inversion blended by intensity |
| Dodge | 3 | `scene + color * intensity` | Additive brightening |
| Burn | 4 | `scene - color * intensity` | Subtractive darkening |
| Soft Tint | 5 | `mix(scene, scene * color, intensity * 0.5)` | Gentle version of tint for sustained effects |

## Render Pipeline Ordering

```
1. Level geometry pass (walls, floors, ceilings) -> intermediate texture
2. Entity sprite pass (monsters, items, projectiles) -> intermediate texture (shared depth buffer)
3. Fader post-process pass (fullscreen triangle, samples intermediate texture) -> swapchain
   - For each active fader: draw fullscreen triangle with fader uniform
   - When no faders active: blit intermediate texture to swapchain
4. HUD overlay pass (health, shield, oxygen, weapon, radar, inventory) -> swapchain
```

## Risks / Trade-offs

**[Intermediate texture overhead]** Adding a render-to-texture step for the 3D scene introduces GPU memory allocation (one surface-resolution RGBA texture) and a potential performance cost from the extra texture sample. Mitigation: The texture is allocated once and reused. At Marathon's typical polygon counts, the overhead is negligible. If no faders are active, we can skip the fader pass and render directly to the swapchain (optimization for the common case).

**[Multiple active faders]** When multiple faders are active simultaneously (e.g., damage flash + invincibility glow), each requires a separate fullscreen draw call reading and writing the framebuffer. Mitigation: Marathon rarely has more than 2-3 simultaneous faders. The fullscreen triangle draw is extremely lightweight. If needed, multiple faders can be packed into a single draw call with an array uniform.

**[Web build parity]** Both `marathon-game` and `marathon-web` need the fader system. The shader is identical (WGSL), but the render pipeline setup code must be duplicated or shared. Mitigation: The fader manager struct and shader are shared. Pipeline setup follows the same pattern in both renderers -- the duplication is minimal (bind group layout, pipeline creation).

**[MML fader parsing gaps]** The MML parser recognizes the `faders` section but may not extract all fields (color channels, type, duration). Mitigation: Extend the MML interpretation layer as needed. The parser infrastructure is solid; we just need to map parsed attributes to the `FaderConfig` struct.
