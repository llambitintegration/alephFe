//! Box 3.1 integration test: the web frame path drives the per-polygon
//! data-texture upload from a single `SimWorld::render_snapshot()` per frame,
//! not from a separate `poly_dynamic_data()` accessor call.
//!
//! # Why this is a CPU-side assertion, not a real headless-wgpu readback
//!
//! `render.rs::frame()` is `wasm32`-only (it drives a live `wgpu::Surface` via
//! `requestAnimationFrame`) and cannot run in the `rust:slim` CI test runner,
//! which has no GPU and no browser. So — exactly as the box 4.3 test
//! (`dynamic_geometry.rs`) does — we assert on the CPU-side value that *fully
//! determines* the GPU upload:
//!
//! * `poly_data::poly_dyn_data_from_snapshot(&snapshot)` is the slice that
//!   `frame()` now feeds to `write_poly_data_texture`, sourced from
//!   `snapshot.poly_dynamic`. The box's requirement is that this single
//!   snapshot replaces the scattered per-frame accessor calls *without changing
//!   the uploaded bytes*: the data-texture payload built from
//!   `render_snapshot().poly_dynamic` must equal the payload the pre-migration
//!   path built from `poly_dynamic_data()` directly.
//!
//! We also assert the single snapshot still surfaces the player/weapon/entity
//! state that the scattered `player_*` / `entities()` / `player_weapon_state()`
//! calls used to fetch, so the one snapshot genuinely feeds every per-frame
//! consumer the box enumerates (render.rs ~245-265, 309, 451-459, 467-473).

mod common_map;
use common_map::{make_test_map, make_test_physics};

use marathon_formats::map::StaticPlatformData;
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimWorld};
use marathon_web::poly_data::{
    pack_poly_data, poly_dyn_data_from_sim_slice, poly_dyn_data_from_snapshot,
};

/// Box 3.1: the data-texture payload built from a single
/// `render_snapshot().poly_dynamic` is byte-identical to the payload the
/// pre-migration path built from a standalone `poly_dynamic_data()` call, after
/// ticking an animated door. The migration is value-preserving.
#[test]
fn data_texture_upload_feeds_from_render_snapshot_poly_dynamic() {
    let mut map = make_test_map();
    // Player-entry-activated platform (door) on poly 0 so poly_dynamic changes.
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

    let mut world = SimWorld::new(&map, &physics, &SimConfig::default()).unwrap();

    for _ in 0..12 {
        world.tick(ActionFlags::default().into());
    }

    // (a) Legacy path: the standalone accessor frame() used pre-3.1.
    let legacy_dyn = world.poly_dynamic_data();
    let legacy_packed = pack_poly_data(&poly_dyn_data_from_sim_slice(&legacy_dyn));

    // (b) New path: a single render_snapshot() per frame, data texture fed from
    //     snapshot.poly_dynamic via the migration helper frame() now calls.
    let snapshot = world.render_snapshot();
    let snapshot_packed = pack_poly_data(&poly_dyn_data_from_snapshot(&snapshot));

    assert_eq!(
        legacy_packed, snapshot_packed,
        "data-texture payload from snapshot.poly_dynamic must equal the legacy \
         poly_dynamic_data() payload (box 3.1: single snapshot, same bytes)"
    );

    // Sanity: the door actually animated, so this is a non-trivial payload.
    assert!(
        snapshot.poly_dynamic[0].floor_height > 0.1,
        "door (poly 0) floor should have risen so the payload is non-trivial: {}",
        snapshot.poly_dynamic[0].floor_height
    );
}

/// Box 3.1: the single snapshot surfaces the player/weapon/entity state that the
/// scattered per-frame accessors used to fetch individually, so one
/// `render_snapshot()` can feed every consumer the box enumerates.
#[test]
fn render_snapshot_surfaces_all_scattered_accessor_data() {
    let map = make_test_map();
    let physics = make_test_physics();
    let mut world = SimWorld::new(&map, &physics, &SimConfig::default()).unwrap();

    for _ in 0..5 {
        world.tick(ActionFlags::default().into());
    }

    // Capture the individual accessors first (mirrors render.rs ~245-253, 309).
    let pos = world.player_position();
    let facing = world.player_facing();
    let vlook = world.player_vertical_look();
    let health = world.player_health();
    let shield = world.player_shield();
    let oxygen = world.player_oxygen();
    let entities = world.entities();
    let weapon = world.player_weapon_state();

    let snap = world.render_snapshot();

    let player = snap.player.expect("player present");
    assert_eq!(Some(player.position), pos, "snapshot.player.position");
    assert_eq!(Some(player.facing), facing, "snapshot.player.facing");
    assert_eq!(
        Some(player.vertical_look),
        vlook,
        "snapshot.player.vertical_look"
    );
    assert_eq!(Some(player.health), health, "snapshot.player.health");
    assert_eq!(Some(player.shield), shield, "snapshot.player.shield");
    assert_eq!(Some(player.oxygen), oxygen, "snapshot.player.oxygen");
    assert_eq!(
        snap.entities.len(),
        entities.len(),
        "snapshot.entities count matches entities()"
    );
    assert_eq!(
        snap.weapon.is_some(),
        weapon.is_some(),
        "snapshot.weapon presence matches player_weapon_state()"
    );
}
