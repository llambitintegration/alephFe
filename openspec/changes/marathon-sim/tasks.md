## 1. Crate Scaffolding

- [x] 1.1 Create `marathon-sim` crate with `Cargo.toml` (deps: marathon-formats, bevy_ecs, glam, serde, bincode, rand, thiserror)
- [x] 1.2 Add `marathon-sim` to workspace members in root `Cargo.toml`
- [x] 1.3 Create module structure: `lib.rs`, `world.rs`, `tick.rs`, `components.rs`, `collision.rs`, `player/`, `monster/`, `combat/`, `world_mechanics/`
- [x] 1.4 Define ECS components: `Position(Vec3)`, `Velocity(Vec3)`, `Facing(f32)`, `CollisionRadius(f32)`, `Health(i16)`, `Shield(i16)`, `Oxygen(i16)`, `PolygonIndex(usize)`, `Grounded(bool)`, `EntityType` marker components (Player, Monster, Projectile, Item, Effect)

## 2. World Construction

- [x] 2.1 Implement `SimWorld::new(map_data, physics_data, config)`: create bevy_ecs World, seed PRNG, store physics tables as resources
- [x] 2.2 Spawn player entity at the first player-start `MapObject` position with components from `PhysicsConstants`
- [x] 2.3 Spawn monster entities from `MapObject` entries: look up `MonsterDefinition` by type, attach AI state and combat components
- [x] 2.4 Spawn item entities from `MapObject` entries with item type and position
- [x] 2.5 Initialize platform state from `StaticPlatformData`: create platform entities with position, speed, resting/extended heights, activation type
- [x] 2.6 Initialize light state from `StaticLightData`: create light entities with function type, period, phase, intensity bounds
- [x] 2.7 Initialize media state from `MediaData` entries: create media entities with height bounds, associated light, media type, current direction
- [x] 2.8 Store map geometry (polygons, lines, endpoints) as a shared ECS resource for collision and pathfinding
- [ ] 2.9 Write tests for world construction from synthetic map/physics data

## 3. Collision Detection

- [x] 3.1 Implement point-in-polygon test using polygon vertex list
- [x] 3.2 Implement line-segment intersection for wall collision (ray vs line between two endpoints)
- [x] 3.3 Implement polygon adjacency traversal: given a position and current polygon, find which adjacent polygon the position is in
- [x] 3.4 Implement entity-vs-wall collision: move entity, detect wall crossings, compute slide response
- [x] 3.5 Implement entity-vs-entity radius overlap test
- [x] 3.6 Implement line-of-sight check via polygon adjacency graph (ray trace checking for solid line crossings)
- [x] 3.7 Write tests for collision primitives with known geometric cases

## 4. Player Physics

- [x] 4.1 Implement input processing system: convert `ActionFlags` into player intent (acceleration direction, turn rate, action triggers)
- [x] 4.2 Implement player movement system: apply acceleration from input, deceleration when no input, clamp to max velocities from `PhysicsConstants`
- [x] 4.3 Implement player gravity system: apply gravitational acceleration when airborne, zero vertical velocity on floor contact
- [ ] 4.4 Implement player-wall collision response: detect wall crossings per tick, slide along walls, update polygon index
- [ ] 4.5 Implement step climbing: allow crossing lines where floor height difference <= `step_delta`
- [ ] 4.6 Implement ceiling collision: block entry to polygons where ceiling-floor gap < player height
- [ ] 4.7 Implement media submersion: detect when player is in liquid, apply drag, deplete oxygen, apply drowning damage
- [x] 4.8 Implement player facing: apply angular velocity from turn input, clamp vertical look angle to `maximum_elevation`
- [x] 4.9 Write tests for player movement, gravity, wall collision, step climbing, and media effects

## 5. Monster AI

- [x] 5.1 Define `MonsterState` component enum: Idle, Alerted, Attacking, Moving, Fleeing, Dying, Dead
- [x] 5.2 Implement monster activation system: check line-of-sight to player for idle monsters within visual range/arc
- [ ] 5.3 Implement activation cascading: propagate alerts to nearby monsters of same class
- [x] 5.4 Implement target acquisition system: select target based on enemies bitmask, distance, and line-of-sight
- [ ] 5.5 Implement friendly-fire response: redirect target when damaged by a friendly entity
- [x] 5.6 Implement monster movement system: pathfind via polygon adjacency toward target, respect speed and collision
- [ ] 5.7 Implement flying monster movement: 3D movement toward target at preferred hover height
- [ ] 5.8 Implement monster gravity: apply gravity to non-flying monsters, track floor height
- [ ] 5.9 Implement monster attack system: execute melee attacks (direct damage) and ranged attacks (spawn projectile) at defined frequency
- [x] 5.10 Implement monster death: transition to Dying on zero vitality, play death animation, transition to Dead corpse
- [x] 5.11 Write tests for AI state transitions, line-of-sight, pathfinding, and attack execution

## 6. Combat System

- [x] 6.1 Implement weapon state machine: idle, firing, recovering, reloading, switching states with timing from `WeaponDefinition`
- [x] 6.2 Implement weapon firing: consume ammo, spawn projectile at weapon offset with trigger's projectile type and theta error
- [ ] 6.3 Implement burst fire: spawn `burst_count` projectiles per trigger pull with spread
- [ ] 6.4 Implement dual-wielded weapons: independent left/right firing for twofisted_pistol class
- [x] 6.5 Implement ammunition management: magazine tracking, auto-reload from reserves, reserve additions from pickups
- [x] 6.6 Implement weapon switching: cycle through inventory, apply `ready_ticks` delay
- [x] 6.7 Implement projectile movement system: advance position by velocity each tick, apply gravity if flagged
- [ ] 6.8 Implement projectile-wall collision: detect wall crossing, detonate at intersection
- [ ] 6.9 Implement projectile-entity collision: check path against entity radii, apply damage on hit
- [x] 6.10 Implement homing projectile tracking: adjust velocity toward nearest target each tick
- [ ] 6.11 Implement projectile range limit: despawn projectiles that exceed `maximum_range`
- [x] 6.12 Implement area-of-effect damage: on detonation, damage all entities within `area_of_effect` radius scaled by distance
- [x] 6.13 Implement damage calculation: base + random(0, random), apply scale, check immunities (zero damage) and weaknesses (double damage)
- [ ] 6.14 Implement damage application: subtract from health/shield, emit damage events, handle entity death
- [x] 6.15 Write tests for weapon firing, projectile lifecycle, damage calculation with immunities/weaknesses, and AoE

## 7. World Mechanics

- [x] 7.1 Implement platform movement system: move floor/ceiling toward target position at defined speed each tick
- [ ] 7.2 Implement platform activation: player standing, action key, monster entry, and projectile impact triggers
- [x] 7.3 Implement platform return: delay timer, then move back to resting position
- [ ] 7.4 Implement platform crushing: damage entities caught between floor and ceiling, or reverse if no crush flag
- [ ] 7.5 Implement platform triggers: activate linked platforms and toggle linked lights on position reached
- [x] 7.6 Implement light animation system: compute intensity from function type (constant, linear, smooth, flicker), period, and phase each tick
- [x] 7.7 Implement media height system: derive liquid height from associated light intensity, interpolated between low and high bounds
- [x] 7.8 Implement media effects: apply current flow forces to entities in liquid, apply environmental damage for lava/goo
- [x] 7.9 Implement item pickup system: detect player-item collision, apply item effect (health, shield, ammo, weapon, oxygen, inventory), remove item entity
- [ ] 7.10 Implement item respawn: timer-based respawn in multiplayer modes
- [ ] 7.11 Implement control panel activation: detect action key + facing a panel side, trigger linked platform/light/terminal
- [x] 7.12 Write tests for platform movement, light animation, media height, item pickup, and panel activation

## 8. Public API and Events

- [x] 8.1 Implement `SimWorld::tick(action_flags)`: run all systems in order (input, player physics, monster AI, combat, projectiles, damage, world mechanics, cleanup)
- [x] 8.2 Implement player state accessors: position, facing, health, shield, oxygen, weapon, ammo, inventory, polygon index
- [ ] 8.3 Implement entity state iterator: return all active entities with position, facing, shape, and frame for rendering
- [x] 8.4 Implement event queue: level teleport, terminal activation, sound triggers, damage events
- [ ] 8.5 Implement `SimWorld` serde serialization and deserialization for save/load
- [ ] 8.6 Write tests for deterministic replay (two worlds with same seed produce identical state)

## 9. Integration Testing

- [ ] 9.1 Create integration test: construct SimWorld from synthetic map data, advance 100 ticks with scripted input, verify player position
- [ ] 9.2 Create integration test: spawn monster facing player, advance ticks, verify monster becomes alerted and attacks
- [ ] 9.3 Create integration test: fire weapon, verify projectile spawns and hits wall, verify detonation
- [ ] 9.4 Create integration test: activate platform, verify movement over ticks, verify platform trigger cascade
- [ ] 9.5 Create integration test: pick up items, verify inventory changes, verify weapon switching
