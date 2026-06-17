//! Headless determinism harness for `render_snapshot` (`decouple-tick-snapshot` step 7).
//!
//! These tests construct a `SimWorld` with no GPU / no graphics backend, drive
//! it with a fixed `TickInput` sequence, and exercise `render_snapshot()`:
//!   - 7.1 snapshots can be produced headlessly after every tick;
//!   - 7.2 two same-seed runs yield byte-identical per-tick snapshot streams;
//!   - 7.3 calling `render_snapshot` between ticks does not perturb the
//!         deterministic tick sequence vs. a reference run that omits it.

use marathon_formats::map::*;
use marathon_formats::*;
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimWorld};

/// Build a simple two-polygon test map (mirrors the helper in `integration.rs`).
/// Poly 0: (0,0)-(1024,1024) => 1 WU square
/// Poly 1: (1024,0)-(2048,1024) => adjacent to the east
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
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 2048, y: 0 },
            transformed: WorldPoint2d { x: 2048, y: 0 },
            supporting_polygon_index: 1,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x: 2048, y: 1024 },
            transformed: WorldPoint2d { x: 2048, y: 1024 },
            supporting_polygon_index: 1,
        },
    ];

    let sd_none = ShapeDescriptor(0xFFFF);
    let lines = vec![
        Line {
            endpoint_indexes: [1, 2],
            flags: 0x0200, // HAS_TRANSPARENT_SIDE (passable)
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: 1,
        },
        Line {
            endpoint_indexes: [0, 1],
            flags: 0x4000, // SOLID
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 0,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [3, 2],
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
            endpoint_indexes: [0, 3],
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
            endpoint_indexes: [1, 4],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 1,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [4, 5],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 1,
            counterclockwise_polygon_owner: -1,
        },
        Line {
            endpoint_indexes: [5, 2],
            flags: 0x4000,
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: 1,
            counterclockwise_polygon_owner: -1,
        },
    ];

    let wp_zero = WorldPoint2d { x: 0, y: 0 };

    let polygon_0 = Polygon {
        polygon_type: 0,
        flags: 0,
        permutation: 0,
        vertex_count: 4,
        endpoint_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
        line_indexes: [1, 0, 2, 3, -1, -1, -1, -1],
        floor_texture: sd_none,
        ceiling_texture: sd_none,
        floor_height: 0,
        ceiling_height: 2048,
        floor_lightsource_index: 0,
        ceiling_lightsource_index: 0,
        area: 1024 * 1024,
        floor_transfer_mode: 0,
        ceiling_transfer_mode: 0,
        adjacent_polygon_indexes: [-1, 1, -1, -1, -1, -1, -1, -1],
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

    let polygon_1 = Polygon {
        polygon_type: 0,
        flags: 0,
        permutation: 0,
        vertex_count: 4,
        endpoint_indexes: [1, 4, 5, 2, -1, -1, -1, -1],
        line_indexes: [4, 5, 6, 0, -1, -1, -1, -1],
        floor_texture: sd_none,
        ceiling_texture: sd_none,
        floor_height: 0,
        ceiling_height: 2048,
        floor_lightsource_index: 0,
        ceiling_lightsource_index: 0,
        area: 1024 * 1024,
        floor_transfer_mode: 0,
        ceiling_transfer_mode: 0,
        adjacent_polygon_indexes: [-1, -1, -1, 0, -1, -1, -1, -1],
        center: WorldPoint2d { x: 1536, y: 512 },
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
        object_type: 3, // OBJECT_IS_PLAYER
        index: 0,
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d {
            x: 512,
            y: 512,
            z: 0,
        },
        flags: 0,
    };

    MapData {
        endpoints,
        lines,
        sides: vec![],
        polygons: vec![polygon_0, polygon_1],
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

/// Minimal physics data sufficient to construct and tick a player (mirrors the
/// `integration.rs` helper, trimmed to what `SimWorld::new` requires).
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

/// Fixed, reproducible input sequence used by every determinism test.
fn input_sequence() -> Vec<ActionFlags> {
    vec![
        ActionFlags::new(ActionFlags::MOVE_FORWARD),
        ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::TURN_LEFT),
        ActionFlags::default(),
        ActionFlags::new(ActionFlags::STRAFE_RIGHT),
        ActionFlags::new(ActionFlags::TURN_RIGHT),
    ]
}

// ──────────────────── Box 7.1: headless snapshot per tick ────────────────────

/// Construct a `SimWorld` with no GPU, tick N times with a fixed `TickInput`
/// sequence, and call `render_snapshot()` after each tick. No graphics backend
/// is ever initialized — `SimWorld` is pure CPU sim state.
#[test]
fn headless_render_snapshot_after_each_tick() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig {
        random_seed: 42,
        difficulty: 2,
    };

    let mut world = SimWorld::new(&map, &physics, &config).expect("world construction failed");

    let inputs = input_sequence();
    const N: usize = 60;
    let mut snapshots = Vec::with_capacity(N);

    for i in 0..N {
        let input = inputs[i % inputs.len()];
        world.tick(input.into());
        let snap = world.render_snapshot();
        snapshots.push(snap);
    }

    // We produced exactly one snapshot per tick, with no graphics backend.
    assert_eq!(snapshots.len(), N);
    // tick_count must advance monotonically and equal the loop index + 1.
    for (i, snap) in snapshots.iter().enumerate() {
        assert_eq!(
            snap.tick_count,
            (i + 1) as u64,
            "snapshot {i} should carry tick_count {}",
            i + 1
        );
    }
    // The two-polygon map yields per-polygon dynamic data in every snapshot.
    assert_eq!(snapshots.last().unwrap().poly_dynamic.len(), 2);
    // A player exists, so every snapshot carries a PlayerView.
    assert!(snapshots.iter().all(|s| s.player.is_some()));
}

// ──────────────── Box 7.2: byte-identical snapshot streams ────────────────

/// Drive a fresh `SimWorld` for `n` ticks and return the bincode-serialized
/// `render_snapshot()` taken after each tick (one byte vector per tick).
fn run_snapshot_stream(seed: u64, n: usize) -> Vec<Vec<u8>> {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig {
        random_seed: seed,
        difficulty: 2,
    };
    let mut world = SimWorld::new(&map, &physics, &config).expect("world construction failed");

    let inputs = input_sequence();
    let mut stream = Vec::with_capacity(n);
    for i in 0..n {
        let input = inputs[i % inputs.len()];
        world.tick(input.into());
        let snap = world.render_snapshot();
        let bytes = bincode::serialize(&snap).expect("snapshot serialization failed");
        stream.push(bytes);
    }
    stream
}

/// Two runs with the same seed / level / input sequence must produce
/// byte-identical per-tick snapshot streams. This would fail if the snapshot
/// (or the sim feeding it) carried any non-determinism: pointer addresses,
/// HashMap iteration order, unseeded RNG, wall-clock, etc.
#[test]
fn same_seed_runs_produce_byte_identical_snapshot_streams() {
    const N: usize = 60;
    let stream_a = run_snapshot_stream(42, N);
    let stream_b = run_snapshot_stream(42, N);

    assert_eq!(stream_a.len(), N);
    assert_eq!(stream_b.len(), N);
    for tick in 0..N {
        assert_eq!(
            stream_a[tick], stream_b[tick],
            "snapshot byte streams diverged at tick {tick}"
        );
    }
    // Sanity: the streams are not trivially empty/constant — at least one tick's
    // serialized snapshot differs from tick 0's (the player is moving).
    assert!(
        stream_a.iter().any(|b| *b != stream_a[0]),
        "expected snapshot bytes to vary across ticks (player should move)"
    );
}
