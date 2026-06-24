//! Box 6.8 — full-cascade integration test.
//!
//! Exercises the END-TO-END MML override pipeline that the earlier boxes built,
//! using ONLY the crate's public API and synthetic MML constructed in-test:
//!
//! ```text
//! cascade assembly (global < scenario < plugin < level)
//!   -> MmlOverrideSet::from_document
//!     -> PhysicsData::apply_overrides
//! ```
//!
//! Layering precedence (lowest to highest): global < scenario < plugin < level.
//! These tests verify both that the highest-priority layer wins on a conflicting
//! field AND that disjoint per-field overrides MERGE rather than wholesale-replace.

use marathon_formats::test_helpers::{BinaryWriter, TagData, WadBuilder};
use marathon_formats::{
    assemble_mml_cascade, cascade_documents, tags::WadTag, wad::WadFile, MmlDocument,
    MmlOverrideSet, PhysicsData, PluginMmlSource,
};

/// MML byte format accepted by `MmlDocument::from_bytes`: a `<marathon>` root
/// wrapping section elements, here a single `<monsters>` with `<monster>`
/// children keyed by `index`. This is the exact shape the existing cascade unit
/// tests use.
fn mml_doc(xml: &[u8]) -> MmlDocument {
    MmlDocument::from_bytes(xml).expect("synthetic MML must parse")
}

/// Build a 156-byte `MonsterDefinition` record (mirrors the crate's internal
/// `build_monster` byte layout) so a real `PhysicsData` can be assembled from an
/// integration test, where the `#[cfg(test)]` builders inside `physics.rs` are
/// not visible. `collection`/`vitality`/`speed` are caller-controlled; every
/// other field is a sane fixed default. The record parses cleanly via
/// `MonsterDefinition::read` (no field has parse-time validation).
fn build_monster_record(collection: i16, vitality: i16, speed: i16) -> Vec<u8> {
    let mut w = BinaryWriter::new()
        .write_i16(collection)
        .write_i16(vitality)
        .write_u32(0) // immunities
        .write_u32(0) // weaknesses
        .write_u32(0) // flags
        .write_i32(0) // monster_class
        .write_i32(0) // friends
        .write_i32(0) // enemies
        .write_fixed(1.0); // sound_pitch
                           // 9 sound indices (activation..random_sound_mask)
    for _ in 0..9 {
        w = w.write_i16(-1);
    }
    w = w
        .write_i16(-1) // carrying_item_type
        .write_i16(256) // radius
        .write_i16(512) // height
        .write_i16(0) // preferred_hover_height
        .write_i16(0) // minimum_ledge_delta
        .write_i16(0) // maximum_ledge_delta
        .write_fixed(1.0) // external_velocity_scale
        .write_i16(-1) // impact_effect
        .write_i16(-1) // melee_impact_effect
        .write_i16(-1); // contrail_effect
                        // 9 i16s: half_visual_arc, half_vertical_visual_arc, visual_range,
                        // dark_visual_range, intelligence, speed, gravity, terminal_velocity,
                        // door_retry_mask. `speed` is index 5 of this block.
    let arc_block: [i16; 9] = [0, 0, 0, 0, 0, speed, 0, 0, 0];
    for v in arc_block {
        w = w.write_i16(v);
    }
    w = w.write_i16(0); // shrapnel_radius
    w = w.write_bytes(&build_damage(50)); // shrapnel_damage
                                          // 9 shape descriptors (hit..teleport_out)
    for _ in 0..9 {
        w = w.write_u16(0xFFFF);
    }
    w = w.write_i16(0); // attack_frequency
    w = w.write_bytes(&build_attack(-1)); // melee_attack
    w = w.write_bytes(&build_attack(0)); // ranged_attack
    w.build()
}

fn build_damage(base: i16) -> Vec<u8> {
    BinaryWriter::new()
        .write_i16(0) // type
        .write_i16(0) // flags
        .write_i16(base)
        .write_i16(0) // random
        .write_fixed(1.0) // scale
        .build()
}

fn build_attack(attack_type: i16) -> Vec<u8> {
    BinaryWriter::new()
        .write_i16(attack_type)
        .write_i16(1) // repetitions
        .write_i16(0) // error
        .write_i16(512) // range
        .write_i16(-1) // attack_shape
        .write_i16(0) // dx
        .write_i16(0) // dy
        .write_i16(0) // dz
        .build()
}

/// Build a real `PhysicsData` carrying a single monster at index 0 with the
/// given starting `vitality`/`speed`, via the public `PhysicsData::from_entry`
/// WAD path.
fn physics_with_one_monster(vitality: i16, speed: i16) -> PhysicsData {
    let monster_bytes = build_monster_record(0, vitality, speed);
    let wad_data = WadBuilder::new()
        .version(4)
        .add_entry(0, vec![TagData::new(WadTag::MonsterPhysics, monster_bytes)])
        .build();
    let wad = WadFile::from_bytes(&wad_data).unwrap();
    PhysicsData::from_entry(wad.entry(0).unwrap()).unwrap()
}

/// The core cascade-order assertion, end to end through PhysicsData.
///
/// Four layers all override monster index 0's `vitality` to a different value:
/// global=100, scenario=200, plugin=300, level=400. After the full cascade
/// (global < scenario < plugin < level) + `from_document` + `apply_overrides`,
/// the applied monster's vitality MUST be 400 — the highest-priority (level)
/// layer wins.
#[test]
fn full_cascade_highest_priority_layer_wins_through_physics() {
    let global = mml_doc(
        b"<marathon><monsters><monster index=\"0\" vitality=\"100\"/></monsters></marathon>",
    );
    let scenario = mml_doc(
        b"<marathon><monsters><monster index=\"0\" vitality=\"200\"/></monsters></marathon>",
    );
    let plugin = mml_doc(
        b"<marathon><monsters><monster index=\"0\" vitality=\"300\"/></monsters></marathon>",
    );
    let level = mml_doc(
        b"<marathon><monsters><monster index=\"0\" vitality=\"400\"/></monsters></marathon>",
    );

    // Assemble the cascade (lowest-priority first), interpret, and apply.
    let merged = cascade_documents(vec![global, scenario, plugin, level]);
    let overrides = MmlOverrideSet::from_document(&merged);

    // The interpreted override set already reflects the cascade winner.
    assert_eq!(
        overrides.monsters.len(),
        1,
        "exactly one monster override survives the cascade"
    );
    assert_eq!(overrides.monsters[0].index, 0);
    assert_eq!(
        overrides.monsters[0].vitality,
        Some(400),
        "level (highest priority) wins the vitality cascade"
    );

    // Apply to a real PhysicsData and assert the final definition value.
    let mut physics = physics_with_one_monster(/*vitality*/ 50, /*speed*/ 7);
    physics.apply_overrides(&overrides);
    let monster = &physics.monsters.as_ref().unwrap()[0];
    assert_eq!(
        monster.vitality, 400,
        "final physics vitality reflects the level layer's value after the full cascade"
    );
    assert_eq!(
        monster.speed, 7,
        "unrelated field (speed) preserved from the original physics definition"
    );
}

/// Layering MERGES per-field rather than wholesale-replacing the element:
/// global sets only `vitality`, level sets only `speed`. After the cascade the
/// single override for index 0 carries BOTH — vitality from global, speed from
/// level — and applying it changes both fields on the physics definition.
#[test]
fn full_cascade_disjoint_fields_merge_through_physics() {
    let global = mml_doc(
        b"<marathon><monsters><monster index=\"0\" vitality=\"123\"/></monsters></marathon>",
    );
    let level =
        mml_doc(b"<marathon><monsters><monster index=\"0\" speed=\"45\"/></monsters></marathon>");

    let merged = cascade_documents(vec![global, level]);
    let overrides = MmlOverrideSet::from_document(&merged);

    assert_eq!(overrides.monsters.len(), 1, "merged into one override");
    assert_eq!(
        overrides.monsters[0].vitality,
        Some(123),
        "vitality survives from the global layer (not wiped by the level layer)"
    );
    assert_eq!(
        overrides.monsters[0].speed,
        Some(45),
        "speed survives from the level layer (merge, not wholesale replace)"
    );

    let mut physics = physics_with_one_monster(/*vitality*/ 50, /*speed*/ 7);
    physics.apply_overrides(&overrides);
    let monster = &physics.monsters.as_ref().unwrap()[0];
    assert_eq!(monster.vitality, 123, "vitality from global applied");
    assert_eq!(monster.speed, 45, "speed from level applied");
}

/// Plugin layers apply in ALPHABETICAL name order (later-alphabetical wins),
/// independent of insertion order, exercised through the I/O cascade entry point
/// `assemble_mml_cascade` and verified at the interpreted-override level.
#[test]
fn full_cascade_plugins_alphabetical_order_then_interpret() {
    // Passed out of order on purpose: "Zeta" must still win over "Alpha".
    let plugins = vec![
        PluginMmlSource::new(
            "Zeta",
            b"<marathon><monsters><monster index=\"0\" vitality=\"900\"/></monsters></marathon>"
                .to_vec(),
        ),
        PluginMmlSource::new(
            "Alpha",
            b"<marathon><monsters><monster index=\"0\" vitality=\"100\"/></monsters></marathon>"
                .to_vec(),
        ),
    ];

    let merged = assemble_mml_cascade(&[], &[], None, &plugins, None);
    let overrides = MmlOverrideSet::from_document(&merged);

    assert_eq!(
        overrides.monsters[0].vitality,
        Some(900),
        "alphabetically-last plugin (Zeta) applied last and wins"
    );
}
