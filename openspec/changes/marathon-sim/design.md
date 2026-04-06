## Context

Marathon's simulation logic lives in Aleph One's `GameWorld/` directory (~39K LOC C++), which mixes simulation with rendering callbacks, global mutable state, and slot-based fixed-size arrays. Entity state is spread across parallel arrays indexed by "slot" integers, with no type safety or ownership discipline. The simulation is driven at 30 ticks/second and is deterministic given the same random seed and input sequence -- this is what enables Marathon's film replay system and (historically) lockstep multiplayer.

The `marathon-formats` crate already parses all the data the sim needs: `MapData` (polygons, lines, endpoints, objects, platforms, lights, media), `PhysicsData` (player physics, monster definitions, projectile definitions, weapon definitions, effect definitions), and `DamageDefinition`. Coordinates use `i16` world units (1024 = 1 WU) and angles use Marathon's 512-per-revolution system; `marathon-formats` already provides conversion helpers (`fixed_to_f32`, `world_distance_to_f32`, `MarathonAngle::to_radians`).

## Goals / Non-Goals

**Goals:**
- Implement Marathon's game simulation as a standalone, pure-logic Rust crate
- Use bevy_ecs (standalone) for entity-component storage and system scheduling
- Use f32 math via glam for all simulation math (not bit-identical to original fixed-point)
- Maintain determinism: same inputs + seed = same state across runs
- Expose a clean public API for consumers (marathon-integration, marathon-viewer) to construct, advance, and query the simulation
- Support all Marathon 2 gameplay: player movement, monsters, weapons, projectiles, platforms, lights, media, items

**Non-Goals:**
- Bit-for-bit parity with Aleph One's fixed-point arithmetic
- Rendering or audio (this crate has no wgpu/kira dependencies)
- Networking (determinism enables it, but netcode is a separate concern)
- Lua scripting (Aleph One extension, not original Marathon)
- Marathon Infinity-specific features (extended physics, MML overrides) in the initial version

## Decisions

### D1: bevy_ecs for entity storage and system scheduling

**Choice**: Use `bevy_ecs` (standalone, not full Bevy) for all entity-component storage and system scheduling. Replace Aleph One's slot-based parallel arrays with ECS entities and typed components.

**Rationale**: bevy_ecs provides type-safe entity storage, system ordering, change detection, and parallel-safe queries. This replaces the fragile slot-index patterns in C++ while maintaining good cache locality (component storage is archetype-based). The system scheduler handles execution ordering, which maps naturally to Marathon's tick structure (physics before AI before damage resolution).

**Alternatives considered**:
- Raw Vec/HashMap: No scheduling, manual ordering, error-prone lifecycle management
- specs/legion: Less actively maintained than bevy_ecs
- Full Bevy: Too much -- we only need ECS, not the renderer/windowing/asset pipeline

### D2: glam for vector/angle math, f32 throughout

**Choice**: Use `glam::Vec2`/`Vec3` for positions and velocities. All simulation math uses f32. Convert from marathon-formats' i16 world coordinates at the boundary (level load).

**Rationale**: f32 is sufficient precision for Marathon's world scale (maps are typically <128 WU across) and avoids the complexity of implementing Marathon's fixed-point wrapping behavior. glam provides SIMD-accelerated math on supported platforms. The tradeoff is that simulation results won't be bit-identical to the C++ engine, but they will be deterministic within a single build.

**Alternatives considered**:
- Replicating 16.16 fixed-point: Exact parity, but adds significant complexity for marginal benefit. Users wanting exact film playback compatibility can use Aleph One.
- f64: Overkill for Marathon's scale; f32 is more than sufficient.

### D3: Deterministic PRNG via rand with seeded ChaCha8

**Choice**: Use `rand::rngs::StdRng` (ChaCha) seeded at level start. The PRNG is a simulation resource, passed through to all systems that need randomness.

**Rationale**: ChaCha is deterministic across platforms and Rust versions. By making it a single resource threaded through systems (not thread-local or global), the PRNG sequence is reproducible given the same seed and system execution order. This is the foundation for film recording and replay.

### D4: Simulation public API: construct, advance, query

**Choice**: The crate exposes a `SimWorld` struct that wraps a bevy_ecs `World`. Consumers:
1. Construct with `SimWorld::new(map_data, physics_data, config)` which spawns all entities
2. Advance with `sim_world.tick(action_flags)` which runs one 30Hz simulation tick
3. Query state via accessor methods: `players()`, `monsters()`, `projectiles()`, `map_state()`, etc.

**Rationale**: This keeps the bevy_ecs `World` internal. Consumers don't need to know about ECS -- they get a high-level interface. The `tick()` method runs all systems in the correct order for one simulation step.

### D5: Component-per-concern, not mega-components

**Choice**: Use fine-grained components: `Position(Vec3)`, `Velocity(Vec3)`, `Facing(f32)`, `Health(i16)`, `MonsterState(enum)`, `WeaponSlots(...)`, etc. rather than monolithic `Monster { pos, vel, hp, state, ... }` structs.

**Rationale**: Fine-grained components enable ECS queries to access only what they need (e.g., the physics system queries Position+Velocity without touching MonsterState). This improves cache locality for tight loops and makes it easy to add new behaviors by composing components.

### D6: Collision via BSP polygon adjacency, not spatial hash

**Choice**: Use the map's polygon adjacency graph for spatial queries. Point-in-polygon testing uses the polygon's vertex list. Line-of-sight uses the polygon adjacency graph with line intersection. Entity-vs-wall collision walks the adjacency graph from the entity's current polygon.

**Rationale**: Marathon maps are BSP-partitioned with pre-computed polygon adjacency. This is the same approach the original engine uses and is efficient for Marathon's relatively simple geometry. A spatial hash would be redundant since the polygon graph already provides spatial locality.

### D7: Module structure

```
marathon-sim/
  src/
    lib.rs           # SimWorld public API
    world.rs         # World construction from map/physics data
    tick.rs          # System scheduling and tick execution
    components.rs    # All ECS component definitions
    collision.rs     # Line intersection, point-in-polygon, entity collision
    player/
      mod.rs         # Player systems
      movement.rs    # Player movement and physics
      inventory.rs   # Weapon/item inventory
    monster/
      mod.rs         # Monster systems
      ai.rs          # AI state machine and targeting
      pathfinding.rs # Polygon-graph pathfinding
    combat/
      mod.rs         # Combat systems
      weapons.rs     # Weapon firing, reload, trigger logic
      projectiles.rs # Projectile movement and detonation
      damage.rs      # Damage types, calculation, and application
    world_mechanics/
      mod.rs         # World systems
      platforms.rs   # Platform movement and triggers
      lights.rs      # Light animation
      media.rs       # Liquid simulation
      items.rs       # Item spawning and pickup
```

### D8: System execution order within a tick

One tick runs systems in this order:
1. **Input processing**: Apply action flags to player intent
2. **Player physics**: Movement, gravity, collision response
3. **Monster AI**: State machine updates, target acquisition, attack decisions
4. **Weapon/combat**: Weapon firing, projectile creation
5. **Projectile physics**: Projectile movement, collision, detonation
6. **Damage resolution**: Apply queued damage, check kills
7. **World mechanics**: Platform movement, light updates, media updates, item spawns
8. **Cleanup**: Remove dead entities, update polygon occupancy

This matches the original engine's ordering and ensures cause-effect chains resolve within one tick where appropriate.

## Risks / Trade-offs

**[f32 drift vs. original engine]** Using f32 instead of fixed-point means simulation results diverge from the original C++ engine. Films recorded in Aleph One won't replay correctly.
  - Mitigation: This is accepted per the proposal's "spiritually faithful but not bit-identical" stance. The engine is deterministic within itself (f32 ops are deterministic on a given platform with a given compiler).

**[bevy_ecs version churn]** bevy_ecs has a rapid release cycle with breaking changes.
  - Mitigation: Pin to bevy_ecs 0.15. Only core ECS features are used (World, Component, Query, System, Resource, States) which are stable across releases.

**[Monster AI fidelity]** Marathon's monster AI has many subtle behaviors (pathfinding quirks, activation cascades, friendly-fire reactions). Getting these exactly right requires careful study of the C++ source.
  - Mitigation: Each AI state and behavior is a separately testable system. Build tests against known behavior from gameplay.

**[Collision edge cases]** Marathon's BSP collision has known edge cases (thin walls, overlapping polygons, vertices at the same coordinates). The f32 conversion may expose new edge cases.
  - Mitigation: Comprehensive collision tests with edge-case map geometries. Use epsilon-based comparisons where appropriate.

**[Large component count]** Fine-grained ECS components mean many small allocations and archetype permutations.
  - Mitigation: Marathon maps have at most ~hundreds of entities. ECS overhead is negligible at this scale. Optimize only if profiling shows issues.
