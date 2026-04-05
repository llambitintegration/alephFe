use std::io::Cursor;

use binrw::BinRead;

use crate::error::PhysicsError;
use crate::tags::WadTag;
use crate::types::{fixed_to_f32, DamageDefinition};
use crate::wad::WadEntry;

// ─── Constants ──────────────────────────────────────────────────────────────

const PHYSICS_CONSTANTS_SIZE: usize = 104;
const MONSTER_SIZE: usize = 156;
const EFFECT_SIZE: usize = 14;
const PROJECTILE_SIZE: usize = 48;
const WEAPON_SIZE: usize = 134;
const M1_PHYS_EDITOR_RECORD_SIZE: usize = 100;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn validate_tag_length(
    tag_name: &str,
    data_len: usize,
    record_size: usize,
) -> Result<(), PhysicsError> {
    if !data_len.is_multiple_of(record_size) {
        return Err(PhysicsError::InvalidTagLength {
            tag: tag_name.to_string(),
            length: data_len,
            record_size,
        });
    }
    Ok(())
}

fn parse_array<T: for<'a> BinRead<Args<'a> = ()> + binrw::meta::ReadEndian>(
    data: &[u8],
    record_size: usize,
) -> Result<Vec<T>, PhysicsError> {
    let count = data.len() / record_size;
    let mut cursor = Cursor::new(data);
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(T::read(&mut cursor)?);
    }
    Ok(items)
}

// ─── PhysicsConstants (104 bytes, 26 fixed-point fields) ────────────────────

/// Player physics model (walking or running).
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct PhysicsConstants {
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_forward_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_backward_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_perpendicular_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub acceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub deceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub airborne_deceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub gravitational_acceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub climbing_acceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub terminal_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub external_deceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub angular_acceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub angular_deceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_angular_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub angular_recentering_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub fast_angular_velocity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub fast_angular_maximum: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_elevation: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub external_angular_deceleration: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub step_delta: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub step_amplitude: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub radius: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub dead_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub camera_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub splash_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub half_camera_separation: f32,
}

// ─── AttackDefinition (16 bytes) ────────────────────────────────────────────

/// Melee or ranged attack parameters for a monster.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct AttackDefinition {
    pub attack_type: i16,
    pub repetitions: i16,
    pub error: i16,
    pub range: i16,
    pub attack_shape: i16,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}

// ─── MonsterDefinition (156 bytes) ──────────────────────────────────────────

/// Complete definition of a monster type.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct MonsterDefinition {
    pub collection: i16,
    pub vitality: i16,
    pub immunities: u32,
    pub weaknesses: u32,
    pub flags: u32,
    pub monster_class: i32,
    pub friends: i32,
    pub enemies: i32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub sound_pitch: f32,
    pub activation_sound: i16,
    pub friendly_activation_sound: i16,
    pub clear_sound: i16,
    pub kill_sound: i16,
    pub apology_sound: i16,
    pub friendly_fire_sound: i16,
    pub flaming_sound: i16,
    pub random_sound: i16,
    pub random_sound_mask: i16,
    pub carrying_item_type: i16,
    pub radius: i16,
    pub height: i16,
    pub preferred_hover_height: i16,
    pub minimum_ledge_delta: i16,
    pub maximum_ledge_delta: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub external_velocity_scale: f32,
    pub impact_effect: i16,
    pub melee_impact_effect: i16,
    pub contrail_effect: i16,
    pub half_visual_arc: i16,
    pub half_vertical_visual_arc: i16,
    pub visual_range: i16,
    pub dark_visual_range: i16,
    pub intelligence: i16,
    pub speed: i16,
    pub gravity: i16,
    pub terminal_velocity: i16,
    pub door_retry_mask: i16,
    pub shrapnel_radius: i16,
    pub shrapnel_damage: DamageDefinition,
    pub hit_shapes: u16,
    pub hard_dying_shape: u16,
    pub soft_dying_shape: u16,
    pub hard_dead_shapes: u16,
    pub soft_dead_shapes: u16,
    pub stationary_shape: u16,
    pub moving_shape: u16,
    pub teleport_in_shape: u16,
    pub teleport_out_shape: u16,
    pub attack_frequency: i16,
    pub melee_attack: AttackDefinition,
    pub ranged_attack: AttackDefinition,
}

// ─── ProjectileDefinition (48 bytes) ────────────────────────────────────────

/// Definition of a projectile type.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct ProjectileDefinition {
    pub collection: i16,
    pub shape: i16,
    pub detonation_effect: i16,
    pub media_detonation_effect: i16,
    pub contrail_effect: i16,
    pub ticks_between_contrails: i16,
    pub maximum_contrails: i16,
    pub media_projectile_promotion: i16,
    pub radius: i16,
    pub area_of_effect: i16,
    pub damage: DamageDefinition,
    pub flags: u32,
    pub speed: i16,
    pub maximum_range: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub sound_pitch: f32,
    pub flyby_sound: i16,
    pub rebound_sound: i16,
}

// ─── EffectDefinition (14 bytes) ────────────────────────────────────────────

/// Definition of a visual/audio effect.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct EffectDefinition {
    pub collection: i16,
    pub shape: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub sound_pitch: f32,
    pub flags: u16,
    pub delay: i16,
    pub delay_sound: i16,
}

// ─── TriggerDefinition (36 bytes) ───────────────────────────────────────────

/// Weapon trigger configuration (primary or secondary).
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct TriggerDefinition {
    pub rounds_per_magazine: i16,
    pub ammunition_type: i16,
    pub ticks_per_round: i16,
    pub recovery_ticks: i16,
    pub charging_ticks: i16,
    pub recoil_magnitude: i16,
    pub firing_sound: i16,
    pub click_sound: i16,
    pub charging_sound: i16,
    pub shell_casing_sound: i16,
    pub reloading_sound: i16,
    pub charged_sound: i16,
    pub projectile_type: i16,
    pub theta_error: i16,
    pub dx: i16,
    pub dz: i16,
    pub shell_casing_type: i16,
    pub burst_count: i16,
}

// ─── WeaponDefinition (134 bytes) ───────────────────────────────────────────

/// Complete definition of a weapon type.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct WeaponDefinition {
    pub item_type: i16,
    pub powerup_type: i16,
    pub weapon_class: i16,
    pub flags: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub firing_light_intensity: f32,
    pub firing_intensity_decay_ticks: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub idle_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub bob_amplitude: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub kick_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub reload_height: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub idle_width: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub horizontal_amplitude: f32,
    pub collection: i16,
    pub idle_shape: i16,
    pub firing_shape: i16,
    #[br(pad_after = 2)]
    pub reloading_shape: i16,
    pub charging_shape: i16,
    pub charged_shape: i16,
    pub ready_ticks: i16,
    pub await_reload_ticks: i16,
    pub loading_ticks: i16,
    pub finish_loading_ticks: i16,
    pub powerup_ticks: i16,
    pub primary_trigger: TriggerDefinition,
    pub secondary_trigger: TriggerDefinition,
}

// ─── PhysicsData ────────────────────────────────────────────────────────────

/// Aggregate of all physics data from a WAD entry.
#[derive(Debug, Clone)]
pub struct PhysicsData {
    pub monsters: Option<Vec<MonsterDefinition>>,
    pub effects: Option<Vec<EffectDefinition>>,
    pub projectiles: Option<Vec<ProjectileDefinition>>,
    pub physics: Option<Vec<PhysicsConstants>>,
    pub weapons: Option<Vec<WeaponDefinition>>,
}

impl PhysicsData {
    /// Parse all physics data from a WAD entry.
    /// M2/Infinity tags take precedence over M1 tags.
    pub fn from_entry(entry: &WadEntry) -> Result<Self, PhysicsError> {
        let monsters = parse_tag_or_fallback(
            entry,
            WadTag::MonsterPhysics,
            WadTag::M1MonsterPhysics,
            "MNpx",
            MONSTER_SIZE,
        )?;

        let effects = parse_tag_or_fallback(
            entry,
            WadTag::EffectsPhysics,
            WadTag::M1EffectsPhysics,
            "FXpx",
            EFFECT_SIZE,
        )?;

        let projectiles = parse_tag_or_fallback(
            entry,
            WadTag::ProjectilePhysics,
            WadTag::M1ProjectilePhysics,
            "PRpx",
            PROJECTILE_SIZE,
        )?;

        let weapons = parse_tag_or_fallback(
            entry,
            WadTag::WeaponsPhysics,
            WadTag::M1WeaponsPhysics,
            "WPpx",
            WEAPON_SIZE,
        )?;

        // Player physics: special M1 handling (skip editor record)
        let physics = if let Some(data) = entry.get_tag_data(WadTag::PlayerPhysics) {
            validate_tag_length("PXpx", data.len(), PHYSICS_CONSTANTS_SIZE)?;
            Some(parse_array::<PhysicsConstants>(
                data,
                PHYSICS_CONSTANTS_SIZE,
            )?)
        } else if let Some(data) = entry.get_tag_data(WadTag::M1PlayerPhysics) {
            if data.len() > M1_PHYS_EDITOR_RECORD_SIZE {
                let phys_data = &data[M1_PHYS_EDITOR_RECORD_SIZE..];
                validate_tag_length("phys", phys_data.len(), PHYSICS_CONSTANTS_SIZE)?;
                Some(parse_array::<PhysicsConstants>(
                    phys_data,
                    PHYSICS_CONSTANTS_SIZE,
                )?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            monsters,
            effects,
            projectiles,
            physics,
            weapons,
        })
    }
}

fn parse_tag_or_fallback<T: for<'a> BinRead<Args<'a> = ()> + binrw::meta::ReadEndian>(
    entry: &WadEntry,
    m2_tag: WadTag,
    m1_tag: WadTag,
    tag_name: &str,
    record_size: usize,
) -> Result<Option<Vec<T>>, PhysicsError> {
    let data = entry
        .get_tag_data(m2_tag)
        .or_else(|| entry.get_tag_data(m1_tag));
    match data {
        Some(data) => {
            validate_tag_length(tag_name, data.len(), record_size)?;
            Ok(Some(parse_array(data, record_size)?))
        }
        None => Ok(None),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{BinaryWriter, TagData, WadBuilder};
    use crate::wad::WadFile;

    fn build_physics_constants(max_forward: f32) -> Vec<u8> {
        let mut w = BinaryWriter::new().write_fixed(max_forward);
        // 25 more fixed-point fields, all zero
        for _ in 0..25 {
            w = w.write_fixed(0.0);
        }
        w.build()
    }

    fn build_effect(collection: i16, shape: i16, flags: u16) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(collection)
            .write_i16(shape)
            .write_fixed(1.0) // sound_pitch
            .write_u16(flags)
            .write_i16(0) // delay
            .write_i16(-1) // delay_sound
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

    fn build_damage(base: i16) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(0) // type
            .write_i16(0) // flags
            .write_i16(base)
            .write_i16(0) // random
            .write_fixed(1.0) // scale
            .build()
    }

    fn build_monster(collection: i16, vitality: i16, flags: u32) -> Vec<u8> {
        let mut w = BinaryWriter::new()
            .write_i16(collection)
            .write_i16(vitality)
            .write_u32(0) // immunities
            .write_u32(0) // weaknesses
            .write_u32(flags)
            .write_i32(0) // class
            .write_i32(0) // friends
            .write_i32(0) // enemies
            .write_fixed(1.0); // sound_pitch
                               // 6 sound indices + flaming_sound + random_sound + random_sound_mask
        for _ in 0..9 {
            w = w.write_i16(-1);
        }
        w = w
            .write_i16(-1) // carrying_item_type
            .write_i16(256) // radius
            .write_i16(512) // height
            .write_i16(0) // preferred_hover_height
            .write_i16(0) // min_ledge_delta
            .write_i16(0) // max_ledge_delta
            .write_fixed(1.0) // external_velocity_scale
            .write_i16(-1) // impact_effect
            .write_i16(-1) // melee_impact_effect
            .write_i16(-1); // contrail_effect
                            // visual arcs, ranges, intelligence, speed, gravity, terminal_velocity, door_retry_mask
        for _ in 0..9 {
            w = w.write_i16(0);
        }
        w = w.write_i16(0); // shrapnel_radius
        w = w.write_bytes(&build_damage(50)); // shrapnel_damage
                                              // shape descriptors: hit, hard_dying, soft_dying, hard_dead, soft_dead, stationary, moving, teleport_in, teleport_out
        for _ in 0..9 {
            w = w.write_u16(0xFFFF);
        }
        w = w.write_i16(0); // attack_frequency
        w = w.write_bytes(&build_attack(-1)); // melee_attack
        w = w.write_bytes(&build_attack(0)); // ranged_attack
        w.build()
    }

    fn build_projectile(collection: i16, flags: u32) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(collection)
            .write_i16(0) // shape
            .write_i16(-1) // detonation_effect
            .write_i16(-1) // media_detonation_effect
            .write_i16(-1) // contrail_effect
            .write_i16(0) // ticks_between_contrails
            .write_i16(-1) // maximum_contrails
            .write_i16(-1) // media_projectile_promotion
            .write_i16(64) // radius
            .write_i16(0) // area_of_effect
            .write_bytes(&build_damage(10)) // damage
            .write_u32(flags)
            .write_i16(128) // speed
            .write_i16(1024) // maximum_range
            .write_fixed(1.0) // sound_pitch
            .write_i16(-1) // flyby_sound
            .write_i16(-1) // rebound_sound
            .build()
    }

    fn build_trigger() -> Vec<u8> {
        let mut w = BinaryWriter::new()
            .write_i16(8) // rounds_per_magazine
            .write_i16(-1) // ammunition_type
            .write_i16(2) // ticks_per_round
            .write_i16(5) // recovery_ticks
            .write_i16(0) // charging_ticks
            .write_i16(10); // recoil_magnitude
                            // 6 sound indices
        for _ in 0..6 {
            w = w.write_i16(-1);
        }
        w = w
            .write_i16(0) // projectile_type
            .write_i16(5) // theta_error
            .write_i16(0) // dx
            .write_i16(0) // dz
            .write_i16(-1) // shell_casing_type
            .write_i16(0); // burst_count
        w.build()
    }

    fn build_weapon(item_type: i16, weapon_class: i16) -> Vec<u8> {
        let mut w = BinaryWriter::new()
            .write_i16(item_type)
            .write_i16(-1) // powerup_type
            .write_i16(weapon_class)
            .write_i16(0) // flags
            .write_fixed(1.0) // firing_light_intensity
            .write_i16(4); // firing_intensity_decay_ticks
                           // 6 fixed-point visual fields
        for _ in 0..6 {
            w = w.write_fixed(0.0);
        }
        w = w
            .write_i16(0) // collection
            .write_i16(0) // idle_shape
            .write_i16(1) // firing_shape
            .write_i16(2) // reloading_shape
            .write_i16(0) // unused
            .write_i16(3) // charging_shape
            .write_i16(4); // charged_shape
                           // 5 timing ticks
        for _ in 0..5 {
            w = w.write_i16(0);
        }
        w = w.write_bytes(&build_trigger()); // primary trigger
        w = w.write_bytes(&build_trigger()); // secondary trigger
        w.build()
    }

    // ─── PhysicsConstants ───────────────────────────────────────────────────

    #[test]
    fn test_physics_constants_parsing() {
        let data = build_physics_constants(1.0);
        assert_eq!(data.len(), PHYSICS_CONSTANTS_SIZE);
        let mut cursor = Cursor::new(&data[..]);
        let pc = PhysicsConstants::read(&mut cursor).unwrap();
        assert!((pc.maximum_forward_velocity - 1.0).abs() < 0.001);
        assert!((pc.maximum_backward_velocity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_physics_constants_fixed_point_values() {
        // Build with specific values for first 3 fields
        let mut w = BinaryWriter::new();
        w = w.write_i32(0x00010000); // 1.0
        w = w.write_i32(0xFFFF0000u32 as i32); // -1.0
        w = w.write_i32(0x00008000); // 0.5
        for _ in 0..23 {
            w = w.write_fixed(0.0);
        }
        let data = w.build();
        let mut cursor = Cursor::new(&data[..]);
        let pc = PhysicsConstants::read(&mut cursor).unwrap();
        assert!((pc.maximum_forward_velocity - 1.0).abs() < 0.001);
        assert!((pc.maximum_backward_velocity - (-1.0)).abs() < 0.001);
        assert!((pc.maximum_perpendicular_velocity - 0.5).abs() < 0.001);
    }

    // ─── MonsterDefinition ──────────────────────────────────────────────────

    #[test]
    fn test_monster_definition_parsing() {
        let data = build_monster(5, 100, 0x0002);
        assert_eq!(data.len(), MONSTER_SIZE);
        let mut cursor = Cursor::new(&data[..]);
        let m = MonsterDefinition::read(&mut cursor).unwrap();
        assert_eq!(m.collection, 5);
        assert_eq!(m.vitality, 100);
        assert_eq!(m.flags, 0x0002); // _monster_flys
        assert_eq!(m.carrying_item_type, -1);
        assert_eq!(m.melee_attack.attack_type, -1); // no melee
        assert_eq!(m.ranged_attack.attack_type, 0); // has ranged
    }

    // ─── EffectDefinition ───────────────────────────────────────────────────

    #[test]
    fn test_effect_definition_parsing() {
        let data = build_effect(3, 7, 0x0001);
        assert_eq!(data.len(), EFFECT_SIZE);
        let mut cursor = Cursor::new(&data[..]);
        let e = EffectDefinition::read(&mut cursor).unwrap();
        assert_eq!(e.collection, 3);
        assert_eq!(e.shape, 7);
        assert_eq!(e.flags, 0x0001);
        assert_eq!(e.delay, 0);
        assert_eq!(e.delay_sound, -1);
    }

    // ─── ProjectileDefinition ───────────────────────────────────────────────

    #[test]
    fn test_projectile_definition_parsing() {
        let data = build_projectile(2, 0x0010);
        assert_eq!(data.len(), PROJECTILE_SIZE);
        let mut cursor = Cursor::new(&data[..]);
        let p = ProjectileDefinition::read(&mut cursor).unwrap();
        assert_eq!(p.collection, 2);
        assert_eq!(p.flags, 0x0010); // affected_by_gravity
        assert_eq!(p.radius, 64);
        assert_eq!(p.area_of_effect, 0);
        assert_eq!(p.damage.base, 10);
    }

    #[test]
    fn test_projectile_invisible() {
        let data = build_projectile(-1, 0);
        let mut cursor = Cursor::new(&data[..]);
        let p = ProjectileDefinition::read(&mut cursor).unwrap();
        assert_eq!(p.collection, -1);
    }

    // ─── WeaponDefinition ───────────────────────────────────────────────────

    #[test]
    fn test_weapon_definition_parsing() {
        let data = build_weapon(0, 3);
        assert_eq!(data.len(), WEAPON_SIZE);
        let mut cursor = Cursor::new(&data[..]);
        let w = WeaponDefinition::read(&mut cursor).unwrap();
        assert_eq!(w.item_type, 0);
        assert_eq!(w.weapon_class, 3); // twofisted_pistol
        assert_eq!(w.powerup_type, -1);
        assert_eq!(w.primary_trigger.rounds_per_magazine, 8);
        assert_eq!(w.primary_trigger.ammunition_type, -1);
        assert_eq!(w.primary_trigger.burst_count, 0);
        assert_eq!(w.secondary_trigger.rounds_per_magazine, 8);
    }

    // ─── TriggerDefinition ──────────────────────────────────────────────────

    #[test]
    fn test_trigger_definition_parsing() {
        let data = build_trigger();
        assert_eq!(data.len(), 36);
        let mut cursor = Cursor::new(&data[..]);
        let t = TriggerDefinition::read(&mut cursor).unwrap();
        assert_eq!(t.rounds_per_magazine, 8);
        assert_eq!(t.ammunition_type, -1);
        assert_eq!(t.ticks_per_round, 2);
        assert_eq!(t.recovery_ticks, 5);
        assert_eq!(t.recoil_magnitude, 10);
        assert_eq!(t.theta_error, 5);
        assert_eq!(t.burst_count, 0);
    }

    // ─── PhysicsData from WadEntry ──────────────────────────────────────────

    #[test]
    fn test_physics_data_from_entry() {
        let physics = build_physics_constants(0.5);
        let monster = build_monster(0, 50, 0);
        let effect = build_effect(1, 2, 0);

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::PlayerPhysics, physics),
                    TagData::new(WadTag::MonsterPhysics, monster),
                    TagData::new(WadTag::EffectsPhysics, effect),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        assert!(pd.physics.is_some());
        assert_eq!(pd.physics.as_ref().unwrap().len(), 1);
        assert!((pd.physics.as_ref().unwrap()[0].maximum_forward_velocity - 0.5).abs() < 0.001);

        assert!(pd.monsters.is_some());
        assert_eq!(pd.monsters.as_ref().unwrap()[0].vitality, 50);

        assert!(pd.effects.is_some());
        assert_eq!(pd.effects.as_ref().unwrap()[0].collection, 1);

        assert!(pd.projectiles.is_none());
        assert!(pd.weapons.is_none());
    }

    #[test]
    fn test_physics_data_m2_precedence() {
        // M2 tag should take precedence over M1 tag
        let m2_effect = build_effect(10, 0, 0);
        let m1_effect = build_effect(20, 0, 0);

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::EffectsPhysics, m2_effect),
                    TagData::new(WadTag::M1EffectsPhysics, m1_effect),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        assert_eq!(pd.effects.as_ref().unwrap()[0].collection, 10);
    }

    #[test]
    fn test_physics_data_m1_fallback() {
        let m1_effect = build_effect(20, 0, 0);

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::M1EffectsPhysics, m1_effect)])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        assert_eq!(pd.effects.as_ref().unwrap()[0].collection, 20);
    }

    #[test]
    fn test_physics_data_m1_phys_skip_editor() {
        // M1 phys: 100 byte editor record + 104 byte physics constants
        let mut phys_data = vec![0u8; M1_PHYS_EDITOR_RECORD_SIZE];
        phys_data.extend_from_slice(&build_physics_constants(2.0));

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::M1PlayerPhysics, phys_data)])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        assert!(pd.physics.is_some());
        assert!((pd.physics.as_ref().unwrap()[0].maximum_forward_velocity - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_physics_data_empty_entry() {
        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::MapInfo, vec![0u8; 88])])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        assert!(pd.monsters.is_none());
        assert!(pd.effects.is_none());
        assert!(pd.projectiles.is_none());
        assert!(pd.physics.is_none());
        assert!(pd.weapons.is_none());
    }

    #[test]
    fn test_physics_data_invalid_tag_length() {
        // Effect is 14 bytes, so 15 bytes is invalid
        let bad_data = vec![0u8; 15];

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::EffectsPhysics, bad_data)])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let result = PhysicsData::from_entry(entry);
        assert!(result.is_err());
        match result.unwrap_err() {
            PhysicsError::InvalidTagLength {
                tag,
                length,
                record_size,
            } => {
                assert_eq!(tag, "FXpx");
                assert_eq!(length, 15);
                assert_eq!(record_size, 14);
            }
            other => panic!("expected InvalidTagLength, got {other:?}"),
        }
    }

    #[test]
    fn test_physics_data_multiple_records() {
        // 2 physics constants (walking + running)
        let mut two_records = build_physics_constants(1.0);
        two_records.extend_from_slice(&build_physics_constants(2.0));

        let wad_data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::PlayerPhysics, two_records)])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let pd = PhysicsData::from_entry(entry).unwrap();

        let records = pd.physics.unwrap();
        assert_eq!(records.len(), 2);
        assert!((records[0].maximum_forward_velocity - 1.0).abs() < 0.001);
        assert!((records[1].maximum_forward_velocity - 2.0).abs() < 0.001);
    }
}
