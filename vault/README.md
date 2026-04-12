# Marathon Rust Engine - Research Vault

Research vault for the Rust rebuild of the Alephone engine (Marathon / Marathon 2).

## Structure

- **[[tier-1-game-loop/index|Tier 1: Game Loop]]** - Item pickups, weapon behaviors, projectile physics, platform mechanics, screen effects
- **[[tier-2-visual-audio/index|Tier 2: Visual & Audio]]** - Liquid animation, VFX, overhead map, glow textures, infravision
- **[[tier-3-content-pipeline/index|Tier 3: Content Pipeline & Modding]]** - MML overrides, Lua VM integration, plugin patching
- **[[tier-4-multiplayer/index|Tier 4: Multiplayer & Advanced]]** - Network sync, film/replay system, control remapping
- **[[architecture/index|Architecture]]** - Crate structure, ECS patterns, rendering pipeline, comparison with alephone C++
- **[[alephone-reference/index|Alephone Reference]]** - Original engine documentation, file formats, game mechanics

## Status

| Tier | Focus | Current % | Target |
|------|-------|-----------|--------|
| 1 | Complete Game Loop | ~70% | Playable single-player |
| 2 | Visual/Audio Polish | ~50% | Match original feel |
| 3 | Content Pipeline | ~10% | Community content support |
| 4 | Multiplayer | ~20% | Network play |
