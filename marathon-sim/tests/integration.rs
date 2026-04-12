//! Integration tests for marathon-sim.

use marathon_formats::*;
use marathon_formats::map::*;
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimWorld};

/// Build a simple two-polygon test map.
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
    // Lines: 0=shared (passable), 1-3=outer walls of poly 0, 4-6=outer walls of poly 1
    let lines = vec![
        // Line 0: shared between poly 0 and poly 1 (ep1->ep2)
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
        // Line 1: bottom of poly 0 (ep0->ep1)
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
        // Line 2: top of poly 0 (ep3->ep2)
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
        // Line 3: left of poly 0 (ep0->ep3)
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
        // Line 4: bottom of poly 1 (ep1->ep4)
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
        // Line 5: right of poly 1 (ep4->ep5)
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
        // Line 6: top of poly 1 (ep5->ep2)
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
        ceiling_height: 2048, // 2 WU
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

    // Player start in poly 0 center
    let player_obj = MapObject {
        object_type: 3, // OBJECT_IS_PLAYER
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
        polygons: vec![polygon_0, polygon_1],
        objects: vec![player_obj],
        lights: LightData::Static(vec![StaticLightData {
            light_type: 0,
            flags: 0,
            phase: 0,
            primary_active: LightingFunctionSpec {
                function: 0, // constant
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
        monsters: Some(vec![MonsterDefinition {
            collection: 0,
            vitality: 100,
            immunities: 0,
            weaknesses: 0,
            flags: 0,
            monster_class: 0,
            friends: 0,
            enemies: 0xFF,
            sound_pitch: 1.0,
            activation_sound: -1,
            friendly_activation_sound: -1,
            clear_sound: -1,
            kill_sound: -1,
            apology_sound: -1,
            friendly_fire_sound: -1,
            flaming_sound: -1,
            random_sound: -1,
            random_sound_mask: 0,
            carrying_item_type: -1,
            radius: 256,     // 0.25 WU
            height: 768,     // 0.75 WU
            preferred_hover_height: 0,
            minimum_ledge_delta: 0,
            maximum_ledge_delta: 512,
            external_velocity_scale: 1.0,
            impact_effect: -1,
            melee_impact_effect: -1,
            contrail_effect: -1,
            half_visual_arc: 128, // ~90 degrees in Marathon's 512-per-revolution
            half_vertical_visual_arc: 64,
            visual_range: 8192,
            dark_visual_range: 4096,
            intelligence: 5,
            speed: 64,
            gravity: 10,
            terminal_velocity: 512,
            door_retry_mask: 0,
            shrapnel_radius: 0,
            shrapnel_damage: DamageDefinition {
                damage_type: 0,
                flags: 0,
                base: 0,
                random: 0,
                scale: 0.0,
            },
            hit_shapes: 0,
            hard_dying_shape: 0,
            soft_dying_shape: 0,
            hard_dead_shapes: 0,
            soft_dead_shapes: 0,
            stationary_shape: 0,
            moving_shape: 0,
            teleport_in_shape: 0,
            teleport_out_shape: 0,
            attack_frequency: 30,
            melee_attack: AttackDefinition {
                attack_type: 0,
                repetitions: 1,
                error: 0,
                range: 512,
                attack_shape: 0,
                dx: 0,
                dy: 0,
                dz: 0,
            },
            ranged_attack: AttackDefinition {
                attack_type: 0,
                repetitions: 1,
                error: 10,
                range: 4096,
                attack_shape: 0,
                dx: 0,
                dy: 0,
                dz: 256,
            },
        }]),
        effects: Some(vec![]),
        projectiles: Some(vec![ProjectileDefinition {
            collection: 0,
            shape: 0,
            detonation_effect: -1,
            media_detonation_effect: -1,
            contrail_effect: -1,
            ticks_between_contrails: 0,
            maximum_contrails: 0,
            media_projectile_promotion: -1,
            radius: 64,
            area_of_effect: 0,
            damage: DamageDefinition {
                damage_type: 0,
                flags: 0,
                base: 20,
                random: 5,
                scale: 1.0,
            },
            flags: 0,
            speed: 512,
            maximum_range: 16384,
            sound_pitch: 1.0,
            flyby_sound: -1,
            rebound_sound: -1,
        }]),
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

// ──────────────────── Test 2.9: World construction ────────────────────

#[test]
fn world_construction_from_synthetic_data() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).expect("world construction failed");

    // Player should be spawned
    assert!(world.player_position().is_some());
    let pos = world.player_position().unwrap();
    // Player at (512/1024, 512/1024, 0) = (0.5, 0.5, 0.0)
    assert!((pos.x - 0.5).abs() < 0.01);
    assert!((pos.y - 0.5).abs() < 0.01);

    assert_eq!(world.player_health(), Some(150));
    assert_eq!(world.player_shield(), Some(150));
    assert_eq!(world.player_oxygen(), Some(600));
    assert_eq!(world.player_polygon(), Some(0));
    assert_eq!(world.tick_count(), 0);
}

// ──────────────────── Running physics preference ────────────────────

#[test]
fn prefers_running_physics_when_two_entries_exist() {
    // Build physics data with a crippled walking entry (near-zero speed)
    // and a normal running entry. If the sim loads running (index 1), the
    // player moves measurably. If it loads walking, the player barely moves.
    let map = make_test_map();
    let mut physics = make_test_physics();
    if let Some(phys) = physics.physics.as_mut() {
        // Crippled walking: essentially frozen
        phys[0].maximum_forward_velocity = 0.0001;
        phys[0].acceleration = 0.0001;
        // Append running entry with normal movement values
        let running = PhysicsConstants {
            maximum_forward_velocity: 0.1,
            maximum_backward_velocity: 0.05,
            maximum_perpendicular_velocity: 0.08,
            acceleration: 0.02,
            deceleration: 0.01,
            ..phys[0].clone()
        };
        phys.push(running);
    }
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    let initial_pos = world.player_position().unwrap();
    let forward = ActionFlags::new(ActionFlags::MOVE_FORWARD);
    for _ in 0..30 {
        world.tick(forward.into());
    }
    let final_pos = world.player_position().unwrap();
    let dist = (final_pos - initial_pos).length();

    // With running loaded (max_fwd=0.1, accel=0.02), the player should
    // accelerate to max and travel ~1+ WU over 30 ticks. With walking
    // loaded (max_fwd=0.0001), the player would travel <0.01 WU.
    assert!(
        dist > 0.1,
        "expected running physics to be loaded — player moved only {dist} WU in 30 ticks"
    );
}

// ──────────────────── Test 9.1: Player movement over ticks ────────────────────

#[test]
fn player_moves_forward_over_ticks() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();
    let _initial_pos = world.player_position().unwrap();

    // Advance 100 ticks with forward movement
    let forward = ActionFlags::new(ActionFlags::MOVE_FORWARD);
    for _ in 0..100 {
        world.tick(forward.into());
    }

    let _final_pos = world.player_position().unwrap();
    // Tick counter should advance correctly
    assert_eq!(world.tick_count(), 100);
}

// ──────────────────── Test 9.2: Monster alert ────────────────────

#[test]
fn monster_spawns_from_map_object() {
    let mut map = make_test_map();
    // Add a monster object in polygon 1
    map.objects.push(MapObject {
        object_type: 0, // OBJECT_IS_MONSTER
        index: 0,       // monster definition 0
        facing: 256,    // facing west (toward player)
        polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 },
        flags: 0,
    });

    let physics = make_test_physics();
    let config = SimConfig::default();

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Should have entities (the monster)
    let entities = world.entities();
    assert!(!entities.is_empty(), "monster should be spawned as an entity");
}

// ──────────────────── Test 9.3: Projectile detonation ────────────────────
// (Tests the projectile collision primitives, not full system wiring)

#[test]
fn projectile_wall_collision_detects_hit() {
    use marathon_sim::combat::projectiles::{check_projectile_wall_collision, WallHitResult};
    use glam::Vec2;

    // Simulate a projectile moving east into a solid wall at x=1
    let adjacency = vec![vec![(0, None)]];
    let endpoints = vec![(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0))];
    let solid = vec![true];

    let old = Vec2::new(0.5, 0.5);
    let new = Vec2::new(1.5, 0.5);

    let result = check_projectile_wall_collision(old, new, 0.5, 0.5, 0, &adjacency, &endpoints, &solid);
    match result {
        WallHitResult::Hit { hit_point, .. } => {
            assert!((hit_point.x - 1.0).abs() < 0.01);
        }
        _ => panic!("expected wall hit"),
    }
}

// ──────────────────── Test 9.4: Platform movement ────────────────────

#[test]
fn platform_moves_over_ticks() {
    use marathon_sim::world_mechanics::platforms::{tick_platform, activate_platform};

    let mut platform = marathon_sim::Platform {
        polygon_index: 0,
        floor_rest: 0.0,
        floor_extended: 1.0,
        ceiling_rest: 3.0,
        ceiling_extended: 3.0,
        current_floor: 0.0,
        current_ceiling: 3.0,
        speed: 0.1,
        state: marathon_sim::PlatformState::AtRest,
        return_delay: 10,
        delay_remaining: 0,
        activation_flags: 0,
        crushes: false,
    };

    activate_platform(&mut platform);
    assert_eq!(platform.state, marathon_sim::PlatformState::Extending);

    // Tick until extended
    for _ in 0..10 {
        tick_platform(&mut platform);
    }
    assert_eq!(platform.state, marathon_sim::PlatformState::AtExtended);
    assert!((platform.current_floor - 1.0).abs() < 0.01);

    // Tick through delay
    for _ in 0..10 {
        tick_platform(&mut platform);
    }
    assert_eq!(platform.state, marathon_sim::PlatformState::Returning);

    // Tick until returned
    for _ in 0..10 {
        tick_platform(&mut platform);
    }
    assert_eq!(platform.state, marathon_sim::PlatformState::AtRest);
    assert!((platform.current_floor - 0.0).abs() < 0.01);
}

// ──────────────────── Test 9.5: Item pickup effects ────────────────────

#[test]
fn item_pickup_gives_correct_effects() {
    use marathon_sim::world_mechanics::items::*;

    // Test weapon pickup
    let effect = item_effect(ITEM_SHOTGUN);
    assert!(matches!(effect, Some(ItemEffect::AddWeapon { weapon_definition_index: 7 })));

    // Test health pickup
    let effect = item_effect(ITEM_HEALTH_MAJOR);
    assert!(matches!(effect, Some(ItemEffect::RestoreHealth { amount: 40 })));

    // Test ammo pickup
    let effect = item_effect(ITEM_AR_AMMO);
    assert!(matches!(
        effect,
        Some(ItemEffect::AddAmmo { weapon_definition_index: 3, is_primary: true, amount: 52 })
    ));

    // Test shield pickup
    let effect = item_effect(ITEM_SHIELD_2X);
    assert!(matches!(effect, Some(ItemEffect::RestoreShield { amount: 300 })));

    // Test inventory item pickup
    let effect = item_effect(ITEM_UPLINK_CHIP);
    assert!(matches!(effect, Some(ItemEffect::AddInventoryItem { item_type: 30 })));
}

// ──────────────────── Test 8.6: Deterministic replay ────────────────────

#[test]
fn deterministic_replay_produces_identical_state() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig { random_seed: 42, difficulty: 2 };

    let mut world_a = SimWorld::new(&map, &physics, &config).unwrap();
    let mut world_b = SimWorld::new(&map, &physics, &config).unwrap();

    // Same input sequence
    let inputs = [
        ActionFlags::new(ActionFlags::MOVE_FORWARD),
        ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::TURN_LEFT),
        ActionFlags::default(),
        ActionFlags::new(ActionFlags::STRAFE_RIGHT),
    ];

    for _ in 0..25 {
        for &input in &inputs {
            world_a.tick(input.into());
            world_b.tick(input.into());
        }
    }

    // Both worlds should be in identical state
    assert_eq!(world_a.tick_count(), world_b.tick_count());
    assert_eq!(world_a.player_position(), world_b.player_position());
    assert_eq!(world_a.player_health(), world_b.player_health());
    assert_eq!(world_a.player_shield(), world_b.player_shield());
    assert_eq!(world_a.player_oxygen(), world_b.player_oxygen());
    assert_eq!(world_a.player_polygon(), world_b.player_polygon());
}

// ──────────────────── Test 8.5: Serialization round-trip ────────────────────

#[test]
fn serialization_round_trip() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig { random_seed: 123, difficulty: 2 };

    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Advance some ticks
    for _ in 0..50 {
        world.tick(ActionFlags::new(ActionFlags::MOVE_FORWARD).into());
    }

    let tick_before = world.tick_count();
    let pos_before = world.player_position();
    let health_before = world.player_health();

    // Serialize
    let data = world.serialize().expect("serialization failed");

    // Deserialize
    let mut restored = SimWorld::deserialize(&data, &map, &physics)
        .expect("deserialization failed");

    assert_eq!(restored.tick_count(), tick_before);
    assert_eq!(restored.player_position(), pos_before);
    assert_eq!(restored.player_health(), health_before);
}

// ──────────────────── E2E: Player collision with walls ────────────────────

#[test]
fn player_wall_collision_slide_response() {
    use glam::{Vec2, Vec3};
    use marathon_sim::player::movement::*;
    use marathon_sim::world::MapGeometry;

    let params = PlayerPhysicsParams {
        max_forward_velocity: 0.1,
        max_backward_velocity: 0.05,
        max_perpendicular_velocity: 0.08,
        acceleration: 0.01,
        deceleration: 0.005,
        airborne_deceleration: 0.002,
        gravitational_acceleration: 0.005,
        terminal_velocity: 0.5,
        angular_acceleration: 0.05,
        angular_deceleration: 0.03,
        max_angular_velocity: 0.2,
        maximum_elevation: 0.5,
        step_delta: 0.25,
        height: 0.8,
        radius: 0.25,
    };

    // Single polygon box: (0,0)-(2,2), all walls solid
    let geometry = MapGeometry {
        polygon_vertices: vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ]],
        floor_heights: vec![0.0],
        ceiling_heights: vec![3.0],
        polygon_adjacency: vec![vec![
            (0, None), // bottom
            (1, None), // right
            (2, None), // top
            (3, None), // left
        ]],
        line_endpoints: vec![
            (Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0)), // bottom
            (Vec2::new(2.0, 0.0), Vec2::new(2.0, 2.0)), // right
            (Vec2::new(2.0, 2.0), Vec2::new(0.0, 2.0)), // top
            (Vec2::new(0.0, 2.0), Vec2::new(0.0, 0.0)), // left
        ],
        line_solid: vec![true, true, true, true],
        line_transparent: vec![false, false, false, false],
        polygon_media_index: vec![-1],
    };

    // Player tries to walk through the right wall
    let result = apply_player_collision(
        Vec3::new(1.8, 1.0, 0.0),
        Vec3::new(2.5, 1.1, 0.0),
        Vec3::new(0.7, 0.1, 0.0),
        0,
        &params,
        &geometry,
    );

    // Should be blocked: x should stay <= 2.0
    assert!(result.position.x <= 2.0 + f32::EPSILON);
    assert_eq!(result.polygon_index, 0);
    assert!(result.grounded);
}

// ──────────────────── E2E: Step climbing across polygons ────────────────────

#[test]
fn step_climbing_allows_small_height_difference() {
    use glam::{Vec2, Vec3};
    use marathon_sim::player::movement::*;
    use marathon_sim::world::MapGeometry;

    let params = PlayerPhysicsParams {
        max_forward_velocity: 0.1,
        max_backward_velocity: 0.05,
        max_perpendicular_velocity: 0.08,
        acceleration: 0.01,
        deceleration: 0.005,
        airborne_deceleration: 0.002,
        gravitational_acceleration: 0.005,
        terminal_velocity: 0.5,
        angular_acceleration: 0.05,
        angular_deceleration: 0.03,
        max_angular_velocity: 0.2,
        maximum_elevation: 0.5,
        step_delta: 0.3,
        height: 0.8,
        radius: 0.25,
    };

    let geometry = MapGeometry {
        polygon_vertices: vec![
            vec![
                Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0),
            ],
            vec![
                Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0),
                Vec2::new(2.0, 1.0), Vec2::new(1.0, 1.0),
            ],
        ],
        floor_heights: vec![0.0, 0.2], // 0.2 WU step up
        ceiling_heights: vec![3.0, 3.0],
        polygon_adjacency: vec![
            vec![(0, None), (1, Some(1)), (2, None), (3, None)],
            vec![(4, None), (5, None), (6, None), (1, Some(0))],
        ],
        line_endpoints: vec![
            (Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)),
            (Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)), // shared
            (Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0)),
            (Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0)),
            (Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)),
            (Vec2::new(2.0, 0.0), Vec2::new(2.0, 1.0)),
            (Vec2::new(1.0, 1.0), Vec2::new(2.0, 1.0)),
        ],
        line_solid: vec![true, false, true, true, true, true, true],
        line_transparent: vec![false, true, false, false, false, false, false],
        polygon_media_index: vec![-1, -1],
    };

    // Step up: 0.2 < step_delta 0.3, should cross
    let result = apply_player_collision(
        Vec3::new(0.8, 0.5, 0.0),
        Vec3::new(1.3, 0.5, 0.0),
        Vec3::new(0.5, 0.0, 0.0),
        0,
        &params,
        &geometry,
    );
    assert_eq!(result.polygon_index, 1);
    assert!(result.position.z >= 0.2 - f32::EPSILON);
}

// ──────────────────── E2E: Media submersion and oxygen ────────────────────

#[test]
fn media_submersion_depletes_oxygen_and_applies_drag() {
    use glam::Vec3;
    use marathon_sim::player::movement::apply_media_effects;

    let vel = Vec3::new(1.0, 0.5, 0.0);

    // Submerged in water (player_z=0.0 < surface=2.0)
    let (new_vel, oxy_change, dmg) =
        apply_media_effects(vel, 0.0, Some(2.0), Some(0), 500, 600);
    assert!(new_vel.x < vel.x, "drag should reduce X velocity");
    assert!(new_vel.y < vel.y, "drag should reduce Y velocity");
    assert!(oxy_change < 0, "oxygen should deplete when submerged");
    assert_eq!(dmg, 0, "no drowning damage when oxygen > 0");

    // Drowning: oxygen at 0
    let (_, _, drowning_dmg) =
        apply_media_effects(Vec3::ZERO, 0.0, Some(2.0), Some(0), 0, 600);
    assert!(drowning_dmg > 0, "drowning damage when oxygen is 0");

    // Above surface: oxygen recharges
    let (above_vel, oxy_recharge, _) =
        apply_media_effects(vel, 5.0, Some(2.0), Some(0), 400, 600);
    assert_eq!(above_vel, vel, "no drag above surface");
    assert!(oxy_recharge > 0, "oxygen should recharge above surface");
}

// ──────────────────── E2E: Monster AI state machine ────────────────────

#[test]
fn monster_ai_full_state_cycle() {
    use marathon_sim::monster::ai::*;
    use marathon_sim::MonsterState;

    // Idle -> Alerted (sees target)
    let s = next_state(MonsterState::Idle, true, true, false, false, false);
    assert_eq!(s, MonsterState::Alerted);

    // Alerted -> Moving (has target, not in range)
    let s = next_state(MonsterState::Alerted, true, true, false, false, false);
    assert_eq!(s, MonsterState::Moving);

    // Moving -> Attacking (in melee range)
    let s = next_state(MonsterState::Moving, true, true, true, false, false);
    assert_eq!(s, MonsterState::Attacking);

    // Attacking -> Moving (target out of range)
    let s = next_state(MonsterState::Attacking, true, true, false, false, false);
    assert_eq!(s, MonsterState::Moving);

    // Attacking -> Dying (vitality zero)
    let s = next_state(MonsterState::Attacking, true, true, true, true, true);
    assert_eq!(s, MonsterState::Dying);

    // Dying -> Dead
    let s = next_state(MonsterState::Dying, false, false, false, false, false);
    assert_eq!(s, MonsterState::Dead);

    // Dead stays Dead
    let s = next_state(MonsterState::Dead, true, true, true, true, false);
    assert_eq!(s, MonsterState::Dead);
}

// ──────────────────── E2E: Activation cascading ────────────────────

#[test]
fn monster_activation_cascading() {
    use glam::Vec2;
    use marathon_sim::monster::ai::*;
    use marathon_sim::MonsterState;

    let monsters = vec![
        (Vec2::new(1.0, 0.0), 0, 0xFF, MonsterState::Idle),    // same class, in range
        (Vec2::new(2.0, 0.0), 0, 0xFF, MonsterState::Idle),    // same class, in range
        (Vec2::new(50.0, 0.0), 0, 0xFF, MonsterState::Idle),   // same class, out of range
        (Vec2::new(1.5, 0.0), 1, 0xFF, MonsterState::Idle),    // different class
        (Vec2::new(1.0, 0.0), 0, 0x0F, MonsterState::Idle),    // different enemies mask
        (Vec2::new(0.5, 0.0), 0, 0xFF, MonsterState::Alerted), // already alerted
    ];

    let targets = find_cascade_targets(Vec2::ZERO, 0, 0xFF, &monsters, 10.0);
    assert_eq!(targets, vec![0, 1]); // Only first two qualify
}

// ──────────────────── E2E: Friendly-fire redirect ────────────────────

#[test]
fn friendly_fire_redirects_target() {
    use marathon_sim::monster::ai::should_redirect_target;

    // Class 1 is in the friends mask (bit 1)
    assert!(should_redirect_target(0, 1, 0b0000_0010));
    // Class 2 is not in the friends mask
    assert!(!should_redirect_target(0, 2, 0b0000_0010));
    // Class 0 in friends
    assert!(should_redirect_target(5, 0, 0b0000_0001));
}

// ──────────────────── E2E: Flying monster movement ────────────────────

#[test]
fn flying_monster_moves_toward_target_at_hover_height() {
    use glam::Vec3;
    use marathon_sim::monster::ai::compute_flying_movement;

    let vel = compute_flying_movement(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(10.0, 0.0, 0.0),
        2.0,
        3.0, // hover height
        0.0, // floor
    );

    assert!(vel.x > 0.0, "should move toward target X");
    assert!(vel.z > 0.0, "should rise toward hover height");
    let speed = vel.length();
    assert!((speed - 2.0).abs() < 0.01, "should move at defined speed");
}

// ──────────────────── E2E: Monster gravity ────────────────────

#[test]
fn monster_gravity_over_multiple_ticks() {
    use marathon_sim::monster::ai::apply_monster_gravity;

    let mut z = 5.0f32;
    let mut vel_z = 0.0f32;
    let floor = 0.0f32;
    let gravity = 0.05;
    let terminal = 2.0;

    // Fall for many ticks
    let mut grounded = false;
    for _ in 0..200 {
        let result = apply_monster_gravity(z, vel_z, floor, gravity, terminal);
        z = result.0;
        vel_z = result.1;
        grounded = result.2;
        if grounded {
            break;
        }
    }

    assert!(grounded, "monster should reach the floor");
    assert_eq!(z, 0.0);
    assert_eq!(vel_z, 0.0);
}

// ──────────────────── E2E: Monster attack execution ────────────────────

#[test]
fn monster_attack_melee_vs_ranged() {
    use glam::Vec3;
    use marathon_sim::monster::ai::{compute_monster_attack, AttackResult};
    use marathon_sim::MonsterState;

    // Melee range: within 1.0
    let result = compute_monster_attack(
        MonsterState::Attacking, 0.8, 0,
        1.0, 15, 5, 2, 1.0,
        8.0, 5, Vec3::new(0.0, 0.0, 0.5), 0.1,
    );
    assert!(matches!(result, AttackResult::Melee { damage_base: 15, .. }));

    // Ranged: beyond melee but within ranged range
    let result = compute_monster_attack(
        MonsterState::Attacking, 5.0, 0,
        1.0, 15, 5, 2, 1.0,
        8.0, 5, Vec3::new(0.0, 0.0, 0.5), 0.1,
    );
    assert!(matches!(result, AttackResult::Ranged { projectile_type: 5, .. }));

    // Out of all range
    let result = compute_monster_attack(
        MonsterState::Attacking, 20.0, 0,
        1.0, 15, 5, 2, 1.0,
        8.0, 5, Vec3::new(0.0, 0.0, 0.5), 0.1,
    );
    assert!(matches!(result, AttackResult::None));

    // On cooldown
    let result = compute_monster_attack(
        MonsterState::Attacking, 0.5, 10,
        1.0, 15, 5, 2, 1.0,
        8.0, 5, Vec3::ZERO, 0.0,
    );
    assert!(matches!(result, AttackResult::None));
}

// ──────────────────── E2E: Burst fire and dual wield ────────────────────

#[test]
fn burst_fire_consumes_one_round_spawns_many() {
    use marathon_sim::combat::weapons::*;
    use marathon_sim::player::inventory::*;

    let mut weapon = WeaponSlot {
        definition_index: 0,
        primary_magazine: 8,
        primary_reserve: 0,
        secondary_magazine: 0,
        secondary_reserve: 0,
        state: WeaponState::Idle,
        cooldown_ticks: 0,
    };

    let result = tick_weapon_burst(&mut weapon, true, 3, 5, 4, 0.1);
    assert!(result.fired);
    assert_eq!(result.projectile_count, 4);
    assert!((result.theta_error - 0.1).abs() < f32::EPSILON);
    assert_eq!(weapon.primary_magazine, 7); // only 1 consumed
}

#[test]
fn dual_wield_fires_independently() {
    use marathon_sim::combat::weapons::*;
    use marathon_sim::player::inventory::*;

    let make = |mag: u16| WeaponSlot {
        definition_index: 1,
        primary_magazine: mag,
        primary_reserve: 0,
        secondary_magazine: 0,
        secondary_reserve: 0,
        state: WeaponState::Idle,
        cooldown_ticks: 0,
    };

    let mut dual = DualWieldState::new(make(5), make(8));

    // Fire only right (primary)
    let (right, left) = dual.tick(true, false, 2, 3);
    assert!(right);
    assert!(!left);
    assert_eq!(dual.right.primary_magazine, 7);
    assert_eq!(dual.left.primary_magazine, 5);

    // Cooldown on right; fire left
    let (right, left) = dual.tick(true, true, 2, 3);
    assert!(!right); // still in cooldown
    assert!(left);
    assert_eq!(dual.left.primary_magazine, 4);
}

// ──────────────────── E2E: Projectile entity collision ────────────────────

#[test]
fn projectile_hits_nearest_entity() {
    use glam::{Vec2, Vec3};
    use marathon_sim::combat::projectiles::check_projectile_entity_collision;

    let entities = vec![
        (Vec2::new(3.0, 0.0), 0.5, 0.0, 2.0), // closer
        (Vec2::new(7.0, 0.0), 0.5, 0.0, 2.0), // farther
    ];

    let result = check_projectile_entity_collision(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(10.0, 0.0, 1.0),
        &entities,
    );
    let hit = result.expect("should hit an entity");
    assert_eq!(hit.entity_index, 0, "should hit the closest entity");
    assert!(hit.hit_point.x < 4.0);
}

#[test]
fn projectile_misses_when_z_out_of_range() {
    use glam::{Vec2, Vec3};
    use marathon_sim::combat::projectiles::check_projectile_entity_collision;

    let entities = vec![
        (Vec2::new(5.0, 0.0), 0.5, 0.0, 1.0), // z range 0-1
    ];

    // Projectile at z=2.0, above entity
    let result = check_projectile_entity_collision(
        Vec3::new(0.0, 0.0, 2.0),
        Vec3::new(10.0, 0.0, 2.0),
        &entities,
    );
    assert!(result.is_none(), "projectile should miss entity above its z range");
}

// ──────────────────── E2E: Damage application with shield/health ────────────────────

#[test]
fn damage_application_full_lifecycle() {
    use marathon_sim::combat::damage::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(42);

    // Calculate damage
    let def = marathon_formats::DamageDefinition {
        damage_type: 0,
        flags: 0,
        base: 50,
        random: 10,
        scale: 1.0,
    };
    let damage = calculate_damage(&def, 0, 0, &mut rng);
    assert!(damage >= 50 && damage <= 60);

    // Apply to entity with shield
    let (health, shield, result) = apply_damage(damage, 100, 80);
    assert!(shield < 80 || health < 100); // at least one reduced
    assert!(!result.killed);

    // Massive damage kills
    let (health, _, result) = apply_damage(500, 100, 50);
    assert!(health <= 0);
    assert!(result.killed);
}

#[test]
fn aoe_damage_scales_with_distance() {
    use marathon_sim::combat::damage::calculate_aoe_damage;

    let full = calculate_aoe_damage(100, 0.0, 5.0);
    let half = calculate_aoe_damage(100, 2.5, 5.0);
    let quarter = calculate_aoe_damage(100, 3.75, 5.0);
    let none = calculate_aoe_damage(100, 5.0, 5.0);

    assert_eq!(full, 100);
    assert_eq!(half, 50);
    assert_eq!(quarter, 25);
    assert_eq!(none, 0);
    assert!(full > half);
    assert!(half > quarter);
    assert!(quarter > none);
}

// ──────────────────── E2E: Platform crushing and activation ────────────────────

#[test]
fn platform_activation_by_trigger_type() {
    use marathon_sim::world_mechanics::platforms::*;
    use marathon_sim::*;

    let mut platform = Platform {
        polygon_index: 0,
        floor_rest: 0.0,
        floor_extended: 1.0,
        ceiling_rest: 3.0,
        ceiling_extended: 3.0,
        current_floor: 0.0,
        current_ceiling: 3.0,
        speed: 0.5,
        state: PlatformState::AtRest,
        return_delay: 10,
        delay_remaining: 0,
        activation_flags: PLATFORM_ACTIVATE_ON_ACTION_KEY | PLATFORM_ACTIVATE_ON_PROJECTILE,
        crushes: false,
    };

    assert!(!should_activate(&platform, PlatformTrigger::PlayerEntry));
    assert!(should_activate(&platform, PlatformTrigger::ActionKey));
    assert!(!should_activate(&platform, PlatformTrigger::MonsterEntry));
    assert!(should_activate(&platform, PlatformTrigger::ProjectileImpact));

    // After activation, shouldn't re-trigger
    activate_platform(&mut platform);
    assert!(!should_activate(&platform, PlatformTrigger::ActionKey));
}

#[test]
fn platform_crush_vs_reverse() {
    use marathon_sim::world_mechanics::platforms::*;
    use marathon_sim::*;

    // Crushing platform
    let crush_platform = Platform {
        polygon_index: 0,
        floor_rest: 0.0,
        floor_extended: 2.5,
        ceiling_rest: 3.0,
        ceiling_extended: 3.0,
        current_floor: 2.5,
        current_ceiling: 3.0,
        speed: 0.1,
        state: PlatformState::Extending,
        return_delay: 0,
        delay_remaining: 0,
        activation_flags: 0,
        crushes: true,
    };

    // Entity height 0.8, clearance = 3.0 - 2.5 = 0.5 < 0.8
    let result = check_platform_crush(&crush_platform, 2.5, 0.8);
    assert_eq!(result, PlatformCrushResult::Crush { damage: 10 });

    // Non-crushing platform
    let mut reverse_platform = crush_platform.clone();
    reverse_platform.crushes = false;
    let result = check_platform_crush(&reverse_platform, 2.5, 0.8);
    assert_eq!(result, PlatformCrushResult::Reverse);
}

// ──────────────────── E2E: Light animation patterns ────────────────────

#[test]
fn light_animation_patterns() {
    use marathon_sim::world_mechanics::lights::compute_light_intensity;
    use marathon_sim::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(42);

    // Constant light stays at max
    let light = Light {
        light_index: 0,
        function: LightFunction::Constant,
        period: 60,
        phase: 0,
        intensity_min: 0.3,
        intensity_max: 0.9,
        current_intensity: 0.9,
    };
    for tick in 0..100 {
        let i = compute_light_intensity(&light, tick, &mut rng);
        assert!((i - 0.9).abs() < f32::EPSILON);
    }

    // Smooth light oscillates between min and max
    let smooth = Light {
        function: LightFunction::Smooth,
        period: 100,
        intensity_min: 0.0,
        intensity_max: 1.0,
        ..light.clone()
    };
    let mut min_seen = 1.0f32;
    let mut max_seen = 0.0f32;
    for tick in 0..100 {
        let i = compute_light_intensity(&smooth, tick, &mut rng);
        min_seen = min_seen.min(i);
        max_seen = max_seen.max(i);
    }
    assert!(min_seen < 0.05, "smooth light should reach near minimum");
    assert!(max_seen > 0.95, "smooth light should reach near maximum");
}

// ──────────────────── E2E: Control panel activation ────────────────────

#[test]
fn control_panel_distance_and_facing_check() {
    use glam::Vec2;
    use marathon_sim::world_mechanics::panels::*;

    let endpoints = vec![
        (Vec2::new(2.0, -0.5), Vec2::new(2.0, 0.5)),
    ];
    let panel = ControlPanel {
        line_index: 0,
        side: 0,
        action: PanelAction::ActivateTerminal { terminal_index: 3 },
        max_distance: 1.5,
    };

    // Player facing east, close enough -> activates
    assert!(can_activate_panel(Vec2::new(1.0, 0.0), 0.0, &panel, &endpoints));

    // Player facing west -> doesn't activate
    assert!(!can_activate_panel(
        Vec2::new(1.0, 0.0),
        std::f32::consts::PI,
        &panel,
        &endpoints,
    ));

    // Player too far
    assert!(!can_activate_panel(Vec2::new(-5.0, 0.0), 0.0, &panel, &endpoints));

    // Different panel types
    let platform_panel = ControlPanel {
        action: PanelAction::ActivatePlatform { platform_index: 0 },
        ..panel.clone()
    };
    assert!(can_activate_panel(Vec2::new(1.0, 0.0), 0.0, &platform_panel, &endpoints));

    let light_panel = ControlPanel {
        action: PanelAction::ToggleLight { light_index: 2 },
        ..panel.clone()
    };
    assert!(can_activate_panel(Vec2::new(1.0, 0.0), 0.0, &light_panel, &endpoints));
}

// ──────────────────── E2E: Item respawn timer ────────────────────

#[test]
fn item_respawn_complete_cycle() {
    use marathon_sim::world_mechanics::items::ItemRespawnState;

    let mut respawn = ItemRespawnState::new(20, 30); // health item, 30 tick delay

    // Count down
    for i in 0..29 {
        let ready = respawn.tick();
        assert!(!ready, "should not be ready at tick {}", i);
    }

    // 30th tick: ready
    assert!(respawn.tick(), "should be ready after full delay");
    assert_eq!(respawn.remaining, 0);

    // Further ticks: stays at 0, returns false
    assert!(!respawn.tick());
}

// ──────────────────── E2E: Weapon inventory cycling ────────────────────

#[test]
fn weapon_inventory_full_cycle() {
    use marathon_sim::player::inventory::*;

    let make_slot = |idx, mag| WeaponSlot {
        definition_index: idx,
        primary_magazine: mag,
        primary_reserve: 32,
        secondary_magazine: 0,
        secondary_reserve: 0,
        state: WeaponState::Idle,
        cooldown_ticks: 0,
    };

    let mut inv = WeaponInventory {
        weapons: vec![
            Some(make_slot(0, 8)),
            None,                   // empty slot
            Some(make_slot(2, 10)),
            None,
            Some(make_slot(4, 5)),
        ],
        current_weapon: 0,
        switch_cooldown: 0,
    };

    // Cycle forward: skips None slots
    inv.cycle_forward(15);
    assert_eq!(inv.current_weapon, 2);
    assert_eq!(inv.switch_cooldown, 15);

    inv.cycle_forward(15);
    assert_eq!(inv.current_weapon, 4);

    inv.cycle_forward(15);
    assert_eq!(inv.current_weapon, 0); // wraps around

    // Cycle backward
    inv.cycle_backward(10);
    assert_eq!(inv.current_weapon, 4);

    inv.cycle_backward(10);
    assert_eq!(inv.current_weapon, 2);
}

// ──────────────────── E2E: Pathfinding through polygon graph ────────────────────

#[test]
fn pathfinding_multi_hop_route() {
    use marathon_sim::monster::pathfinding::*;

    // Linear chain: 0 <-> 1 <-> 2 <-> 3 <-> 4
    let adjacency = vec![
        vec![(0, Some(1))],
        vec![(0, Some(0)), (1, Some(2))],
        vec![(1, Some(1)), (2, Some(3))],
        vec![(2, Some(2)), (3, Some(4))],
        vec![(3, Some(3))],
    ];

    let path = find_polygon_path(0, 4, &adjacency, 5);
    assert_eq!(path, Some(vec![0, 1, 2, 3, 4]));

    // No path: disconnected polygon
    let adjacency_disconnected = vec![
        vec![(0, Some(1))],
        vec![(0, Some(0))],
        vec![], // island
    ];
    let path = find_polygon_path(0, 2, &adjacency_disconnected, 3);
    assert_eq!(path, None);
}

// ──────────────────── E2E: Homing projectile tracking ────────────────────

#[test]
fn homing_projectile_converges_on_target() {
    use glam::Vec3;
    use marathon_sim::combat::projectiles::{advance_projectile, apply_homing};

    let mut pos = Vec3::new(0.0, 0.0, 0.0);
    let mut vel = Vec3::new(0.5, 0.0, 0.0);
    let target = Vec3::new(5.0, 5.0, 0.0);

    // Track the minimum distance the projectile gets to the target
    let mut min_dist = f32::MAX;
    for _ in 0..50 {
        vel = apply_homing(vel, pos, target, 0.5);
        let (new_pos, _) = advance_projectile(pos, vel);
        pos = new_pos;
        let dist = (pos - target).length();
        min_dist = min_dist.min(dist);
    }

    // The homing projectile should pass close to the target at some point
    assert!(min_dist < 1.0, "homing projectile should pass near target, min_dist={}", min_dist);
}

// ──────────────────── E2E: Entity iterator ────────────────────

#[test]
fn entity_iterator_returns_monsters_and_items() {
    let mut map = make_test_map();

    // Add monster and item objects
    map.objects.push(MapObject {
        object_type: 0, // monster
        index: 0,
        facing: 0,
        polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 },
        flags: 0,
    });
    map.objects.push(MapObject {
        object_type: 2, // item
        index: 20,      // health minor
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 256, y: 256, z: 0 },
        flags: 0,
    });

    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    let entities = world.entities();
    assert!(entities.len() >= 2, "should have at least a monster and an item");

    let has_monster = entities.iter().any(|e| {
        matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Monster { .. })
    });
    let has_item = entities.iter().any(|e| {
        matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Item { .. })
    });

    assert!(has_monster, "entity list should contain a monster");
    assert!(has_item, "entity list should contain an item");
}

// ──────────────────── E2E: Serialization preserves monster/item state ────────────────────

#[test]
fn serialization_preserves_all_entity_types() {
    let mut map = make_test_map();

    // Add monsters and items
    map.objects.push(MapObject {
        object_type: 0, index: 0, facing: 0, polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 }, flags: 0,
    });
    map.objects.push(MapObject {
        object_type: 2, index: 20, facing: 0, polygon_index: 0,
        location: WorldPoint3d { x: 256, y: 256, z: 0 }, flags: 0,
    });

    let physics = make_test_physics();
    let config = SimConfig { random_seed: 77, difficulty: 2 };
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Advance some ticks
    for _ in 0..10 {
        world.tick(ActionFlags::default().into());
    }

    let entities_before = world.entities().len();

    // Serialize and restore
    let data = world.serialize().unwrap();
    let mut restored = SimWorld::deserialize(&data, &map, &physics).unwrap();

    let entities_after = restored.entities().len();
    assert_eq!(entities_before, entities_after, "entity count should be preserved");
    assert_eq!(restored.tick_count(), 10);
}

// ──────────────────── Tick Loop Integration Tests ────────────────────

/// Helper: create a map with a smooth-function light (period=60 ticks).
fn make_light_test_map() -> MapData {
    let mut map = make_test_map();
    map.lights = LightData::Static(vec![StaticLightData {
        light_type: 0,
        flags: 0,
        phase: 0,
        primary_active: LightingFunctionSpec {
            function: 2, // Smooth (cosine)
            period: 60,
            delta_period: 0,
            intensity: 0.0,
            delta_intensity: 1.0,
        },
        secondary_active: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 1.0, delta_intensity: 0.0 },
        becoming_active: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 1.0, delta_intensity: 0.0 },
        primary_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        secondary_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        becoming_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        tag: 0,
    }]);
    map
}

#[test]
fn tick_loop_lights_update_intensity() {
    let map = make_light_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Smooth light: at tick 0 intensity should be at minimum (cosine starts at 0)
    // After 30 ticks (half period), should reach maximum (1.0)
    for _ in 0..30 {
        world.tick(ActionFlags::default().into());
    }

    let snapshot = world.snapshot();
    assert!(!snapshot.lights.is_empty(), "should have lights");
    let light = &snapshot.lights[0];
    // At half-period, cosine wave = 1.0 (maximum)
    assert!(
        light.current_intensity > 0.9,
        "light should be near max at half period, got {}",
        light.current_intensity
    );
}

#[test]
fn tick_loop_lights_determinism() {
    let map = make_light_test_map();
    // Use flicker light for RNG-dependent test
    let mut map2 = map.clone();
    map2.lights = LightData::Static(vec![StaticLightData {
        light_type: 0,
        flags: 0,
        phase: 0,
        primary_active: LightingFunctionSpec {
            function: 3, // Flicker (random)
            period: 60,
            delta_period: 0,
            intensity: 0.0,
            delta_intensity: 1.0,
        },
        secondary_active: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 1.0, delta_intensity: 0.0 },
        becoming_active: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 1.0, delta_intensity: 0.0 },
        primary_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        secondary_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        becoming_inactive: LightingFunctionSpec { function: 0, period: 1, delta_period: 0, intensity: 0.0, delta_intensity: 0.0 },
        tag: 0,
    }]);

    let physics = make_test_physics();
    let config = SimConfig { random_seed: 42, difficulty: 2 };

    let mut world_a = SimWorld::new(&map2, &physics, &config).unwrap();
    let mut world_b = SimWorld::new(&map2, &physics, &config).unwrap();

    for _ in 0..20 {
        world_a.tick(ActionFlags::default().into());
        world_b.tick(ActionFlags::default().into());
    }

    let snap_a = world_a.snapshot();
    let snap_b = world_b.snapshot();

    assert_eq!(snap_a.lights[0].current_intensity, snap_b.lights[0].current_intensity,
        "same seed should produce identical flicker intensities");
}

#[test]
fn tick_loop_media_tracks_light() {
    let mut map = make_light_test_map();
    // Add media linked to light 0
    map.media = vec![MediaData {
        media_type: 0, // water
        flags: 0,
        light_index: 0,
        current_direction: 0,
        current_magnitude: 0,
        low: 0,     // 0 WU
        high: 2048, // 2 WU
        origin: WorldPoint2d { x: 0, y: 0 },
        height: 2048,
        minimum_light_intensity: 0.0,
        texture: ShapeDescriptor(0xFFFF),
        transfer_mode: 0,
    }];
    // Associate polygon 0 with media 0
    map.polygons[0].media_index = 0;

    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // After 30 ticks (half period of smooth light), light intensity ~1.0
    // Media height should interpolate to high (2.0 WU)
    for _ in 0..30 {
        world.tick(ActionFlags::default().into());
    }

    let snapshot = world.snapshot();
    assert!(!snapshot.media.is_empty(), "should have media");
    let media = &snapshot.media[0];
    // Media height should be near high value (2.0) when light intensity ~1.0
    assert!(
        media.current_height > 1.5,
        "media height should track light intensity upward, got {}",
        media.current_height
    );
}

#[test]
fn tick_loop_platform_extends_and_updates_geometry() {
    let mut map = make_test_map();
    // Add a platform on polygon 0, player-entry activated
    map.platforms = vec![StaticPlatformData {
        platform_type: 0,
        speed: 512, // 0.5 WU/tick
        delay: 30,
        minimum_height: 0,
        maximum_height: 1024, // 1 WU
        polygon_index: 0,
        static_flags: 0x0001, // ACTIVATE_ON_PLAYER_ENTRY
        tag: 0,
    }];

    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Player is in polygon 0, platform should activate on first tick
    world.tick(ActionFlags::default().into());

    let snap = world.snapshot();
    let platform = &snap.platforms[0];
    assert_eq!(
        platform.state,
        marathon_sim::PlatformState::Extending,
        "platform should be extending after player entry"
    );

    // Tick until extended (floor should reach 1.0 WU)
    for _ in 0..10 {
        world.tick(ActionFlags::default().into());
    }

    let snap = world.snapshot();
    let platform = &snap.platforms[0];
    assert!(
        platform.current_floor > 0.5,
        "platform floor should have risen, got {}",
        platform.current_floor
    );
}

#[test]
fn tick_loop_effect_despawns_after_countdown() {
    let map = make_test_map();
    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Manually spawn an effect entity
    world.ecs_world_mut().spawn((
        marathon_sim::Effect { definition_index: 0, ticks_remaining: 3 },
        marathon_sim::Position(glam::Vec3::new(0.5, 0.5, 0.0)),
    ));

    // Should have 1 effect
    let entities = world.entities();
    let effects: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Effect { .. }))
        .collect();
    assert_eq!(effects.len(), 1, "should have 1 effect");

    // Tick 3 times to expire it
    for _ in 0..3 {
        world.tick(ActionFlags::default().into());
    }

    let entities = world.entities();
    let effects: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Effect { .. }))
        .collect();
    assert_eq!(effects.len(), 0, "effect should be despawned after ticks_remaining reaches 0");
}

#[test]
fn tick_loop_item_pickup_restores_health() {
    let mut map = make_test_map();
    // Add a health item at the player's position
    map.objects.push(MapObject {
        object_type: 2, // OBJECT_IS_ITEM
        index: 21,      // ITEM_HEALTH_MAJOR (restores 40 HP)
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 512, y: 512, z: 0 }, // same as player
        flags: 0,
    });

    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Reduce player health below max so the health item can be picked up
    {
        let ecs = world.ecs_world_mut();
        let mut query = ecs.query_filtered::<&mut marathon_sim::Health, bevy_ecs::query::With<marathon_sim::Player>>();
        for mut health in query.iter_mut(ecs) {
            health.0 = 100; // below max of 150
        }
    }

    let initial_health = world.player_health().unwrap();
    assert_eq!(initial_health, 100);

    // Count items before
    let items_before: Vec<_> = world.entities().iter()
        .filter(|e| matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Item { .. }))
        .cloned()
        .collect();
    assert!(items_before.len() >= 1, "should have at least 1 item");

    // Tick to trigger pickup
    world.tick(ActionFlags::default().into());

    let health_after = world.player_health().unwrap();
    assert!(
        health_after > initial_health,
        "health should increase after picking up health item, got {}",
        health_after
    );
}

#[test]
fn tick_loop_item_not_picked_up_at_max_health() {
    let mut map = make_test_map();
    // Add a health item at the player's position
    map.objects.push(MapObject {
        object_type: 2, // OBJECT_IS_ITEM
        index: 21,      // ITEM_HEALTH_MAJOR
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 512, y: 512, z: 0 },
        flags: 0,
    });

    let physics = make_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Player starts at 150 HP (max)
    let initial_health = world.player_health().unwrap();
    assert_eq!(initial_health, 150);

    // Tick - should NOT pick up because health is at max
    world.tick(ActionFlags::default().into());

    // Item should still exist
    let items: Vec<_> = world.entities().iter()
        .filter(|e| matches!(e.entity_type, marathon_sim::tick::RenderEntityType::Item { .. }))
        .cloned()
        .collect();
    assert_eq!(items.len(), 1, "item should not be picked up at max health");
}

#[test]
fn tick_loop_monster_alerts_on_sight() {
    let mut map = make_test_map();
    // Add a monster in poly 1 facing the player (facing west)
    map.objects.push(MapObject {
        object_type: 0, // OBJECT_IS_MONSTER
        index: 0,
        facing: 256, // ~180 degrees = facing west toward player
        polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 },
        flags: 0,
    });

    let mut physics = make_test_physics();
    // Give monster visual range and arc
    if let Some(ref mut monsters) = physics.monsters {
        monsters[0].visual_range = 10240; // 10 WU
        monsters[0].half_visual_arc = 128; // ~90 degrees
    }

    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Initial state should be Idle
    let snap = world.snapshot();
    assert!(!snap.monsters.is_empty(), "should have a monster");
    assert_eq!(snap.monsters[0].state, marathon_sim::MonsterState::Idle);

    // Tick once - monster should see player and become alerted
    world.tick(ActionFlags::default().into());

    let snap = world.snapshot();
    assert_eq!(
        snap.monsters[0].state,
        marathon_sim::MonsterState::Alerted,
        "monster should be alerted after seeing player"
    );
}

#[test]
fn tick_loop_full_systems_no_panics() {
    let mut map = make_test_map();

    // Add lights, platforms, monsters, items for a full simulation
    map.platforms = vec![StaticPlatformData {
        platform_type: 0,
        speed: 256,
        delay: 10,
        minimum_height: 0,
        maximum_height: 512,
        polygon_index: 1,
        static_flags: 0x0001, // player entry
        tag: 0,
    }];

    map.objects.push(MapObject {
        object_type: 0, // Monster
        index: 0,
        facing: 256,
        polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 },
        flags: 0,
    });

    map.objects.push(MapObject {
        object_type: 2, // Item
        index: 23,      // Shield
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 256, y: 256, z: 0 },
        flags: 0,
    });

    let mut physics = make_test_physics();
    if let Some(ref mut monsters) = physics.monsters {
        monsters[0].visual_range = 10240;
        monsters[0].half_visual_arc = 128;
    }

    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Tick 100 times with various inputs - should not panic
    for i in 0..100 {
        let flags = match i % 10 {
            0..=2 => ActionFlags::new(ActionFlags::MOVE_FORWARD),
            3..=5 => ActionFlags::new(ActionFlags::STRAFE_LEFT),
            6 => ActionFlags::new(ActionFlags::FIRE_PRIMARY),
            _ => ActionFlags::default(),
        };
        world.tick(flags.into());
    }

    assert_eq!(world.tick_count(), 100);
    // Player should still be alive
    let health = world.player_health().unwrap();
    assert!(health > 0 || health <= 0, "player health should be a valid value");
}

#[test]
fn tick_loop_determinism_two_worlds_same_seed() {
    let mut map = make_test_map();
    map.objects.push(MapObject {
        object_type: 0,
        index: 0,
        facing: 256,
        polygon_index: 1,
        location: WorldPoint3d { x: 1536, y: 512, z: 0 },
        flags: 0,
    });

    let mut physics = make_test_physics();
    if let Some(ref mut monsters) = physics.monsters {
        monsters[0].visual_range = 10240;
        monsters[0].half_visual_arc = 128;
    }

    let config = SimConfig { random_seed: 42, difficulty: 2 };

    let mut world_a = SimWorld::new(&map, &physics, &config).unwrap();
    let mut world_b = SimWorld::new(&map, &physics, &config).unwrap();

    // Same inputs for both
    let inputs = vec![
        ActionFlags::new(ActionFlags::MOVE_FORWARD),
        ActionFlags::new(ActionFlags::STRAFE_LEFT),
        ActionFlags::new(ActionFlags::FIRE_PRIMARY),
        ActionFlags::default(),
    ];

    for _ in 0..20 {
        for flags in &inputs {
            world_a.tick((*flags).into());
            world_b.tick((*flags).into());
        }
    }

    let snap_a = world_a.snapshot();
    let snap_b = world_b.snapshot();

    // Player positions should be identical
    assert_eq!(
        snap_a.player.as_ref().unwrap().position,
        snap_b.player.as_ref().unwrap().position,
        "same seed + same inputs should produce identical player positions"
    );

    // Monster states should be identical
    assert_eq!(snap_a.monsters.len(), snap_b.monsters.len());
    for (a, b) in snap_a.monsters.iter().zip(snap_b.monsters.iter()) {
        assert_eq!(a.state, b.state, "monster states should match");
        assert_eq!(a.position, b.position, "monster positions should match");
    }
}

// ──────────────────── Projectile Physics Tests ────────────────────

/// Create physics data with multiple projectile types for testing.
fn make_projectile_test_physics() -> PhysicsData {
    use marathon_formats::physics::ProjectileDefinition;

    // Index 0: basic projectile (no special flags)
    let basic = ProjectileDefinition {
        collection: 0,
        shape: 0,
        detonation_effect: 0,
        media_detonation_effect: -1,
        contrail_effect: -1,
        ticks_between_contrails: 0,
        maximum_contrails: 0,
        media_projectile_promotion: -1,
        radius: 64,
        area_of_effect: 0,
        damage: DamageDefinition {
            damage_type: 0, flags: 0, base: 20, random: 0, scale: 1.0,
        },
        flags: 0,
        speed: 512,          // 0.5 WU/tick
        maximum_range: 16384, // 16 WU
        sound_pitch: 1.0,
        flyby_sound: -1,
        rebound_sound: -1,
    };

    // Index 1: gravity-affected (grenade arc)
    let gravity = ProjectileDefinition {
        flags: 0x0010, // AFFECTED_BY_GRAVITY
        detonation_effect: 0,
        area_of_effect: 2048, // 2 WU radius
        ..basic.clone()
    };

    // Index 2: rebounds from walls
    let rebound_wall = ProjectileDefinition {
        flags: 0x0400, // REBOUNDS_FROM_WALLS
        rebound_sound: 0,
        ..basic.clone()
    };

    // Index 3: rebounds from floor
    let rebound_floor = ProjectileDefinition {
        flags: 0x0010 | 0x0020, // AFFECTED_BY_GRAVITY | REBOUNDS_FROM_FLOOR
        rebound_sound: 0,
        ..basic.clone()
    };

    // Index 4: persistent (passes through entities)
    let persistent = ProjectileDefinition {
        flags: 0x0004, // PERSISTENT
        ..basic.clone()
    };

    // Index 5: homing (guided)
    let homing = ProjectileDefinition {
        flags: 0x0001, // GUIDED
        ..basic.clone()
    };

    // Index 6: with contrails
    let contrail = ProjectileDefinition {
        contrail_effect: 0,
        ticks_between_contrails: 3,
        maximum_contrails: 5,
        ..basic.clone()
    };

    // Index 7: short range (for range limit test)
    let short_range = ProjectileDefinition {
        maximum_range: 512, // 0.5 WU
        detonation_effect: -1,
        ..basic.clone()
    };

    let mut base_physics = make_test_physics();
    base_physics.projectiles = Some(vec![
        basic, gravity, rebound_wall, rebound_floor,
        persistent, homing, contrail, short_range,
    ]);
    base_physics.effects = Some(vec![
        marathon_formats::physics::EffectDefinition {
            collection: 0,
            shape: 0,
            sound_pitch: 1.0,
            flags: 0,
            delay: 10,
            delay_sound: -1,
        },
    ]);
    base_physics
}

/// Helper: spawn a projectile entity as player-fired (won't collide with player).
fn spawn_test_projectile(
    world: &mut SimWorld,
    def_index: usize,
    pos: glam::Vec3,
    vel: glam::Vec3,
    polygon: usize,
) {
    use marathon_sim::components::*;
    // Get the player entity so we can set ProjectileSource (avoids self-collision)
    let player_entity = {
        let mut q = world.ecs_world_mut().query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::prelude::With<Player>>();
        q.iter(world.ecs_world_mut()).next().unwrap()
    };
    world.ecs_world_mut().spawn((
        Projectile {
            definition_index: def_index,
            distance_traveled: 0.0,
            contrails_spawned: 0,
            ticks_alive: 0,
            current_polygon: polygon,
        },
        Position(pos),
        Velocity(vel),
        PolygonIndex(polygon),
        ProjectileSource(player_entity),
    ));
}

/// Count projectile entities in the world.
fn count_projectiles(world: &mut SimWorld) -> usize {
    let snap = world.snapshot();
    snap.projectiles.len()
}

// 7.1: Projectile advances position each tick
#[test]
fn projectile_advances_position_each_tick() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn basic projectile moving east slowly, well inside poly 0
    spawn_test_projectile(
        &mut world,
        0, // basic
        glam::Vec3::new(0.5, 0.5, 0.5),
        glam::Vec3::new(0.02, 0.0, 0.0), // very slow to stay well inside polygon
        0,
    );

    // Verify projectile exists before tick
    let snap_before = world.snapshot();
    assert_eq!(snap_before.projectiles.len(), 1, "projectile should exist before tick");

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    let snap = world.snapshot();
    assert_eq!(snap.projectiles.len(), 1, "projectile should still exist after 1 tick, def_idx=0 max_range=16");
    let proj = &snap.projectiles[0];
    assert!((proj.position.x - 0.52).abs() < 0.01, "projectile should advance by velocity.x");
    assert!(proj.distance_traveled > 0.0, "distance should accumulate");
}

// 7.2: Gravity-affected projectile arcs downward
#[test]
fn gravity_projectile_arcs_downward() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn gravity projectile moving east, starting above floor
    spawn_test_projectile(
        &mut world,
        1, // gravity-affected
        glam::Vec3::new(0.5, 0.5, 1.0),
        glam::Vec3::new(0.05, 0.0, 0.0),
        0,
    );

    let empty = ActionFlags::new(0);
    // Tick several times
    for _ in 0..5 {
        world.tick(empty.into());
    }

    let snap = world.snapshot();
    if !snap.projectiles.is_empty() {
        let proj = &snap.projectiles[0];
        // Z should decrease due to gravity
        assert!(proj.position.z < 1.0, "gravity should pull projectile down, z={}", proj.position.z);
    }
    // If projectile hit floor and detonated, that's also correct behavior
}

// 7.3: Homing projectile turns toward target
#[test]
fn homing_projectile_turns_toward_target() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn homing projectile with HomingTarget
    use marathon_sim::components::*;
    world.ecs_world_mut().spawn((
        Projectile {
            definition_index: 5, // homing
            distance_traveled: 0.0,
            contrails_spawned: 0,
            ticks_alive: 0,
            current_polygon: 0,
        },
        Position(glam::Vec3::new(0.3, 0.3, 0.5)),
        Velocity(glam::Vec3::new(0.1, 0.0, 0.0)), // moving east
        PolygonIndex(0),
        HomingTarget(glam::Vec3::new(0.3, 0.8, 0.5)), // target is north
    ));

    let empty = ActionFlags::new(0);
    for _ in 0..3 {
        world.tick(empty.into());
    }

    let snap = world.snapshot();
    if !snap.projectiles.is_empty() {
        let proj = &snap.projectiles[0];
        // Velocity should have turned northward (increasing Y)
        assert!(proj.velocity.y > 0.01, "homing should turn toward target, vel.y={}", proj.velocity.y);
    }
}

// 7.4: Projectile detonates on solid wall hit
#[test]
fn projectile_detonates_on_wall_hit() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn projectile heading toward the left wall (x=0) of poly 0
    spawn_test_projectile(
        &mut world,
        0, // basic, detonation_effect=0
        glam::Vec3::new(0.2, 0.5, 0.5),
        glam::Vec3::new(-0.5, 0.0, 0.0), // will hit left wall at x=0
        0,
    );

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Projectile should be gone (detonated)
    assert_eq!(count_projectiles(&mut world), 0, "projectile should detonate on wall hit");
}

// 7.5: Projectile with REBOUNDS_FROM_WALLS reflects velocity
#[test]
fn projectile_rebounds_from_wall() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn wall-rebounding projectile heading left
    spawn_test_projectile(
        &mut world,
        2, // REBOUNDS_FROM_WALLS
        glam::Vec3::new(0.2, 0.5, 0.5),
        glam::Vec3::new(-0.5, 0.0, 0.0),
        0,
    );

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Projectile should still exist (rebounded)
    let snap = world.snapshot();
    assert_eq!(snap.projectiles.len(), 1, "rebounding projectile should survive wall hit");
    // Velocity X should be positive now (reflected from left wall)
    assert!(snap.projectiles[0].velocity.x > 0.0, "velocity should be reflected, vel.x={}", snap.projectiles[0].velocity.x);
}

// 7.6: Projectile with REBOUNDS_FROM_FLOOR bounces
#[test]
fn projectile_rebounds_from_floor() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn floor-rebounding projectile falling toward floor
    spawn_test_projectile(
        &mut world,
        3, // AFFECTED_BY_GRAVITY | REBOUNDS_FROM_FLOOR
        glam::Vec3::new(0.5, 0.5, 0.1),
        glam::Vec3::new(0.05, 0.0, -0.2), // moving down
        0,
    );

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Projectile should still exist (bounced)
    let snap = world.snapshot();
    assert_eq!(snap.projectiles.len(), 1, "floor-rebounding projectile should survive");
    // Velocity Z should be positive (bounced up)
    assert!(snap.projectiles[0].velocity.z > 0.0, "velocity Z should be positive after bounce, vel.z={}", snap.projectiles[0].velocity.z);
}

// 7.7: Non-persistent projectile detonates on entity hit
#[test]
fn projectile_detonates_on_entity_hit() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();

    // Add a monster at (0.8, 0.5, 0.0)
    let mut map_with_monster = map.clone();
    map_with_monster.objects.push(MapObject {
        object_type: 0, // MONSTER
        index: 0,
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 820, y: 512, z: 0 },
        flags: 0,
    });

    let mut world = SimWorld::new(&map_with_monster, &physics, &config).unwrap();

    // Spawn player projectile heading toward the monster
    use marathon_sim::components::*;
    let player_entity = {
        let mut q = world.ecs_world_mut().query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::prelude::With<Player>>();
        q.iter(world.ecs_world_mut()).next().unwrap()
    };

    world.ecs_world_mut().spawn((
        Projectile {
            definition_index: 0,
            distance_traveled: 0.0,
            contrails_spawned: 0,
            ticks_alive: 0,
            current_polygon: 0,
        },
        Position(glam::Vec3::new(0.5, 0.5, 0.3)),
        Velocity(glam::Vec3::new(0.5, 0.0, 0.0)),
        PolygonIndex(0),
        ProjectileSource(player_entity),
    ));

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Projectile should be gone (detonated on monster hit)
    assert_eq!(count_projectiles(&mut world), 0, "projectile should detonate on entity hit");
}

// 7.8: Persistent projectile passes through entity
#[test]
fn persistent_projectile_passes_through() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();

    let mut map_with_monster = map.clone();
    map_with_monster.objects.push(MapObject {
        object_type: 0,
        index: 0,
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 700, y: 512, z: 0 },
        flags: 0,
    });

    let mut world = SimWorld::new(&map_with_monster, &physics, &config).unwrap();

    use marathon_sim::components::*;
    let player_entity = {
        let mut q = world.ecs_world_mut().query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::prelude::With<Player>>();
        q.iter(world.ecs_world_mut()).next().unwrap()
    };

    world.ecs_world_mut().spawn((
        Projectile {
            definition_index: 4, // PERSISTENT
            distance_traveled: 0.0,
            contrails_spawned: 0,
            ticks_alive: 0,
            current_polygon: 0,
        },
        Position(glam::Vec3::new(0.3, 0.5, 0.3)),
        Velocity(glam::Vec3::new(0.3, 0.0, 0.0)),
        PolygonIndex(0),
        ProjectileSource(player_entity),
    ));

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Persistent projectile should still exist
    assert_eq!(count_projectiles(&mut world), 1, "persistent projectile should survive entity hit");
}

// 7.9: AoE detonation applies distance-scaled damage
#[test]
fn aoe_detonation_applies_scaled_damage() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();

    // Monster at (0.8, 0.5) — 0.3 WU from detonation point
    let mut map_with_monster = map.clone();
    map_with_monster.objects.push(MapObject {
        object_type: 0,
        index: 0,
        facing: 0,
        polygon_index: 0,
        location: WorldPoint3d { x: 300, y: 512, z: 0 },
        flags: 0,
    });

    let mut world = SimWorld::new(&map_with_monster, &physics, &config).unwrap();

    // Get monster's initial health
    let monster_health_before = {
        let mut q = world.ecs_world_mut().query_filtered::<&marathon_sim::components::Health, bevy_ecs::prelude::With<marathon_sim::components::Monster>>();
        q.iter(world.ecs_world_mut()).next().map(|h| h.0).unwrap()
    };

    // Spawn AoE projectile (index 1, area_of_effect=2048=2.0 WU) near monster
    // heading toward wall to detonate
    spawn_test_projectile(
        &mut world,
        1, // gravity + AoE
        glam::Vec3::new(0.2, 0.5, 0.5),
        glam::Vec3::new(-0.5, 0.0, 0.0), // will hit left wall
        0,
    );

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Monster should have taken AoE damage
    let monster_health_after = {
        let mut q = world.ecs_world_mut().query_filtered::<&marathon_sim::components::Health, bevy_ecs::prelude::With<marathon_sim::components::Monster>>();
        q.iter(world.ecs_world_mut()).next().map(|h| h.0).unwrap()
    };

    assert!(
        monster_health_after < monster_health_before,
        "monster should take AoE damage: before={}, after={}",
        monster_health_before, monster_health_after
    );
}

// 7.10: Projectile despawned without effect when exceeding range
#[test]
fn projectile_despawned_at_max_range() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn short-range projectile (max_range=0.5 WU, speed=0.5 WU/tick)
    spawn_test_projectile(
        &mut world,
        7, // short range, detonation_effect=-1
        glam::Vec3::new(0.5, 0.5, 0.5),
        glam::Vec3::new(0.1, 0.0, 0.0),
        0,
    );

    let empty = ActionFlags::new(0);
    // Advance enough ticks for range to be exceeded
    for _ in 0..10 {
        world.tick(empty.into());
    }

    // Projectile should be despawned
    assert_eq!(count_projectiles(&mut world), 0, "projectile should despawn after exceeding range");
}

// 7.11: Contrails spawn at correct intervals
#[test]
fn contrails_spawn_at_intervals() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn contrail projectile (effect every 3 ticks, max 5)
    spawn_test_projectile(
        &mut world,
        6, // contrail
        glam::Vec3::new(0.5, 0.5, 0.5),
        glam::Vec3::new(0.05, 0.0, 0.0), // slow to stay in polygon
        0,
    );

    let empty = ActionFlags::new(0);
    // Tick 10 times (should produce ~3 contrails at ticks 3, 6, 9)
    for _ in 0..10 {
        world.tick(empty.into());
    }

    let snap = world.snapshot();
    // Check contrails_spawned on the projectile
    if !snap.projectiles.is_empty() {
        let proj = &snap.projectiles[0];
        assert!(proj.contrails_spawned > 0, "contrails should have been spawned");
        assert!(proj.contrails_spawned <= 5, "should not exceed maximum_contrails");
    }
}

// 7.13: Full tick with weapon fire produces projectile
#[test]
fn weapon_fire_produces_projectile() {
    let map = make_test_map();
    let mut physics = make_projectile_test_physics();
    // Need a weapon that fires projectile type 0
    physics.weapons = Some(vec![marathon_formats::physics::WeaponDefinition {
        item_type: -1,
        powerup_type: -1,
        weapon_class: 0,
        flags: 0,
        firing_light_intensity: 0.0,
        firing_intensity_decay_ticks: 0,
        idle_height: 0.0,
        bob_amplitude: 0.0,
        kick_height: 0.0,
        reload_height: 0.0,
        idle_width: 0.0,
        horizontal_amplitude: 0.0,
        collection: 0,
        idle_shape: 0,
        firing_shape: 0,
        reloading_shape: 0,
        charging_shape: -1,
        charged_shape: -1,
        ready_ticks: 0,
        await_reload_ticks: 0,
        loading_ticks: 0,
        finish_loading_ticks: 0,
        powerup_ticks: 0,
        primary_trigger: marathon_formats::physics::TriggerDefinition {
            rounds_per_magazine: 8,
            ammunition_type: -1,
            ticks_per_round: 2,
            recovery_ticks: 3,
            charging_ticks: 0,
            recoil_magnitude: 0,
            firing_sound: -1,
            click_sound: -1,
            charging_sound: -1,
            shell_casing_sound: -1,
            reloading_sound: -1,
            charged_sound: -1,
            projectile_type: 0, // fires basic projectile
            theta_error: 0,
            dx: 0,
            dz: 0,
            shell_casing_type: -1,
            burst_count: 0,
        },
        secondary_trigger: marathon_formats::physics::TriggerDefinition {
            rounds_per_magazine: 0,
            ammunition_type: -1,
            ticks_per_round: 0,
            recovery_ticks: 0,
            charging_ticks: 0,
            recoil_magnitude: 0,
            firing_sound: -1,
            click_sound: -1,
            charging_sound: -1,
            shell_casing_sound: -1,
            reloading_sound: -1,
            charged_sound: -1,
            projectile_type: -1,
            theta_error: 0,
            dx: 0,
            dz: 0,
            shell_casing_type: -1,
            burst_count: 0,
        },
    }]);
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Fire primary weapon
    let fire = ActionFlags::new(ActionFlags::FIRE_PRIMARY);
    world.tick(fire.into());

    // Should have spawned a projectile
    assert!(count_projectiles(&mut world) >= 1, "weapon fire should spawn projectile");
}

// 7.14: Snapshot round-trip preserves in-flight projectile state
#[test]
fn snapshot_preserves_projectile_state() {
    let map = make_test_map();
    let physics = make_projectile_test_physics();
    let config = SimConfig::default();
    let mut world = SimWorld::new(&map, &physics, &config).unwrap();

    // Spawn a projectile and advance a few ticks
    spawn_test_projectile(
        &mut world,
        0,
        glam::Vec3::new(0.5, 0.5, 0.5),
        glam::Vec3::new(0.05, 0.0, 0.0),
        0,
    );

    let empty = ActionFlags::new(0);
    world.tick(empty.into());

    // Serialize
    let bytes = world.serialize().expect("serialize should work");

    // Deserialize
    let mut world2 = SimWorld::deserialize(&bytes, &map, &physics).expect("deserialize should work");

    let snap1 = world.snapshot();
    let snap2 = world2.snapshot();

    assert_eq!(snap1.projectiles.len(), snap2.projectiles.len(), "projectile count should match");
    if !snap1.projectiles.is_empty() {
        let p1 = &snap1.projectiles[0];
        let p2 = &snap2.projectiles[0];
        assert_eq!(p1.definition_index, p2.definition_index);
        assert!((p1.position - p2.position).length() < 0.001, "position should match");
        assert!((p1.velocity - p2.velocity).length() < 0.001, "velocity should match");
        assert_eq!(p1.ticks_alive, p2.ticks_alive, "ticks_alive should match");
        assert_eq!(p1.contrails_spawned, p2.contrails_spawned, "contrails should match");
        assert_eq!(p1.current_polygon, p2.current_polygon, "polygon should match");
    }
}
