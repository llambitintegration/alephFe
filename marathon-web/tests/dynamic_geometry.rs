//! Box 4.3 integration test: dynamic geometry flows through the data texture,
//! static geometry (the vertex/index buffers) does not.
//!
//! # Why this is a CPU-side assertion, not a real headless-wgpu readback
//!
//! Box 4.3 calls for a headless-wgpu test, but the `rust:slim` CI test runner
//! has **no GPU**, so creating a real `wgpu::Device`, uploading buffers, and
//! reading texels back is not runnable in CI. Instead we assert the CPU-side
//! data that *fully determines* the GPU behavior:
//!
//! * The packed [`PolyDynData`] buffer (`pack_poly_data(...)`) is the **exact
//!   byte payload** `write_poly_data_texture` uploads into the per-polygon data
//!   texture every frame (see `poly_data.rs::write_poly_data_texture`). If those
//!   bytes change, the data-texture entry for the door polygon changes.
//! * `build_level_mesh(&map, &poly_info).vertices` (cast to bytes) is the **exact
//!   byte payload** uploaded into the static vertex buffer; `.indices` likewise
//!   for the index buffer. `frame()` never recreates those buffers (box 4.2).
//!
//! So asserting on these CPU values is faithful to box 4.3's invariant:
//! *dynamics flow through the data texture; geometry stays static.*

use marathon_formats::map::*;
use marathon_formats::*;
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{PolyDynamicData, SimConfig, SimWorld};

/// Build a simple two-polygon test map (copied verbatim from
/// `marathon-sim/tests/integration.rs::make_test_map`, which is not exported
/// from the sim test crate).
///
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

/// Copied verbatim from `marathon-sim/tests/integration.rs::make_test_physics`.
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
            radius: 256, // 0.25 WU
            height: 768, // 0.75 WU
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

/// Build `poly_info` exactly as `render.rs` does (render.rs:1059-1073). This is
/// the static, sim-independent per-polygon data baked into the vertex buffer.
fn build_poly_info(map: &MapData) -> Vec<marathon_web::mesh::PolygonInfo> {
    map.polygons
        .iter()
        .map(|p| marathon_web::mesh::PolygonInfo {
            floor_light: marathon_web::level::evaluate_light_intensity(
                &map.lights,
                p.floor_lightsource_index,
            ),
            floor_transfer_mode: p.floor_transfer_mode as u32,
            ceiling_light: marathon_web::level::evaluate_light_intensity(
                &map.lights,
                p.ceiling_lightsource_index,
            ),
            ceiling_transfer_mode: p.ceiling_transfer_mode as u32,
        })
        .collect()
}

/// Pack a slice of sim per-polygon data into the exact `f32` payload that
/// `write_poly_data_texture` uploads to the data texture each frame.
fn pack_from_sim(dyn_data: &[PolyDynamicData]) -> Vec<f32> {
    marathon_web::poly_data::pack_poly_data(&marathon_web::poly_data::poly_dyn_data_from_sim_slice(
        dyn_data,
    ))
}

/// Box 4.3: ticking a sim with an opening door changes the door polygon's
/// data-texture entry, while the static vertex/index buffers stay byte-for-byte
/// identical.
#[test]
fn door_animation_changes_data_texture_not_vertex_buffer() {
    // --- Build a map with a player-entry-activated platform (door) on poly 0. ---
    let mut map = make_test_map();
    map.platforms = vec![StaticPlatformData {
        platform_type: 0,
        speed: 512,
        delay: 30,
        minimum_height: 0,
        maximum_height: 1024,
        polygon_index: 0,
        static_flags: 0x0001, // ACTIVATE_ON_PLAYER_ENTRY
        tag: 0,
    }];
    let physics = make_test_physics();

    // Static per-polygon info (sim-independent) and the static mesh, built ONCE
    // before any ticking. These bytes are what the vertex/index buffers hold.
    let poly_info = build_poly_info(&map);
    let mesh_before = marathon_web::mesh::build_level_mesh(&map, &poly_info);
    let vbytes_before: Vec<u8> = bytemuck::cast_slice(&mesh_before.vertices).to_vec();
    let ibytes_before: Vec<u8> = bytemuck::cast_slice(&mesh_before.indices).to_vec();

    // --- Create the sim and capture the door polygon's initial dynamic data. ---
    let mut world = SimWorld::new(&map, &physics, &SimConfig::default()).unwrap();
    let dyn_before = world.poly_dynamic_data();
    let packed_before = pack_from_sim(&dyn_before);

    // --- Tick the door open for N ticks. ---
    for _ in 0..12 {
        world.tick(ActionFlags::default().into());
    }

    let dyn_after = world.poly_dynamic_data();
    let packed_after = pack_from_sim(&dyn_after);

    // (a) The door polygon's DYNAMIC data changed: the platform floor rises.
    assert!(
        dyn_after[0].floor_height > dyn_before[0].floor_height + 0.1,
        "door (poly 0) floor should have risen: before={} after={}",
        dyn_before[0].floor_height,
        dyn_after[0].floor_height
    );

    // (b) The packed data-texture buffer (the exact bytes uploaded each frame)
    //     reflects the door motion.
    assert_ne!(
        packed_before, packed_after,
        "data-texture buffer must reflect door motion"
    );

    // (c) The static VERTEX and INDEX buffers did NOT change. This is guaranteed
    //     precisely because `build_level_mesh` takes only the (immutable) map +
    //     static `poly_info` and *no sim state* — that decoupling IS the fix this
    //     WIG delivers. Rebuilding from the same inputs after ticking yields
    //     byte-identical buffers.
    let mesh_after = marathon_web::mesh::build_level_mesh(&map, &poly_info);
    let vbytes_after: Vec<u8> = bytemuck::cast_slice(&mesh_after.vertices).to_vec();
    let ibytes_after: Vec<u8> = bytemuck::cast_slice(&mesh_after.indices).to_vec();
    assert_eq!(
        vbytes_before, vbytes_after,
        "vertex buffer (static geometry) must NOT change while the door animates"
    );
    assert_eq!(
        ibytes_before, ibytes_after,
        "index buffer (static geometry) must NOT change while the door animates"
    );
}
