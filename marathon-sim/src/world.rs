use bevy_ecs::prelude::*;
use glam::{Vec2, Vec3};
use marathon_formats::MapData;
use marathon_formats::map::LightData;
use marathon_formats::physics::PhysicsData;
use rand::rngs::StdRng;
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
#[derive(Resource, Debug)]
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
    LevelTeleport { target_level: usize },
    TerminalActivation { terminal_index: usize },
    SoundTrigger { sound_index: usize, position: Vec3 },
    EntityDamaged { entity: Entity, amount: i16, damage_type: i16 },
    EntityKilled { entity: Entity },
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
        world.insert_resource(SimEvents::default());
        world.insert_resource(PhysicsTables {
            data: physics_data.clone(),
        });

        // Build map geometry resource
        let geometry = build_map_geometry(map_data);
        world.insert_resource(geometry);

        // Spawn entities from map objects
        spawn_map_objects(&mut world, map_data, physics_data, config)?;

        // Initialize platforms
        spawn_platforms(&mut world, map_data);

        // Initialize lights
        spawn_lights(&mut world, map_data);

        // Initialize media
        spawn_media(&mut world, map_data);

        Ok(Self { world })
    }

    /// Get the current tick count.
    pub fn tick_count(&self) -> u64 {
        self.world.resource::<TickCounter>().0
    }

    /// Drain pending simulation events.
    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        self.world.resource_mut::<SimEvents>().drain()
    }
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

    MapGeometry {
        polygon_vertices,
        floor_heights,
        ceiling_heights,
        polygon_adjacency,
        line_endpoints,
        line_solid,
        line_transparent,
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

    for obj in &map_data.objects {
        let pos = Vec3::new(
            world_coord(obj.location.x),
            world_coord(obj.location.y),
            world_coord(obj.location.z),
        );
        let facing = (obj.facing as f32) * (std::f32::consts::TAU / 512.0);
        let polygon = obj.polygon_index as usize;

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
                        Monster { definition_index: def_index },
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
                    entity.insert((
                        SpriteShape(def.stationary_shape),
                        AnimationFrame::default(),
                    ));

                    if is_flying {
                        entity.insert(Flying {
                            preferred_hover_height: world_coord(def.preferred_hover_height),
                        });
                    }
                }
            }
            OBJECT_IS_ITEM => {
                world.spawn((
                    Item { item_type: obj.index },
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
    for platform in &map_data.platforms {
        let poly_idx = platform.polygon_index as usize;
        let speed = world_coord(platform.speed);
        let min_height = world_coord(platform.minimum_height);
        let max_height = world_coord(platform.maximum_height);

        // Marathon platforms move floors between min and max height.
        // Ceiling behavior depends on platform type; default to static ceiling.
        world.spawn(Platform {
            polygon_index: poly_idx,
            floor_rest: min_height,
            floor_extended: max_height,
            ceiling_rest: 0.0,
            ceiling_extended: 0.0,
            current_floor: min_height,
            current_ceiling: 0.0,
            speed,
            state: PlatformState::AtRest,
            return_delay: platform.delay as u16,
            delay_remaining: 0,
            activation_flags: platform.static_flags,
            crushes: platform.static_flags & 0x2000 != 0,
        });
    }
}

fn spawn_lights(world: &mut World, map_data: &MapData) {
    let lights = match &map_data.lights {
        LightData::Static(lights) => lights,
        _ => return,
    };

    for (idx, light) in lights.iter().enumerate() {
        let spec = &light.primary_active;
        let function = match spec.function {
            0 => LightFunction::Constant,
            1 => LightFunction::Linear,
            2 => LightFunction::Smooth,
            3 => LightFunction::Flicker,
            _ => LightFunction::Constant,
        };

        let intensity_min = spec.intensity;
        let intensity_max = spec.intensity + spec.delta_intensity;
        let period = spec.period as u32;
        let phase = light.phase as u32;

        world.spawn(Light {
            light_index: idx,
            function,
            period: period.max(1),
            phase,
            intensity_min,
            intensity_max,
            current_intensity: intensity_max,
        });
    }
}

fn spawn_media(world: &mut World, map_data: &MapData) {
    for media in &map_data.media {
        world.spawn(Media {
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

/// Errors during simulation world construction.
#[derive(Debug, thiserror::Error)]
pub enum SimWorldError {
    #[error("Missing physics data: {0}")]
    MissingPhysicsData(String),
}
