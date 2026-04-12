## 1. Fader State Manager

- [ ] 1.1 Create `FaderBlendMode` enum with variants: Tint, Randomize, Negate, Dodge, Burn, SoftTint
- [ ] 1.2 Create `ActiveFader` struct with fields: color ([f32; 4]), blend_mode, initial_intensity (f32), remaining_ticks (u16), total_ticks (u16), tag (enum for dedup: Damage, Teleport, Invincibility, Oxygen, Shield, Lava, Infravision, None)
- [ ] 1.3 Create `FaderManager` struct with `faders: Vec<ActiveFader>` and methods: `trigger()`, `tick()`, `active_faders()`, `remove_by_tag()`, `clear()`
- [ ] 1.4 Implement `FaderManager::tick()`: decrement remaining_ticks, recompute intensity as `initial_intensity * (remaining / total)`, remove expired faders
- [ ] 1.5 Implement `FaderManager::trigger()`: push a new ActiveFader; for tagged faders, replace existing fader with same tag instead of duplicating
- [ ] 1.6 Add unit test: trigger a fader with duration 10, tick 5 times, verify intensity is ~50% of initial
- [ ] 1.7 Add unit test: trigger a fader with duration 3, tick 4 times, verify fader is removed
- [ ] 1.8 Add unit test: trigger two faders simultaneously, verify both are returned by `active_faders()`
- [ ] 1.9 Add unit test: trigger a tagged fader (Oxygen), trigger again with same tag, verify only one Oxygen fader exists

## 2. MML Fader Configuration

- [ ] 2.1 Create `FaderConfig` struct with per-fader-type defaults: color, blend_mode, duration, base_intensity
- [ ] 2.2 Create `FaderConfigTable` mapping fader type index to `FaderConfig`, with hardcoded Marathon 2 defaults
- [ ] 2.3 Implement MML fader section interpretation: read parsed `faders` entries from `marathon-formats` MML output and override defaults in `FaderConfigTable`
- [ ] 2.4 Add unit test: default config table has red tint for damage (index 0), white randomize for teleport
- [ ] 2.5 Add unit test: MML override changes damage color to blue, verify config table reflects override

## 3. Fader WGSL Shader

- [ ] 3.1 Create `fader.wgsl` vertex shader: compute fullscreen triangle positions from `vertex_index` (vertices at (-1,-1), (3,-1), (-1,3)), output UV coordinates
- [ ] 3.2 Create `fader.wgsl` fragment shader: sample scene texture at UV, read fader uniform (color, intensity, mode, time)
- [ ] 3.3 Implement tint mode (index 0): `mix(scene, scene * color, intensity)`
- [ ] 3.4 Implement randomize mode (index 1): `mix(scene, scene * color, intensity * hash_noise(uv, time))`
- [ ] 3.5 Implement negate mode (index 2): `mix(scene, 1.0 - scene, intensity)`
- [ ] 3.6 Implement dodge mode (index 3): `clamp(scene + color * intensity, 0.0, 1.0)`
- [ ] 3.7 Implement burn mode (index 4): `clamp(scene - color * intensity, 0.0, 1.0)`
- [ ] 3.8 Implement soft_tint mode (index 5): `mix(scene, scene * color, intensity * 0.5)`
- [ ] 3.9 Add hash-based noise function for randomize mode using pixel coords and time uniform

## 4. Render Pipeline Setup (marathon-game)

- [ ] 4.1 Allocate intermediate color texture matching swapchain resolution for scene rendering
- [ ] 4.2 Recreate intermediate texture on window resize
- [ ] 4.3 Create fader uniform buffer (32 bytes: vec4 color, f32 intensity, u32 mode, f32 time, f32 padding)
- [ ] 4.4 Create bind group layout for fader pass: scene texture + sampler + fader uniform
- [ ] 4.5 Create fader render pipeline: fullscreen triangle vertex shader, fader fragment shader, alpha blending to swapchain format
- [ ] 4.6 Modify render loop: render level geometry and sprites to intermediate texture instead of swapchain
- [ ] 4.7 Add fader pass after sprite pass: for each active fader, update uniform buffer and draw fullscreen triangle
- [ ] 4.8 When no faders active, blit intermediate texture to swapchain (or render directly to swapchain)
- [ ] 4.9 Verify HUD pass executes after fader pass, writing to swapchain

## 5. Render Pipeline Setup (marathon-web)

- [ ] 5.1 Allocate intermediate color texture matching swapchain resolution for scene rendering
- [ ] 5.2 Recreate intermediate texture on canvas resize
- [ ] 5.3 Create fader uniform buffer, bind group layout, and render pipeline (same as marathon-game)
- [ ] 5.4 Modify web render loop: render level geometry and sprites to intermediate texture
- [ ] 5.5 Add fader pass after sprite pass, same logic as marathon-game
- [ ] 5.6 Verify HUD pass executes after fader pass

## 6. Sim Event Wiring

- [ ] 6.1 After `sim.tick()`, check `pending_events()` for `EntityDamaged` on the player entity; trigger red tint fader with intensity proportional to damage / max_health
- [ ] 6.2 After `sim.tick()`, check `pending_events()` for `LevelTeleport`; trigger white randomize fader with intensity 1.0, duration 15 ticks
- [ ] 6.3 Each frame, query player invincibility state; if active, maintain gold-green soft_tint fader with cycling color and pulsing intensity; if expired, remove by tag
- [ ] 6.4 Each frame, query player oxygen level; if below 20% max, maintain blue-gray soft_tint fader with intensity scaling inversely with oxygen; if above threshold, remove by tag
- [ ] 6.5 Wire shield recharge event to blue-white dodge fader (intensity 0.4, duration 4 ticks)
- [ ] 6.6 Each frame, query player media submersion; if in lava, maintain orange-red burn fader; if in goo, maintain green tint fader; if in Jjaro goo, maintain blue-white tint fader; on exit, remove by tag
- [ ] 6.7 Each frame, query player infravision state; if active, maintain green soft_tint fader; if expired, remove by tag

## 7. Testing

- [ ] 7.1 Run full cargo test suite in Docker and verify all existing + new tests pass
- [ ] 7.2 Add integration test: create FaderManager, trigger damage fader, tick through full duration, verify state transitions
- [ ] 7.3 Add integration test: trigger multiple faders, verify render ordering (all active faders returned in insertion order)
- [ ] 7.4 Add integration test: verify MML config override applies to triggered fader defaults
- [ ] 7.5 Deploy to marathon.llambit.io and visually verify damage flash when taking damage
- [ ] 7.6 Visually verify teleport static effect on level transition
- [ ] 7.7 Visually verify invincibility glow cycles color while powerup is active
- [ ] 7.8 Visually verify oxygen warning tint when oxygen is low
- [ ] 7.9 Visually verify lava submersion darkens the scene with warm tint
