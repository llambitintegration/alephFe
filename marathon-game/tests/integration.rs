//! Headless integration tests for marathon-game.
//! Tests scenario loading, sim initialization, and entity state collection
//! without requiring a GPU.

use marathon_formats::map::*;
use marathon_formats::*;
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimWorld};

/// Build a simple two-polygon test map with a player start.
fn make_test_map() -> MapData {
    let endpoints = vec![
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 0, y: 0 },
            transformed: WorldPoint2d { x: 0, y: 0 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 1024, y: 0 },
            transformed: WorldPoint2d { x: 1024, y: 0 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 1024, y: 1024 },
            transformed: WorldPoint2d { x: 1024, y: 1024 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 0, y: 1024 },
            transformed: WorldPoint2d { x: 0, y: 1024 },
            supporting_polygon_index: 0,
        },
    ];

    let sd_none = ShapeDescriptor(0xFFFF);

    let lines = vec![
        Line {
            endpoint_indexes: [0, 1],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [1, 2],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [2, 3],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [3, 0],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        },
    ];

    let wp_zero = WorldPoint2d { x: 0, y: 0 };

    let polygon = Polygon {
        polygon_type: 0,
        flags: 0,
        permutation: 0,
        vertex_count: 4,
        endpoint_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
        line_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
        floor_texture: sd_none,
        ceiling_texture: sd_none,
        floor_height: 0,
        ceiling_height: 2048,
        floor_lightsource_index: 0,
        ceiling_lightsource_index: 0,
        area: 1024 * 1024,
        floor_transfer_mode: 0,
        ceiling_transfer_mode: 0,
        adjacent_polygon_indexes: [-1; 8],
        center: WorldPoint2d { x: 512, y: 512 },
        side_indexes: [-1; 8],
        floor_origin: wp_zero,
        ceiling_origin: wp_zero,
        media_index: -1,
        media_lightsource_index: -1,
        sound_source_indexes: -1,
        ambient_sound_image_index: -1,
        random_sound_image_index: -1,
    };

    let player_obj = MapObject {
        object_type: 3,
        index: 0,
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 512, y: 512, z: 0 },
        flags: 0,
    };

    MapData {
        endpoints,
        lines,
        sides: vec![],
        polygons: vec![polygon],
        objects: vec![player_obj],
        lights: LightData::Static(vec![StaticLightData {
            light_type: 0,
            flags: 0,
            phase: 0,
            primary_active: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 1.0,
                delta_intensity: 0.0,
            },
            secondary_active: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 1.0,
                delta_intensity: 0.0,
            },
            becoming_active: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 1.0,
                delta_intensity: 0.0,
            },
            primary_inactive: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 0.0,
                delta_intensity: 0.0,
            },
            secondary_inactive: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 0.0,
                delta_intensity: 0.0,
            },
            becoming_inactive: LightingFunctionSpec {
                function: 0,
                period: 1,
                delta_period: 0,
                intensity: 0.0,
                delta_intensity: 0.0,
            },
            tag: 0,
        }]),
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

fn make_test_physics() -> PhysicsData {
    PhysicsData {
        monsters: Some(vec![]),
        effects: Some(vec![]),
        projectiles: Some(vec![]),
        physics: Some(vec![PhysicsConstants {
            maximum_forward_velocity: 0.1,
            maximum_backward_velocity: 0.05,
            maximum_perpendicular_velocity: 0.08,
            acceleration: 0.01,
            deceleration: 0.005,
            airborne_deceleration: 0.002,
            gravitational_acceleration: 0.005,
            climbing_acceleration: 0.01,
            terminal_velocity: 0.5,
            external_deceleration: 0.01,
            angular_acceleration: 0.05,
            angular_deceleration: 0.03,
            maximum_angular_velocity: 0.2,
            angular_recentering_velocity: 0.1,
            fast_angular_velocity: 0.3,
            fast_angular_maximum: 0.4,
            maximum_elevation: 0.5,
            external_angular_deceleration: 0.05,
            step_delta: 0.25,
            step_amplitude: 0.02,
            radius: 0.25,
            height: 0.8,
            dead_height: 0.3,
            camera_height: 0.6,
            splash_height: 0.1,
            half_camera_separation: 0.05,
        }]),
        weapons: Some(vec![]),
    }
}

#[test]
fn sim_initialization_from_map_data() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig {
        random_seed: 42,
        difficulty: 2,
    };

    let mut world = SimWorld::new(&map, &physics, &config).expect("sim init failed");
    assert!(world.player_position().is_some());
    assert_eq!(world.tick_count(), 0);
}

#[test]
fn sim_tick_advances_state() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();
    let empty = ActionFlags::new(0);

    for _ in 0..30 {
        world.tick(empty);
    }

    assert_eq!(world.tick_count(), 30);
}

#[test]
fn sim_accepts_action_flags() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Verify sim accepts various action flags without panicking
    let forward = ActionFlags::new(ActionFlags::MOVE_FORWARD);
    let strafe = ActionFlags::new(ActionFlags::STRAFE_LEFT);
    let combined = ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::TURN_RIGHT);

    world.tick(forward);
    world.tick(strafe);
    world.tick(combined);
    assert_eq!(world.tick_count(), 3);

    // Player should still be alive
    assert!(world.player_health().unwrap() > 0);
}

#[test]
fn entity_query_returns_results() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();
    // Query entities — should return at least the player-related entities (if any)
    let entities = world.entities();
    // With no monsters placed, entities list may be empty — that's fine
    // Just verify the method doesn't panic
    let _ = entities.len();
}

#[test]
fn snapshot_captures_state() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Tick a few times
    let empty = ActionFlags::new(0);
    for _ in 0..10 {
        world.tick(empty);
    }

    let snapshot = world.snapshot();
    assert_eq!(snapshot.tick_count, 10);
}

#[test]
fn drain_events_does_not_panic() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    let empty = ActionFlags::new(0);
    world.tick(empty);

    let events = world.drain_events();
    // No level teleporters in our test map, so no teleport events expected
    for event in &events {
        match event {
            marathon_sim::world::SimEvent::LevelTeleport { .. } => {
                panic!("Unexpected level teleport in simple test map")
            }
            _ => {}
        }
    }
}
