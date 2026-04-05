## Why

Marathon's game simulation logic is buried in ~39K lines of tightly coupled C++ (Aleph One's `GameWorld/` directory) that mixes simulation, rendering, and platform concerns through global mutable state and slot-based arrays. To build a modern Rust engine capable of playing Marathon content, we need a standalone simulation library that owns the game tick loop and all gameplay mechanics -- decoupled from rendering and I/O -- so it can be developed, tested, and reasoned about independently.

## What Changes

- Introduce the `marathon-sim` crate: a pure simulation library with no rendering or I/O dependencies.
- Implement a deterministic 30Hz tick loop (matching Marathon's `TICKS_PER_SECOND`) with a seeded PRNG, enabling future networked play.
- Use `bevy_ecs` (standalone, no Bevy renderer) as the entity-component-system framework, replacing C++ global arrays and slot-based indexing with typed components and system queries.
- Use `f32` math via `glam` instead of Marathon's fixed-point 16.16 arithmetic. Simulation will be spiritually faithful but not bit-identical to the original engine.
- Implement player physics: movement, gravity, collision response, step climbing, and media (liquid) interaction.
- Implement monster AI: state machines (idle, attacking, dying, etc.), targeting, pathfinding, and attack patterns.
- Implement the projectile system: creation, movement, gravity, homing, collision detection, and damage application.
- Implement the weapon system: firing, ammunition tracking, reloading, and dual-trigger weapons.
- Implement the damage system: 24+ damage types with scaling and per-type resistance.
- Implement collision detection: line-segment intersection, point-in-polygon tests, and radius-based entity collision.
- Implement platform mechanics: moving floors/ceilings with triggers, speed control, and activation logic.
- Implement light animation: multiple light function types with configurable phase and period.
- Implement media simulation: liquid height derived from light intensity, current flow affecting entities.
- Implement the item system: pickups, inventory management, and item spawning.
- Depend on `marathon-formats` for loading physics definitions, monster definitions, weapon definitions, and map geometry.

## Capabilities

### New Capabilities

- `game-loop`: Core tick loop, system execution ordering, game state lifecycle (loading, playing, paused, completed), deterministic PRNG, and the top-level `update_world_elements_one_tick` equivalent.
- `player-physics`: Player movement integration, gravity, floor/ceiling collision, step climbing, media submersion effects, and player state (health, oxygen, inventory).
- `monster-ai`: Monster behavioral state machines, target acquisition, line-of-sight checks, pathfinding through polygon connectivity, attack pattern sequencing, and death/resurrection logic.
- `combat-system`: Weapon firing mechanics (triggers, ammo, reload cycles, dual weapons), projectile lifecycle (creation, trajectory, homing, detonation), damage calculation (24+ types, scaling factors, resistances), and hit resolution.
- `world-mechanics`: Platform movement (floor/ceiling elevation changes, triggers, speed curves), light animation (function types, phase, period, intensity interpolation), media simulation (liquid levels, flow currents), item pickups and spawning, and control panel activation.

### Modified Capabilities

(none -- no existing specs)

## Impact

- **New crate**: `marathon-sim` added to the workspace.
- **Dependency on `marathon-formats`**: Requires parsed physics models, monster definitions, weapon tables, damage tables, and map polygon/line/endpoint data to initialize the simulation world.
- **Public API surface**: Exposes a `SimulationWorld` (or equivalent) that consumers construct from loaded map/physics data, advance via tick, and query for entity state. This is the primary interface for the renderer (`marathon-viewer`) and the integration layer (`marathon-integration`).
- **No runtime I/O**: The crate performs no file reads, no network calls, no audio playback. All external data is injected at construction or via per-tick input commands.
- **Determinism contract**: Identical inputs and PRNG seed must produce identical simulation state across runs, which is a prerequisite for lockstep networking.
