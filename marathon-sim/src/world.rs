use bevy_ecs::prelude::*;
use glam::{Vec2, Vec3};
use marathon_formats::map::LightData;
use marathon_formats::physics::PhysicsData;
use marathon_formats::MapData;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::components::*;

/// Configuration for creating a simulation world.
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub random_seed: u64,
    pub difficulty: u8,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            random_seed: 0,
            difficulty: 2, // Normal
        }
    }
}

/// Shared map geometry resource for collision and pathfinding.
#[derive(Resource, Debug, Clone)]
pub struct MapGeometry {
    /// Polygon vertex positions (2D, in world units converted to f32).
    pub polygon_vertices: Vec<Vec<Vec2>>,
    /// Floor heights per polygon.
    pub floor_heights: Vec<f32>,
    /// Ceiling heights per polygon.
    pub ceiling_heights: Vec<f32>,
    /// Polygon adjacency: for each polygon, list of (line_index, adjacent_polygon_or_none).
    pub polygon_adjacency: Vec<Vec<(usize, Option<usize>)>>,
    /// Line endpoint positions.
    pub line_endpoints: Vec<(Vec2, Vec2)>,
    /// Whether each line is solid (blocks movement and LOS).
    pub line_solid: Vec<bool>,
    /// Whether each line is transparent (allows LOS but may block movement).
    pub line_transparent: Vec<bool>,
    /// Per-polygon media index (-1 if none).
    pub polygon_media_index: Vec<i16>,
    /// Per-polygon floor light-source index (-1 if none).
    pub polygon_floor_light_index: Vec<i16>,
    /// Per-polygon ceiling light-source index (-1 if none).
    pub polygon_ceiling_light_index: Vec<i16>,
    /// Polygon type (e.g. 5 = platform) per polygon.
    pub polygon_types: Vec<i16>,
    /// Polygon permutation (e.g. platform index) per polygon.
    pub polygon_permutations: Vec<i16>,
    /// Side indices per line: (clockwise_side, counterclockwise_side).
    pub line_side_indices: Vec<(Option<usize>, Option<usize>)>,
}

/// Physics tables resource (monster defs, weapon defs, etc.).
#[derive(Resource, Debug)]
pub struct PhysicsTables {
    pub data: PhysicsData,
}

/// Deterministic PRNG resource.
#[derive(Resource)]
pub struct SimRng(pub StdRng);

/// Current simulation tick counter.
#[derive(Resource, Debug, Default)]
pub struct TickCounter(pub u64);

/// Events emitted by the simulation for the integration layer to handle.
#[derive(Resource, Debug, Default)]
pub struct SimEvents {
    pub events: Vec<SimEvent>,
}

/// A simulation event.
#[derive(Debug, Clone)]
pub enum SimEvent {
    LevelTeleport {
        target_level: usize,
    },
    TerminalActivation {
        terminal_index: usize,
    },
    SoundTrigger {
        sound_index: usize,
        position: Vec3,
    },
    EntityDamaged {
        entity: Entity,
        amount: i16,
        damage_type: i16,
    },
    EntityKilled {
        entity: Entity,
    },
    ItemPickedUp {
        item_type: i16,
    },
}

impl SimEvents {
    pub fn push(&mut self, event: SimEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }
}

/// The top-level simulation world.
///
/// Wraps a bevy_ecs `World` and provides a high-level API for
/// constructing, advancing, and querying the simulation.
pub struct SimWorld {
    pub(crate) world: World,
}

/// Convert a Marathon world coordinate (i16, 1024 = 1 WU) to f32.
fn world_coord(v: i16) -> f32 {
    v as f32 / 1024.0
}

impl SimWorld {
    /// Create a new simulation world from map and physics data.
    pub fn new(
        map_data: &MapData,
        physics_data: &PhysicsData,
        config: &SimConfig,
    ) -> Result<Self, SimWorldError> {
        let mut world = World::new();

        // Insert resources
        world.insert_resource(SimRng(StdRng::seed_from_u64(config.random_seed)));
        world.insert_resource(TickCounter(0));
        world.insert_resource(crate::tick::PrevActionKey::default());
        world.insert_resource(SimEvents::default());
        world.insert_resource(PhysicsTables {
            data: physics_data.clone(),
        });

        // Build map geometry resource
        let geometry = build_map_geometry(map_data);
        world.insert_resource(geometry);

        // Store player physics params as a resource.
        // Marathon physics data has two entries: index 0 = walking, index 1 = running.
        // Prefer running for default movement feel, fall back to walking if only one exists.
        if let Some(pc) = physics_data
            .physics
            .as_ref()
            .and_then(|p| p.get(1).or_else(|| p.first()))
        {
            world.insert_resource(
                crate::player::movement::PlayerPhysicsParams::from_physics_constants(pc),
            );
        }

        // Spawn entities from map objects
        spawn_map_objects(&mut world, map_data, physics_data, config)?;

        // Initialize platforms
        spawn_platforms(&mut world, map_data);

        // Initialize lights
        spawn_lights(&mut world, map_data);

        // Initialize media
        spawn_media(&mut world, map_data);

        // Initialize control panels from map sides
        let control_panels = build_control_panels(map_data);
        world.insert_resource(control_panels);

        // Initialize player weapon inventory (start with fists + magnum).
        //
        // Marathon's canonical starting loadout is fists (weapon definition
        // index 0, melee/infinite ammo) plus the .44 magnum pistol (definition
        // index 1), with the magnum equipped. Ammo counts are sourced from the
        // scenario's physics data so the loadout stays scenario-correct rather
        // than hard-coded. If a scenario provides no index-1 weapon we fall back
        // gracefully to a fists-only, fists-equipped inventory.
        let mut weapon_inventory = crate::player::inventory::WeaponInventory::default();
        {
            use crate::player::inventory::{WeaponSlot, WeaponState};

            let num_weapon_slots = physics_data.weapons.as_ref().map_or(0, |w| w.len());
            weapon_inventory.weapons = vec![None; num_weapon_slots.max(1)];

            // Fists: weapon definition index 0, infinite ammo (melee).
            weapon_inventory.weapons[0] = Some(WeaponSlot {
                definition_index: 0,
                primary_magazine: u16::MAX,
                primary_reserve: 0,
                secondary_magazine: 0,
                secondary_reserve: 0,
                state: WeaponState::Idle,
                cooldown_ticks: 0,
            });
            weapon_inventory.current_weapon = 0;

            // Magnum: weapon definition index 1. Insert and equip it when the
            // scenario actually defines it. Magazine is a full magazine and the
            // reserve is two spare magazines, both derived from the weapon's
            // own primary-trigger `rounds_per_magazine` (no magic literals).
            const MAGNUM_INDEX: usize = 1;
            if let Some(magnum_def) = physics_data
                .weapons
                .as_ref()
                .and_then(|w| w.get(MAGNUM_INDEX))
            {
                let rounds_per_magazine =
                    magnum_def.primary_trigger.rounds_per_magazine.max(0) as u16;
                weapon_inventory.weapons[MAGNUM_INDEX] = Some(WeaponSlot {
                    definition_index: MAGNUM_INDEX,
                    primary_magazine: rounds_per_magazine,
                    // Two spare magazines of reserve ammo.
                    primary_reserve: rounds_per_magazine.saturating_mul(2),
                    secondary_magazine: 0,
                    secondary_reserve: 0,
                    state: WeaponState::Idle,
                    cooldown_ticks: 0,
                });
                weapon_inventory.current_weapon = MAGNUM_INDEX;
            }
        }
        world.insert_resource(weapon_inventory);

        Ok(Self { world })
    }

    /// Get the current tick count.
    pub fn tick_count(&self) -> u64 {
        self.world.resource::<TickCounter>().0
    }

    /// Current intensity of every light, as a `Vec` indexed by `light_index`
    /// (box 6.1). Renderers read this each frame to drive per-polygon floor /
    /// ceiling lighting. Indices with no spawned light default to `1.0` (full
    /// bright), matching the renderer's no-light fallback.
    pub fn light_intensities(&mut self) -> Vec<f32> {
        let mut q = self.world.query::<&crate::components::Light>();
        let pairs: Vec<(usize, f32)> = q
            .iter(&self.world)
            .map(|l| (l.light_index, l.current_intensity))
            .collect();
        let len = pairs.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
        let mut out = vec![1.0; len];
        for (i, v) in pairs {
            out[i] = v;
        }
        out
    }

    /// Current surface height of every media, as a `Vec` indexed by media index
    /// (box 6.2). Renderers read this each frame to drive liquid surface
    /// heights. Indices with no spawned media default to `0.0`.
    pub fn media_heights(&mut self) -> Vec<f32> {
        let mut q = self.world.query::<&crate::components::Media>();
        let pairs: Vec<(usize, f32)> = q
            .iter(&self.world)
            .map(|m| (m.index, m.current_height))
            .collect();
        let len = pairs.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
        let mut out = vec![0.0; len];
        for (i, v) in pairs {
            out[i] = v;
        }
        out
    }

    /// Drain pending simulation events.
    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        self.world.resource_mut::<SimEvents>().drain()
    }

    /// Expose the inner ECS world for direct entity manipulation (primarily for tests).
    pub fn ecs_world_mut(&mut self) -> &mut bevy_ecs::world::World {
        &mut self.world
    }

    /// Return current per-polygon dynamic geometry/lighting data for every
    /// polygon in the level, indexed by polygon.
    ///
    /// This is the sim-side source the web renderer feeds into its per-polygon
    /// data texture each frame (box 4.2). Heights are in render units (Marathon
    /// world units / 1024) so they match the renderer's X/Z scale; light values
    /// are 0.0..=1.0 intensity multipliers.
    ///
    /// Field sources:
    /// - `floor_height` / `ceiling_height`: live values from `MapGeometry`,
    ///   which `update_platforms` rewrites each tick as platforms/doors move.
    /// - `media_height`: the current surface height of the `Media` referenced by
    ///   the polygon's `media_index` (animated by `update_media`); `0.0` when the
    ///   polygon has no media.
    /// - `floor_light` / `ceiling_light`: the current intensity of the `Light`
    ///   referenced by the polygon's floor/ceiling light-source index (animated
    ///   by `update_lights`); `1.0` when the polygon references no valid light
    ///   (mirrors the web `evaluate_light_intensity` fallback).
    pub fn poly_dynamic_data(&mut self) -> Vec<PolyDynamicData> {
        let geometry = self.world.resource::<MapGeometry>();
        let floor_heights = geometry.floor_heights.clone();
        let ceiling_heights = geometry.ceiling_heights.clone();
        let polygon_media_index = geometry.polygon_media_index.clone();
        let polygon_floor_light_index = geometry.polygon_floor_light_index.clone();
        let polygon_ceiling_light_index = geometry.polygon_ceiling_light_index.clone();

        // Media current surface height keyed by media array index.
        let media_heights: std::collections::HashMap<usize, f32> = {
            let mut map = std::collections::HashMap::new();
            let mut q = self.world.query::<&crate::components::Media>();
            for media in q.iter(&self.world) {
                map.insert(media.index, media.current_height);
            }
            map
        };

        // Current light intensity keyed by light array index.
        let light_intensities: std::collections::HashMap<usize, f32> = {
            let mut map = std::collections::HashMap::new();
            let mut q = self.world.query::<&crate::components::Light>();
            for light in q.iter(&self.world) {
                map.insert(light.light_index, light.current_intensity);
            }
            map
        };

        let light_for = |idx: i16| -> f32 {
            if idx < 0 {
                return 1.0;
            }
            light_intensities
                .get(&(idx as usize))
                .copied()
                .unwrap_or(1.0)
        };

        let poly_count = floor_heights.len();
        (0..poly_count)
            .map(|p| {
                let media_height = {
                    let mi = polygon_media_index.get(p).copied().unwrap_or(-1);
                    if mi >= 0 {
                        media_heights.get(&(mi as usize)).copied().unwrap_or(0.0)
                    } else {
                        0.0
                    }
                };
                PolyDynamicData {
                    floor_height: floor_heights[p],
                    ceiling_height: ceiling_heights.get(p).copied().unwrap_or(0.0),
                    media_height,
                    floor_light: light_for(polygon_floor_light_index.get(p).copied().unwrap_or(-1)),
                    ceiling_light: light_for(
                        polygon_ceiling_light_index.get(p).copied().unwrap_or(-1),
                    ),
                }
            })
            .collect()
    }
}

/// Current per-polygon dynamic geometry/lighting state, indexed by polygon.
///
/// Heights are in render units (Marathon world units / 1024); light values are
/// 0.0..=1.0 intensity multipliers. This is the sim-side equivalent of the web
/// renderer's `PolyDynData`; the web layer maps one to the other so the sim
/// crate stays free of any web dependency.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PolyDynamicData {
    pub floor_height: f32,
    pub ceiling_height: f32,
    pub media_height: f32,
    pub floor_light: f32,
    pub ceiling_light: f32,
}

fn build_map_geometry(map_data: &MapData) -> MapGeometry {
    let endpoints: Vec<Vec2> = map_data
        .endpoints
        .iter()
        .map(|ep| Vec2::new(world_coord(ep.vertex.x), world_coord(ep.vertex.y)))
        .collect();

    let line_endpoints: Vec<(Vec2, Vec2)> = map_data
        .lines
        .iter()
        .map(|line| {
            let a = endpoints[line.endpoint_indexes[0] as usize];
            let b = endpoints[line.endpoint_indexes[1] as usize];
            (a, b)
        })
        .collect();

    const LINE_SOLID: u16 = 0x4000;
    const LINE_HAS_TRANSPARENT_SIDE: u16 = 0x0200;

    let line_solid: Vec<bool> = map_data
        .lines
        .iter()
        .map(|line| line.flags & LINE_SOLID != 0)
        .collect();

    let line_transparent: Vec<bool> = map_data
        .lines
        .iter()
        .map(|line| line.flags & LINE_HAS_TRANSPARENT_SIDE != 0)
        .collect();

    let polygon_vertices: Vec<Vec<Vec2>> = map_data
        .polygons
        .iter()
        .map(|poly| {
            let count = poly.vertex_count as usize;
            (0..count)
                .map(|i| endpoints[poly.endpoint_indexes[i] as usize])
                .collect()
        })
        .collect();

    let floor_heights: Vec<f32> = map_data
        .polygons
        .iter()
        .map(|poly| world_coord(poly.floor_height))
        .collect();

    let ceiling_heights: Vec<f32> = map_data
        .polygons
        .iter()
        .map(|poly| world_coord(poly.ceiling_height))
        .collect();

    let polygon_adjacency: Vec<Vec<(usize, Option<usize>)>> = map_data
        .polygons
        .iter()
        .map(|poly| {
            let count = poly.vertex_count as usize;
            (0..count)
                .map(|i| {
                    let line_idx = poly.line_indexes[i] as usize;
                    let adj = poly.adjacent_polygon_indexes[i];
                    let adj_opt = if adj < 0 { None } else { Some(adj as usize) };
                    (line_idx, adj_opt)
                })
                .collect()
        })
        .collect();

    let polygon_media_index: Vec<i16> = map_data
        .polygons
        .iter()
        .map(|poly| poly.media_index)
        .collect();

    let polygon_floor_light_index: Vec<i16> = map_data
        .polygons
        .iter()
        .map(|poly| poly.floor_lightsource_index)
        .collect();

    let polygon_ceiling_light_index: Vec<i16> = map_data
        .polygons
        .iter()
        .map(|poly| poly.ceiling_lightsource_index)
        .collect();

    let polygon_types: Vec<i16> = map_data
        .polygons
        .iter()
        .map(|poly| poly.polygon_type)
        .collect();

    let polygon_permutations: Vec<i16> = map_data
        .polygons
        .iter()
        .map(|poly| poly.permutation)
        .collect();

    let line_side_indices: Vec<(Option<usize>, Option<usize>)> = map_data
        .lines
        .iter()
        .map(|line| {
            let cw = if line.clockwise_polygon_side_index >= 0 {
                Some(line.clockwise_polygon_side_index as usize)
            } else {
                None
            };
            let ccw = if line.counterclockwise_polygon_side_index >= 0 {
                Some(line.counterclockwise_polygon_side_index as usize)
            } else {
                None
            };
            (cw, ccw)
        })
        .collect();

    MapGeometry {
        polygon_vertices,
        floor_heights,
        ceiling_heights,
        polygon_adjacency,
        line_endpoints,
        line_solid,
        line_transparent,
        polygon_media_index,
        polygon_floor_light_index,
        polygon_ceiling_light_index,
        polygon_types,
        polygon_permutations,
        line_side_indices,
    }
}

/// Object type constants from Marathon's map format.
const OBJECT_IS_MONSTER: i16 = 0;
const OBJECT_IS_ITEM: i16 = 2;
const OBJECT_IS_PLAYER: i16 = 3;

fn spawn_map_objects(
    world: &mut World,
    map_data: &MapData,
    physics_data: &PhysicsData,
    _config: &SimConfig,
) -> Result<(), SimWorldError> {
    let mut player_spawned = false;
    let geometry = world.resource::<MapGeometry>();
    let floor_heights = geometry.floor_heights.clone();

    for obj in &map_data.objects {
        let polygon = obj.polygon_index as usize;
        let raw_z = world_coord(obj.location.z);
        // Clamp spawn Z: at least floor height so entities don't spawn below the floor
        let floor_z = floor_heights.get(polygon).copied().unwrap_or(0.0);
        let z = raw_z.max(floor_z);
        let pos = Vec3::new(world_coord(obj.location.x), world_coord(obj.location.y), z);
        let facing = (obj.facing as f32) * (std::f32::consts::TAU / 512.0);

        match obj.object_type {
            OBJECT_IS_PLAYER if !player_spawned => {
                let physics = physics_data
                    .physics
                    .as_ref()
                    .and_then(|p| p.first())
                    .ok_or(SimWorldError::MissingPhysicsData(
                        "player physics constants".into(),
                    ))?;

                world.spawn((
                    Player,
                    Position(pos),
                    Velocity::default(),
                    Facing(facing),
                    VerticalLook::default(),
                    AngularVelocity::default(),
                    CollisionRadius(physics.radius),
                    EntityHeight(physics.height),
                    Health(150),
                    Shield(150),
                    Oxygen(600),
                    PolygonIndex(polygon),
                    Grounded(true),
                ));
                player_spawned = true;
            }
            OBJECT_IS_MONSTER => {
                let def_index = obj.index as usize;
                let monster_def = physics_data
                    .monsters
                    .as_ref()
                    .and_then(|m| m.get(def_index));

                if let Some(def) = monster_def {
                    let radius = world_coord(def.radius);
                    let height = world_coord(def.height);
                    let is_flying = def.flags & 0x0002 != 0;

                    let mut entity = world.spawn((
                        Monster {
                            definition_index: def_index,
                        },
                        MonsterState::Idle,
                        Target::default(),
                        AttackCooldown::default(),
                        Position(pos),
                        Velocity::default(),
                        Facing(facing),
                        CollisionRadius(radius),
                        EntityHeight(height),
                        Health(def.vitality),
                        Immunities(def.immunities),
                        Weaknesses(def.weaknesses),
                        PolygonIndex(polygon),
                        Grounded(!is_flying),
                    ));
                    entity.insert((SpriteShape(def.stationary_shape), AnimationFrame::default()));

                    if is_flying {
                        entity.insert(Flying {
                            preferred_hover_height: world_coord(def.preferred_hover_height),
                        });
                    }
                }
            }
            OBJECT_IS_ITEM => {
                world.spawn((
                    Item {
                        item_type: obj.index,
                    },
                    Position(pos),
                    CollisionRadius(0.25),
                    PolygonIndex(polygon),
                    SpriteShape(0),
                    AnimationFrame::default(),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

fn spawn_platforms(world: &mut World, map_data: &MapData) {
    use crate::world_mechanics::platforms::*;

    // Track which polygon indices have explicit platform data
    let mut explicit_polys = std::collections::HashSet::new();

    // First pass: explicit PLAT entries
    for platform in &map_data.platforms {
        let poly_idx = platform.polygon_index as usize;
        explicit_polys.insert(poly_idx);
        let speed = world_coord(platform.speed);
        let min_height = world_coord(platform.minimum_height);
        let max_height = world_coord(platform.maximum_height);

        let poly_ceiling = if poly_idx < map_data.polygons.len() {
            world_coord(map_data.polygons[poly_idx].ceiling_height)
        } else {
            2.0
        };

        world.spawn(Platform {
            polygon_index: poly_idx,
            floor_rest: min_height,
            floor_extended: max_height,
            ceiling_rest: poly_ceiling,
            ceiling_extended: poly_ceiling,
            current_floor: min_height,
            current_ceiling: poly_ceiling,
            speed,
            state: PlatformState::AtRest,
            return_delay: platform.delay as u16,
            delay_remaining: 0,
            activation_flags: platform.static_flags,
            crushes: platform.static_flags & (1 << 8) != 0,
        });
    }

    // Second pass: implicit platforms from polygon_type == 5 without explicit PLAT data.
    // Marathon creates these with defaults for _platform_is_spht_split_door (type 1).
    const POLYGON_IS_PLATFORM: i16 = 5;
    // Default flags for _platform_is_spht_split_door:
    // deactivates_at_initial_level | extends_floor_to_ceiling | player_controllable |
    // monster_controllable | reverses_when_obstructed | comes_from_floor |
    // comes_from_ceiling | initially_extended | is_door
    let default_flags: u32 = PLATFORM_DEACTIVATES_AT_INITIAL_LEVEL
        | (1 << 5) // extends_floor_to_ceiling
        | PLATFORM_IS_PLAYER_CONTROLLABLE
        | PLATFORM_IS_MONSTER_CONTROLLABLE
        | PLATFORM_REVERSES_DIRECTION_WHEN_OBSTRUCTED
        | PLATFORM_COMES_FROM_FLOOR
        | PLATFORM_COMES_FROM_CEILING
        | PLATFORM_IS_INITIALLY_EXTENDED
        | PLATFORM_IS_DOOR;
    // Default speed: _slow_platform = WORLD_ONE / (2 * 30) ≈ 0.0167 WU/tick
    let default_speed: f32 = 1.0 / 60.0;
    // Default delay: _very_long_delay_platform = 4 * 30 = 120 ticks
    let default_delay: u16 = 120;

    for (poly_idx, polygon) in map_data.polygons.iter().enumerate() {
        if polygon.polygon_type == POLYGON_IS_PLATFORM && !explicit_polys.contains(&poly_idx) {
            let floor = world_coord(polygon.floor_height);
            let ceiling = world_coord(polygon.ceiling_height);

            // Initially extended (closed door): floor at ceiling level
            // Rest position (open): floor at floor level
            world.spawn(Platform {
                polygon_index: poly_idx,
                floor_rest: floor,
                floor_extended: ceiling, // floor rises to ceiling when closed
                ceiling_rest: ceiling,
                ceiling_extended: floor, // ceiling lowers to floor when closed
                current_floor: ceiling,  // starts extended (closed)
                current_ceiling: floor,  // starts extended (closed)
                speed: default_speed,
                state: PlatformState::AtRest,
                return_delay: default_delay,
                delay_remaining: 0,
                activation_flags: default_flags,
                crushes: false,
            });
        }
    }
}

fn spawn_lights(world: &mut World, map_data: &MapData) {
    let lights = match &map_data.lights {
        LightData::Static(lights) => lights.clone(),
        _ => return,
    };

    fn map_function(f: i16) -> LightFunction {
        match f {
            1 => LightFunction::Linear,
            2 => LightFunction::Smooth,
            3 => LightFunction::Flicker,
            4 => LightFunction::Random,
            5 => LightFunction::Fluorescent,
            _ => LightFunction::Constant,
        }
    }
    fn to_spec(s: &marathon_formats::map::LightingFunctionSpec) -> LightFunctionSpec {
        LightFunctionSpec {
            function: map_function(s.function),
            period: s.period.max(0) as u16,
            delta_period: s.delta_period.max(0) as u16,
            intensity: s.intensity,
            delta_intensity: s.delta_intensity,
        }
    }
    fn map_light_type(t: i16) -> LightType {
        match t {
            1 => LightType::Strobe,
            2 => LightType::Media,
            _ => LightType::Normal,
        }
    }

    world.resource_scope(|world: &mut World, mut sim_rng: Mut<SimRng>| {
        for (idx, light) in lights.iter().enumerate() {
            // functions[] is indexed by LightState::as_index (cycle order).
            let functions = [
                to_spec(&light.becoming_active),
                to_spec(&light.primary_active),
                to_spec(&light.secondary_active),
                to_spec(&light.becoming_inactive),
                to_spec(&light.primary_inactive),
                to_spec(&light.secondary_inactive),
            ];
            let initially_active = light.flags & LIGHT_IS_INITIALLY_ACTIVE != 0;
            let state = if initially_active {
                LightState::BecomingActive
            } else {
                LightState::BecomingInactive
            };
            // Alephone defaults: the activation ramp starts dark (0.0), the
            // deactivation ramp starts lit (1.0).
            let initial_intensity = if initially_active { 0.0 } else { 1.0 };
            // Roll the starting state's period + target intensity (delta randomized).
            let start = functions[state.as_index()];
            let span = start.delta_period as u32 + 1;
            let period = (start.period as u32 + sim_rng.0.gen_range(0..span)).max(1);
            let final_intensity = start.intensity + sim_rng.0.gen::<f32>() * start.delta_intensity;

            world.spawn(Light {
                light_index: idx,
                light_type: map_light_type(light.light_type),
                state,
                flags: light.flags,
                phase: light.phase.max(0) as u32,
                period,
                current_intensity: initial_intensity,
                initial_intensity,
                final_intensity,
                functions,
                tag: light.tag,
            });
        }
    });
}

fn spawn_media(world: &mut World, map_data: &MapData) {
    for (idx, media) in map_data.media.iter().enumerate() {
        world.spawn(Media {
            index: idx,
            polygon_index: 0,
            media_type: media.media_type,
            height_low: world_coord(media.low),
            height_high: world_coord(media.high),
            light_index: media.light_index as usize,
            current_height: world_coord(media.high),
            current_direction: media.current_direction as f32 * (std::f32::consts::TAU / 512.0),
            current_magnitude: world_coord(media.current_magnitude),
        });
    }
}

fn build_control_panels(map_data: &MapData) -> crate::world_mechanics::panels::ControlPanels {
    use crate::world_mechanics::panels::{ControlPanel, ControlPanels, PanelAction};

    let mut panels = Vec::new();

    for (side_idx, side) in map_data.sides.iter().enumerate() {
        // Check IS_CONTROL_PANEL flag (0x0002)
        if side.flags & 0x0002 == 0 {
            continue;
        }
        if side.control_panel_type < 0 {
            continue;
        }

        let permutation = side.control_panel_permutation as usize;
        let action = match side.control_panel_type {
            4 => PanelAction::ToggleLight {
                light_index: permutation,
            },
            5 => PanelAction::ActivatePlatform {
                platform_index: permutation,
            },
            6 => PanelAction::ActivateTaggedPlatforms {
                tag: side.control_panel_permutation,
            },
            9 => PanelAction::ActivateTerminal {
                terminal_index: permutation,
            },
            _ => continue,
        };

        let line_index = side.line_index as usize;
        let side_num = if line_index < map_data.lines.len() {
            let line = &map_data.lines[line_index];
            if line.clockwise_polygon_side_index >= 0
                && line.clockwise_polygon_side_index as usize == side_idx
            {
                0
            } else {
                1
            }
        } else {
            0
        };

        panels.push(ControlPanel {
            line_index,
            side: side_num,
            action,
            max_distance: 1.5,
        });
    }

    ControlPanels(panels)
}

/// Serializable snapshot of the simulation state for save/load.
#[derive(Debug, Serialize, Deserialize)]
pub struct SimSnapshot {
    pub tick_count: u64,
    /// We store the original seed so we can recreate RNG state by
    /// fast-forwarding. For save/load, the tick_count tells us how many
    /// draws were made. In practice, we re-seed from a combined value.
    pub rng_seed: u64,
    pub player: Option<PlayerSnapshot>,
    pub monsters: Vec<MonsterSnapshot>,
    pub projectiles: Vec<ProjectileSnapshot>,
    pub items: Vec<ItemSnapshot>,
    pub platforms: Vec<crate::components::Platform>,
    pub lights: Vec<crate::components::Light>,
    pub media: Vec<crate::components::Media>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub position: Vec3,
    pub velocity: Vec3,
    pub facing: f32,
    pub vertical_look: f32,
    pub health: i16,
    pub shield: i16,
    pub oxygen: i16,
    pub polygon_index: usize,
    pub grounded: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonsterSnapshot {
    pub definition_index: usize,
    pub state: crate::components::MonsterState,
    pub position: Vec3,
    pub velocity: Vec3,
    pub facing: f32,
    pub health: i16,
    pub polygon_index: usize,
    pub attack_cooldown: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    pub definition_index: usize,
    pub position: Vec3,
    pub velocity: Vec3,
    pub distance_traveled: f32,
    pub ticks_alive: u16,
    pub contrails_spawned: u16,
    pub current_polygon: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemSnapshot {
    pub item_type: i16,
    pub position: Vec3,
    pub polygon_index: usize,
}

use serde::{Deserialize, Serialize};

impl SimWorld {
    /// Create a serializable snapshot of the current simulation state.
    pub fn snapshot(&mut self) -> SimSnapshot {
        // Player
        let player = {
            let mut q = self.world.query_filtered::<(
                &Position,
                &Velocity,
                &Facing,
                &crate::components::VerticalLook,
                &Health,
                &Shield,
                &Oxygen,
                &PolygonIndex,
                &Grounded,
            ), bevy_ecs::prelude::With<Player>>();
            q.iter(&self.world)
                .next()
                .map(
                    |(pos, vel, fac, vlook, hp, sh, ox, poly, gr)| PlayerSnapshot {
                        position: pos.0,
                        velocity: vel.0,
                        facing: fac.0,
                        vertical_look: vlook.0,
                        health: hp.0,
                        shield: sh.0,
                        oxygen: ox.0,
                        polygon_index: poly.0,
                        grounded: gr.0,
                    },
                )
        };

        // Monsters
        let monsters = {
            let mut q = self.world.query::<(
                &Monster,
                &crate::components::MonsterState,
                &Position,
                &Velocity,
                &Facing,
                &Health,
                &PolygonIndex,
                &AttackCooldown,
            )>();
            q.iter(&self.world)
                .map(|(m, state, pos, vel, fac, hp, poly, cd)| MonsterSnapshot {
                    definition_index: m.definition_index,
                    state: *state,
                    position: pos.0,
                    velocity: vel.0,
                    facing: fac.0,
                    health: hp.0,
                    polygon_index: poly.0,
                    attack_cooldown: cd.0,
                })
                .collect()
        };

        // Projectiles
        let projectiles = {
            let mut q = self.world.query::<(&Projectile, &Position, &Velocity)>();
            q.iter(&self.world)
                .map(|(p, pos, vel)| ProjectileSnapshot {
                    definition_index: p.definition_index,
                    position: pos.0,
                    velocity: vel.0,
                    distance_traveled: p.distance_traveled,
                    ticks_alive: p.ticks_alive,
                    contrails_spawned: p.contrails_spawned,
                    current_polygon: p.current_polygon,
                })
                .collect()
        };

        // Items
        let items = {
            let mut q = self.world.query::<(&Item, &Position, &PolygonIndex)>();
            q.iter(&self.world)
                .map(|(item, pos, poly)| ItemSnapshot {
                    item_type: item.item_type,
                    position: pos.0,
                    polygon_index: poly.0,
                })
                .collect()
        };

        // Platforms
        let platforms = {
            let mut q = self.world.query::<&crate::components::Platform>();
            q.iter(&self.world).cloned().collect()
        };

        // Lights
        let lights = {
            let mut q = self.world.query::<&crate::components::Light>();
            q.iter(&self.world).cloned().collect()
        };

        // Media
        let media_vec = {
            let mut q = self.world.query::<&crate::components::Media>();
            q.iter(&self.world).cloned().collect()
        };

        // For RNG, we store a combined seed derived from tick count
        // This allows recreating a usable (but not identical) RNG on load.
        let tick_count = self.world.resource::<TickCounter>().0;
        let rng_seed = tick_count.wrapping_mul(6364136223846793005).wrapping_add(1);

        SimSnapshot {
            tick_count,
            rng_seed,
            player,
            monsters,
            projectiles,
            items,
            platforms,
            lights,
            media: media_vec,
        }
    }

    /// Serialize the simulation state to bytes.
    pub fn serialize(&mut self) -> Result<Vec<u8>, bincode::Error> {
        let snapshot = self.snapshot();
        bincode::serialize(&snapshot)
    }

    /// Deserialize simulation state from bytes, requiring map/physics data to rebuild geometry.
    pub fn deserialize(
        data: &[u8],
        map_data: &MapData,
        physics_data: &PhysicsData,
    ) -> Result<Self, SimWorldError> {
        let snapshot: SimSnapshot = bincode::deserialize(data)
            .map_err(|e| SimWorldError::MissingPhysicsData(format!("deserialize error: {}", e)))?;

        let mut world = World::new();

        // Rebuild geometry and resources
        let geometry = build_map_geometry(map_data);
        world.insert_resource(geometry);
        world.insert_resource(PhysicsTables {
            data: physics_data.clone(),
        });
        world.insert_resource(TickCounter(snapshot.tick_count));
        world.insert_resource(crate::tick::PrevActionKey::default());
        world.insert_resource(SimEvents::default());

        // Restore RNG from seed
        world.insert_resource(SimRng(StdRng::seed_from_u64(snapshot.rng_seed)));

        // Rebuild control panels
        let control_panels = build_control_panels(map_data);
        world.insert_resource(control_panels);

        // Restore player
        if let Some(p) = snapshot.player {
            world.spawn((
                Player,
                Position(p.position),
                Velocity(p.velocity),
                Facing(p.facing),
                crate::components::VerticalLook(p.vertical_look),
                Health(p.health),
                Shield(p.shield),
                Oxygen(p.oxygen),
                PolygonIndex(p.polygon_index),
                Grounded(p.grounded),
                CollisionRadius(0.25),
                EntityHeight(0.8),
            ));
        }

        // Restore monsters
        for m in snapshot.monsters {
            world.spawn((
                Monster {
                    definition_index: m.definition_index,
                },
                m.state,
                crate::components::Target::default(),
                AttackCooldown(m.attack_cooldown),
                Position(m.position),
                Velocity(m.velocity),
                Facing(m.facing),
                Health(m.health),
                PolygonIndex(m.polygon_index),
                Grounded(true),
                CollisionRadius(0.25),
                EntityHeight(0.8),
                SpriteShape(0),
                AnimationFrame::default(),
            ));
        }

        // Restore projectiles
        for p in snapshot.projectiles {
            world.spawn((
                Projectile {
                    definition_index: p.definition_index,
                    distance_traveled: p.distance_traveled,
                    ticks_alive: p.ticks_alive,
                    contrails_spawned: p.contrails_spawned,
                    current_polygon: p.current_polygon,
                },
                Position(p.position),
                Velocity(p.velocity),
                PolygonIndex(p.current_polygon),
            ));
        }

        // Restore items
        for item in snapshot.items {
            world.spawn((
                Item {
                    item_type: item.item_type,
                },
                Position(item.position),
                PolygonIndex(item.polygon_index),
                CollisionRadius(0.25),
                SpriteShape(0),
                AnimationFrame::default(),
            ));
        }

        // Restore platforms, lights, media
        for platform in snapshot.platforms {
            world.spawn(platform);
        }
        for light in snapshot.lights {
            world.spawn(light);
        }
        for media in snapshot.media {
            world.spawn(media);
        }

        Ok(Self { world })
    }
}

/// Errors during simulation world construction.
#[derive(Debug, thiserror::Error)]
pub enum SimWorldError {
    #[error("Missing physics data: {0}")]
    MissingPhysicsData(String),
}

#[cfg(test)]
mod sim_event_tests {
    use super::*;

    #[test]
    fn item_picked_up_carries_item_type() {
        let event = SimEvent::ItemPickedUp { item_type: 7 };
        match event {
            SimEvent::ItemPickedUp { item_type } => assert_eq!(item_type, 7),
            _ => panic!("expected SimEvent::ItemPickedUp variant"),
        }
    }
}

#[cfg(test)]
mod poly_dynamic_data_tests {
    use super::*;
    use marathon_formats::map::{LightData, LightingFunctionSpec, StaticLightData};
    use marathon_formats::physics::PhysicsData;
    use marathon_formats::{Endpoint, Line, MapData, Polygon, ShapeDescriptor, WorldPoint2d};

    const POLYGON_IS_PLATFORM: i16 = 5;

    fn mk_endpoint(x: i16, y: i16, poly: i16) -> Endpoint {
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x, y },
            transformed: WorldPoint2d { x, y },
            supporting_polygon_index: poly,
        }
    }

    fn mk_line(a: i16, b: i16) -> Line {
        Line {
            endpoint_indexes: [a, b],
            flags: 0x4000, // SOLID
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: -1,
            counterclockwise_polygon_owner: -1,
        }
    }

    fn mk_square_polygon(
        polygon_type: i16,
        endpoint_indexes: [i16; 8],
        line_indexes: [i16; 8],
        floor_height: i16,
        ceiling_height: i16,
    ) -> Polygon {
        let wp_zero = WorldPoint2d { x: 0, y: 0 };
        Polygon {
            polygon_type,
            flags: 0,
            permutation: 0,
            vertex_count: 4,
            endpoint_indexes,
            line_indexes,
            floor_texture: ShapeDescriptor(0xFFFF),
            ceiling_texture: ShapeDescriptor(0xFFFF),
            floor_height,
            ceiling_height,
            floor_lightsource_index: 0,
            ceiling_lightsource_index: 0,
            area: 1024 * 1024,
            floor_transfer_mode: 0,
            ceiling_transfer_mode: 0,
            adjacent_polygon_indexes: [-1; 8],
            center: wp_zero,
            side_indexes: [-1; 8],
            floor_origin: wp_zero,
            ceiling_origin: wp_zero,
            media_index: -1,
            media_lightsource_index: -1,
            sound_source_indexes: -1,
            ambient_sound_image_index: -1,
            random_sound_image_index: -1,
        }
    }

    fn constant_light(intensity: f32) -> StaticLightData {
        let spec = LightingFunctionSpec {
            function: 0, // constant
            period: 1,
            delta_period: 0,
            intensity,
            delta_intensity: 0.0,
        };
        StaticLightData {
            light_type: 0,
            flags: 0,
            phase: 0,
            primary_active: spec,
            secondary_active: spec,
            becoming_active: spec,
            primary_inactive: spec,
            secondary_inactive: spec,
            becoming_inactive: spec,
            tag: 0,
        }
    }

    /// Map with two square polygons: poly 0 static (type 0), poly 1 a platform
    /// (type 5). Platform polygons spawn an implicit door platform that starts
    /// "extended" (closed) and can be activated to move.
    fn platform_map() -> MapData {
        // Two side-by-side 1024x1024 squares sharing endpoints 1 and 2.
        let endpoints = vec![
            mk_endpoint(0, 0, 0),
            mk_endpoint(1024, 0, 0),
            mk_endpoint(1024, 1024, 0),
            mk_endpoint(0, 1024, 0),
            mk_endpoint(2048, 0, 1),
            mk_endpoint(2048, 1024, 1),
        ];
        let lines = vec![
            mk_line(0, 1),
            mk_line(1, 2),
            mk_line(2, 3),
            mk_line(3, 0),
            mk_line(1, 4),
            mk_line(4, 5),
            mk_line(5, 2),
        ];
        // poly 0: static room, floor 0, ceiling 2048.
        let poly0 = mk_square_polygon(
            0,
            [0, 1, 2, 3, -1, -1, -1, -1],
            [0, 1, 2, 3, -1, -1, -1, -1],
            0,
            2048,
        );
        // poly 1: platform/door, floor 0, ceiling 2048.
        let poly1 = mk_square_polygon(
            POLYGON_IS_PLATFORM,
            [1, 4, 5, 2, -1, -1, -1, -1],
            [4, 5, 6, 1, -1, -1, -1, -1],
            0,
            2048,
        );

        MapData {
            endpoints,
            lines,
            sides: vec![],
            polygons: vec![poly0, poly1],
            objects: vec![],
            lights: LightData::Static(vec![constant_light(1.0)]),
            platforms: vec![],
            media: vec![],
            annotations: vec![],
            terminals: vec![],
            ambient_sounds: vec![],
            random_sounds: vec![],
            map_info: None,
            item_placement: vec![],
            guard_paths: None,
        }
    }

    fn empty_physics() -> PhysicsData {
        PhysicsData {
            monsters: None,
            effects: None,
            projectiles: None,
            physics: None,
            weapons: None,
        }
    }

    #[test]
    fn light_intensities_indexed_by_light_index() {
        // box 6.1: platform_map has one constant-1.0 light at index 0.
        let map = platform_map();
        let mut world =
            SimWorld::new(&map, &empty_physics(), &SimConfig::default()).expect("world");
        world.tick(crate::tick::TickInput::default());
        let intensities = world.light_intensities();
        assert_eq!(intensities.len(), 1, "one entry per light_index");
        assert!((intensities[0] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn media_heights_empty_without_media() {
        // box 6.2: platform_map has no media, so the Vec is empty. (The
        // populated case is covered by the tick_loop_media_tracks_light
        // integration test.)
        let map = platform_map();
        let mut world =
            SimWorld::new(&map, &empty_physics(), &SimConfig::default()).expect("world");
        world.tick(crate::tick::TickInput::default());
        assert!(world.media_heights().is_empty());
    }

    #[test]
    fn serialize_roundtrip_preserves_light_state() {
        // box 8.3: a SimWorld with state-machine lights survives a
        // serialize/deserialize round-trip with its light state intact.
        let map = platform_map();
        let mut world =
            SimWorld::new(&map, &empty_physics(), &SimConfig::default()).expect("world");
        for _ in 0..5 {
            world.tick(crate::tick::TickInput::default());
        }
        let before_intensities = world.light_intensities();
        let before_state = {
            let w = world.ecs_world_mut();
            let mut q = w.query::<&Light>();
            q.iter(w)
                .map(|l| (l.state, l.phase, l.period, l.light_index))
                .collect::<Vec<_>>()
        };

        let bytes = world.serialize().expect("serialize");
        let mut restored =
            SimWorld::deserialize(&bytes, &map, &empty_physics()).expect("deserialize");

        assert_eq!(
            restored.light_intensities(),
            before_intensities,
            "current intensities preserved"
        );
        let after_state = {
            let w = restored.ecs_world_mut();
            let mut q = w.query::<&Light>();
            q.iter(w)
                .map(|l| (l.state, l.phase, l.period, l.light_index))
                .collect::<Vec<_>>()
        };
        assert_eq!(after_state, before_state, "light state machine preserved");
    }

    #[test]
    fn poly_dynamic_data_tracks_moving_platform() {
        let map = platform_map();
        let physics = empty_physics();
        let config = SimConfig::default();
        let mut world = SimWorld::new(&map, &physics, &config).expect("world construction");

        // Tick once so `MapGeometry` is synced from the platform's current
        // (closed) state — the platform writes its live heights into geometry
        // during `update_platforms`, which is what `poly_dynamic_data` reads.
        world.tick(crate::tick::TickInput::default());

        let before = world.poly_dynamic_data();
        assert_eq!(before.len(), 2, "one entry per polygon");

        // Static polygon 0: floor 0.0, ceiling 2.0, full light, no media.
        assert_eq!(before[0].floor_height, 0.0);
        assert_eq!(before[0].ceiling_height, 2.0);
        assert_eq!(before[0].media_height, 0.0);
        assert_eq!(before[0].floor_light, 1.0);
        assert_eq!(before[0].ceiling_light, 1.0);

        // Platform polygon 1 starts extended (closed door): floor risen to
        // ceiling height (2.0), ceiling lowered to floor height (0.0).
        let start_floor = before[1].floor_height;
        let start_ceiling = before[1].ceiling_height;
        assert_eq!(start_floor, 2.0, "door starts closed (floor at ceiling)");
        assert_eq!(start_ceiling, 0.0, "door starts closed (ceiling at floor)");

        // Activate the platform (open the door) by flipping its state to
        // Returning, mirroring what an action-key activation does.
        {
            let ecs = world.ecs_world_mut();
            let mut q = ecs.query::<&mut crate::components::Platform>();
            for mut platform in q.iter_mut(ecs) {
                if platform.polygon_index == 1 {
                    crate::world_mechanics::platforms::activate_platform(&mut platform);
                }
            }
        }

        // Tick the sim a few times; the platform should move toward its rest
        // (open) position.
        for _ in 0..5 {
            world.tick(crate::tick::TickInput::default());
        }

        let after = world.poly_dynamic_data();
        assert_eq!(after.len(), 2);

        // Static polygon 0 is unchanged.
        assert_eq!(after[0].floor_height, before[0].floor_height);
        assert_eq!(after[0].ceiling_height, before[0].ceiling_height);
        assert_eq!(after[0].floor_light, before[0].floor_light);

        // Platform polygon 1's floor height has changed (door opening: floor
        // dropping from 2.0 toward 0.0).
        assert_ne!(
            after[1].floor_height, start_floor,
            "moving platform's floor height must change after ticking"
        );
        assert!(
            after[1].floor_height < start_floor,
            "opening door floor should drop from {} (got {})",
            start_floor,
            after[1].floor_height
        );
    }
}
