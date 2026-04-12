---
tags: [architecture, ecs, bevy, simulation]
---

# ECS Architecture

The simulation layer (`marathon-sim`) uses `bevy_ecs 0.15` as a standalone ECS library, NOT the full Bevy engine. The ECS world is driven manually -- there is no Bevy scheduler, no App builder, no system sets. Instead, `SimWorld` wraps a raw `bevy_ecs::World` and advances it through direct queries in a carefully ordered `tick()` method.

## Why bevy_ecs Without Bevy

The project uses bevy_ecs for:
- Component storage with compile-time type safety
- Archetypal queries (query-filtered, With<T>, etc.)
- Resources for global state
- Entity spawning/despawning

It does NOT use:
- Bevy's App, Schedule, or SystemStages
- Bevy's rendering, windowing, asset, or input systems
- Any Bevy plugins

This gives the simulation full control over execution order and determinism, while still benefiting from the ergonomic component model.

## Components

All components are defined in `marathon-sim/src/components.rs`. They are grouped by purpose:

### Spatial Components

| Component | Type | Description |
|-----------|------|-------------|
| `Position(Vec3)` | newtype | World-space position |
| `Velocity(Vec3)` | newtype | Velocity in world units per tick (stored in player-local frame for player) |
| `Facing(f32)` | newtype | Horizontal facing angle in radians (0 = east, CCW) |
| `VerticalLook(f32)` | newtype | Vertical look angle in radians (positive = up) |
| `AngularVelocity(f32)` | newtype | Turning speed in radians per tick |
| `CollisionRadius(f32)` | newtype | Radius for entity-wall and entity-entity collision |
| `EntityHeight(f32)` | newtype | Height for ceiling clearance checks |
| `PolygonIndex(usize)` | newtype | Current polygon the entity occupies |
| `Grounded(bool)` | newtype | Whether standing on a floor |

### Vitality Components

| Component | Type | Description |
|-----------|------|-------------|
| `Health(i16)` | newtype | Hit points (death at 0) |
| `Shield(i16)` | newtype | Shield points (absorb damage first) |
| `Oxygen(i16)` | newtype | Oxygen supply (depletes when submerged) |

### Entity Type Markers

| Component | Purpose |
|-----------|---------|
| `Player` | Marker for the player entity (unit struct) |
| `Monster { definition_index }` | Marks a monster with its physics table index |
| `Projectile { definition_index, distance_traveled }` | Marks a projectile |
| `ProjectileSource(Entity)` | Tracks who fired a projectile |
| `Item { item_type }` | Marks an item pickup |
| `Effect { definition_index, ticks_remaining }` | Marks a visual effect |

### Monster AI Components

| Component | Purpose |
|-----------|---------|
| `MonsterState` | Enum: Idle, Alerted, Attacking, Moving, Fleeing, Dying, Dead |
| `Target(Option<Entity>)` | Current target entity |
| `AttackCooldown(u16)` | Ticks until next attack |
| `Flying { preferred_hover_height }` | Present on flying monsters |

### Combat Components

| Component | Purpose |
|-----------|---------|
| `Immunities(u32)` | Damage type immunity bitmask |
| `Weaknesses(u32)` | Damage type weakness bitmask (2x damage) |

### Rendering Hints

| Component | Purpose |
|-----------|---------|
| `SpriteShape(u16)` | Shape descriptor for rendering |
| `AnimationFrame(u16)` | Current animation frame index |

### World Mechanic Components

| Component | Purpose |
|-----------|---------|
| `Platform` | Full platform state: polygon, heights, speed, state machine, activation flags, crushes |
| `Light` | Light animation: function type, period, phase, intensity range |
| `Media` | Liquid state: type, height bounds, current height, flow direction |

## Resources

Resources are global singleton state stored in the bevy_ecs World:

| Resource | Type | Purpose |
|----------|------|---------|
| `MapGeometry` | struct | Pre-built geometry for collision: polygon vertices, floor/ceiling heights, adjacency graph, line endpoints, solid/transparent flags |
| `PhysicsTables` | struct | Full PhysicsData from the scenario (monster defs, weapon defs, etc.) |
| `PlayerPhysicsParams` | struct | Extracted player movement constants (max velocities, accel/decel, gravity, angular params) |
| `SimRng` | StdRng wrapper | Deterministic PRNG for all random simulation decisions |
| `TickCounter` | u64 wrapper | Current simulation tick number |
| `SimEvents` | Vec<SimEvent> | Pending events for the integration layer |
| `TickInput` | struct | Per-tick input: ActionFlags + mouse yaw/pitch deltas |

## SimWorld API

`SimWorld` is the top-level simulation handle. It wraps a `bevy_ecs::World` and exposes:

### Construction

```rust
SimWorld::new(map_data, physics_data, config) -> Result<Self, SimWorldError>
```

This:
1. Creates a fresh bevy_ecs World
2. Inserts resources (RNG, tick counter, events, physics tables, geometry, player params)
3. Spawns entities from map objects (player, monsters, items)
4. Spawns platforms, lights, and media from map data

### Tick Advancement

```rust
sim_world.tick(input: TickInput)
```

The tick pipeline (documented order from the tick() docstring):

```
1. Input processing    -- TickInput resource inserted
2. Player physics      -- Velocity, facing, vertical look, collision
3. Monster AI          -- (stub -- system ordering defined)
4. Weapon/combat       -- (stub)
5. Projectile physics  -- (stub)
6. Damage resolution   -- (stub)
7. World mechanics     -- Platforms, lights, media, items (stubs)
8. Cleanup             -- Tick counter increment
```

Currently only player physics (step 2) is fully wired into the tick loop. The other systems have their logic implemented in standalone functions but are not yet called from `tick()`.

### Queries

```rust
sim_world.player_position() -> Option<Vec3>
sim_world.player_facing() -> Option<f32>
sim_world.player_health() -> Option<i16>
sim_world.player_shield() -> Option<i16>
sim_world.player_oxygen() -> Option<i16>
sim_world.player_vertical_look() -> Option<f32>
sim_world.player_polygon() -> Option<usize>
sim_world.entities() -> Vec<EntityRenderState>
sim_world.drain_events() -> Vec<SimEvent>
```

These use `query_filtered` with `With<Player>` to find the player entity, or plain queries for all renderable entities.

### Serialization

```rust
sim_world.snapshot() -> SimSnapshot
sim_world.serialize() -> Result<Vec<u8>, bincode::Error>
SimWorld::deserialize(data, map_data, physics_data) -> Result<Self, SimWorldError>
```

Snapshots capture all entity state. The geometry is NOT serialized -- it is rebuilt from map_data on load. Entity IDs are ephemeral (bevy_ecs Entity) so components like `Target` and `ProjectileSource` are not serialized.

## Player Physics Pipeline (Detailed)

The `run_player_physics()` method is the most complete system. Here is the exact flow:

```
1. Read TickInput (action_flags, mouse_yaw, mouse_pitch)
2. Clone PlayerPhysicsParams and MapGeometry resources
3. Query player entity: Position, Velocity, Facing, VerticalLook,
   AngularVelocity, PolygonIndex, Grounded
4. compute_player_velocity()
   - Velocity stored in PLAYER-LOCAL frame (x=forward, y=perp, z=vert)
   - Forward and strafe axes decelerate independently
   - Direction reversals get acceleration+deceleration boost
   - Gravity applied when not grounded
5. compute_facing()
   - Keyboard turns use angular velocity (accel/decel/max)
   - Mouse yaw applied directly (1:1)
   - Both compose additively
6. compute_vertical_look()
   - Keyboard look uses fixed rate
   - Mouse pitch applied directly
   - Clamped to maximum_elevation
7. velocity_local_to_world()
   - Projects local (forward, perp, vert) into world (x, y, z) using facing
8. apply_player_collision()
   - Tests movement against polygon adjacency lines
   - Solid lines block, slide along wall normal
   - Passable lines check step_delta and ceiling clearance
   - Step climbing raises Z when crossing to higher floor
   - Up to 3 iterations for multi-wall slides
   - find_polygon_for_point() updates polygon index
   - Z grounded to floor height
9. velocity_world_to_local()
   - Converts post-collision world velocity back to player-local frame
10. Write back all component values
```

## Entity Spawning

Entities are spawned from map objects during `SimWorld::new()`:

- **Player** (object_type == 3): First player spawn only. Gets full spatial, vitality, and collision components.
- **Monsters** (object_type == 0): Gets Monster marker, AI components (MonsterState::Idle, Target, AttackCooldown), spatial, vitality, collision, rendering hints. Flying monsters get the Flying component.
- **Items** (object_type == 2): Gets Item marker, position, collision radius (fixed 0.25), rendering hints.
- **Platforms**: Spawned from platform definitions with full state machine data.
- **Lights**: Spawned from static light definitions with function type and period.
- **Media**: Spawned from media definitions with type, height bounds, flow.

## SimEvent System

The simulation emits events for the integration layer to handle:

```rust
enum SimEvent {
    LevelTeleport { target_level },
    TerminalActivation { terminal_index },
    SoundTrigger { sound_index, position },
    EntityDamaged { entity, amount, damage_type },
    EntityKilled { entity },
}
```

Events are pushed to `SimEvents` resource during tick processing and drained by the integration layer after each tick via `sim_world.drain_events()`.
