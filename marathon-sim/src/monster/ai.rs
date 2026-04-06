use glam::{Vec2, Vec3};

use crate::components::MonsterState;

/// Check if a monster can see a target based on distance, facing, and visual arc.
///
/// `monster_pos`: monster's 2D position
/// `monster_facing`: monster's facing angle in radians
/// `target_pos`: target's 2D position
/// `visual_range`: maximum sight distance
/// `half_visual_arc`: half the horizontal FOV in radians
pub fn can_see_target(
    monster_pos: Vec2,
    monster_facing: f32,
    target_pos: Vec2,
    visual_range: f32,
    half_visual_arc: f32,
) -> bool {
    let to_target = target_pos - monster_pos;
    let distance = to_target.length();

    if distance > visual_range || distance < 1e-6 {
        return false;
    }

    // Angle from monster to target
    let angle_to_target = to_target.y.atan2(to_target.x);
    let angle_diff = normalize_angle(angle_to_target - monster_facing);

    angle_diff.abs() <= half_visual_arc
}

/// Normalize an angle to [-PI, PI].
fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

/// Determine the next AI state based on current state and conditions.
pub fn next_state(
    current: MonsterState,
    can_see_target: bool,
    has_target: bool,
    in_melee_range: bool,
    in_ranged_range: bool,
    vitality_zero: bool,
) -> MonsterState {
    if vitality_zero && current != MonsterState::Dead {
        return MonsterState::Dying;
    }

    match current {
        MonsterState::Idle => {
            if can_see_target {
                MonsterState::Alerted
            } else {
                MonsterState::Idle
            }
        }
        MonsterState::Alerted => {
            if !has_target {
                MonsterState::Idle
            } else if in_melee_range || in_ranged_range {
                MonsterState::Attacking
            } else {
                MonsterState::Moving
            }
        }
        MonsterState::Moving => {
            if !has_target {
                MonsterState::Idle
            } else if in_melee_range || in_ranged_range {
                MonsterState::Attacking
            } else {
                MonsterState::Moving
            }
        }
        MonsterState::Attacking => {
            if !has_target {
                MonsterState::Idle
            } else if !in_melee_range && !in_ranged_range {
                MonsterState::Moving
            } else {
                MonsterState::Attacking
            }
        }
        MonsterState::Fleeing => {
            if !has_target || !can_see_target {
                MonsterState::Idle
            } else {
                MonsterState::Fleeing
            }
        }
        MonsterState::Dying => MonsterState::Dead,
        MonsterState::Dead => MonsterState::Dead,
    }
}

/// Determine which nearby monsters should be alerted via activation cascading.
///
/// When a monster becomes alerted, nearby monsters of the same class with the
/// same enemies bitmask also become alerted.
///
/// Returns indices into the input slice of monsters that should be cascaded.
pub fn find_cascade_targets(
    source_pos: Vec2,
    source_class: usize,
    source_enemies: u32,
    monsters: &[(Vec2, usize, u32, MonsterState)], // (pos, class, enemies, state)
    cascade_radius: f32,
) -> Vec<usize> {
    let mut targets = Vec::new();
    let radius_sq = cascade_radius * cascade_radius;

    for (i, (pos, class, enemies, state)) in monsters.iter().enumerate() {
        if *state != MonsterState::Idle {
            continue;
        }
        if *class != source_class {
            continue;
        }
        if *enemies != source_enemies {
            continue;
        }
        let dist_sq = source_pos.distance_squared(*pos);
        if dist_sq <= radius_sq {
            targets.push(i);
        }
    }

    targets
}

/// Redirect a monster's target when damaged by a friendly entity.
///
/// Returns true if the target should be redirected.
pub fn should_redirect_target(
    _damaged_class: usize,
    attacker_class: usize,
    damaged_friends: u32,
) -> bool {
    // Redirect if the attacker is in the damaged monster's friends bitmask
    if attacker_class < 32 {
        damaged_friends & (1u32 << attacker_class) != 0
    } else {
        false
    }
}

/// Compute flying monster movement toward a target.
///
/// Returns the new velocity for a flying monster moving toward the target
/// at the given speed and preferred hover height.
pub fn compute_flying_movement(
    monster_pos: Vec3,
    target_pos: Vec3,
    speed: f32,
    preferred_hover_height: f32,
    floor_height: f32,
) -> Vec3 {
    let target_with_hover = Vec3::new(
        target_pos.x,
        target_pos.y,
        (floor_height + preferred_hover_height).max(target_pos.z),
    );

    let to_target = target_with_hover - monster_pos;
    let dist = to_target.length();

    if dist < 1e-6 {
        return Vec3::ZERO;
    }

    to_target.normalize() * speed
}

/// Apply gravity to a non-flying monster.
///
/// Returns (new_z, new_vel_z, grounded).
pub fn apply_monster_gravity(
    current_z: f32,
    vel_z: f32,
    floor_height: f32,
    gravity: f32,
    terminal_velocity: f32,
) -> (f32, f32, bool) {
    let new_vel_z = (vel_z - gravity).max(-terminal_velocity);
    let new_z = current_z + new_vel_z;

    if new_z <= floor_height {
        (floor_height, 0.0, true)
    } else {
        (new_z, new_vel_z, false)
    }
}

/// Result of a monster attack attempt.
#[derive(Debug, Clone)]
pub enum AttackResult {
    /// No attack this tick.
    None,
    /// Melee attack: deal direct damage to target.
    Melee {
        damage_base: i16,
        damage_random: i16,
        damage_type: i16,
        damage_scale: f32,
    },
    /// Ranged attack: spawn a projectile.
    Ranged {
        projectile_type: usize,
        /// Spawn offset from monster position.
        offset: Vec3,
        /// Random angular error in radians.
        error: f32,
    },
}

/// Determine what attack a monster should execute this tick.
pub fn compute_monster_attack(
    state: MonsterState,
    distance_to_target: f32,
    attack_cooldown: u16,
    melee_range: f32,
    melee_damage_base: i16,
    melee_damage_random: i16,
    melee_damage_type: i16,
    melee_damage_scale: f32,
    ranged_range: f32,
    ranged_projectile_type: usize,
    ranged_offset: Vec3,
    ranged_error: f32,
) -> AttackResult {
    if state != MonsterState::Attacking || attack_cooldown > 0 {
        return AttackResult::None;
    }

    if distance_to_target <= melee_range && melee_range > 0.0 {
        return AttackResult::Melee {
            damage_base: melee_damage_base,
            damage_random: melee_damage_random,
            damage_type: melee_damage_type,
            damage_scale: melee_damage_scale,
        };
    }

    if distance_to_target <= ranged_range && ranged_range > 0.0 {
        return AttackResult::Ranged {
            projectile_type: ranged_projectile_type,
            offset: ranged_offset,
            error: ranged_error,
        };
    }

    AttackResult::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monster_sees_target_ahead() {
        assert!(can_see_target(
            Vec2::ZERO,
            0.0, // facing east
            Vec2::new(5.0, 0.0),
            10.0,
            std::f32::consts::FRAC_PI_4, // 45 degree half arc
        ));
    }

    #[test]
    fn monster_cant_see_target_behind() {
        assert!(!can_see_target(
            Vec2::ZERO,
            0.0,
            Vec2::new(-5.0, 0.0), // behind
            10.0,
            std::f32::consts::FRAC_PI_4,
        ));
    }

    #[test]
    fn monster_cant_see_target_beyond_range() {
        assert!(!can_see_target(
            Vec2::ZERO,
            0.0,
            Vec2::new(15.0, 0.0),
            10.0,
            std::f32::consts::FRAC_PI_4,
        ));
    }

    #[test]
    fn idle_to_alerted_on_sight() {
        assert_eq!(
            next_state(MonsterState::Idle, true, true, false, false, false),
            MonsterState::Alerted
        );
    }

    #[test]
    fn alerted_to_attacking_in_range() {
        assert_eq!(
            next_state(MonsterState::Alerted, true, true, true, false, false),
            MonsterState::Attacking
        );
    }

    #[test]
    fn alerted_to_moving_out_of_range() {
        assert_eq!(
            next_state(MonsterState::Alerted, true, true, false, false, false),
            MonsterState::Moving
        );
    }

    #[test]
    fn any_state_to_dying_on_zero_vitality() {
        assert_eq!(
            next_state(MonsterState::Attacking, true, true, true, true, true),
            MonsterState::Dying
        );
    }

    #[test]
    fn dying_to_dead() {
        assert_eq!(
            next_state(MonsterState::Dying, false, false, false, false, false),
            MonsterState::Dead
        );
    }

    #[test]
    fn cascade_alerts_same_class() {
        let monsters = vec![
            (Vec2::new(1.0, 0.0), 0, 0xFF, MonsterState::Idle),
            (Vec2::new(5.0, 0.0), 0, 0xFF, MonsterState::Idle),
            (Vec2::new(100.0, 0.0), 0, 0xFF, MonsterState::Idle), // too far
            (Vec2::new(2.0, 0.0), 1, 0xFF, MonsterState::Idle),   // different class
        ];
        let targets = find_cascade_targets(Vec2::ZERO, 0, 0xFF, &monsters, 10.0);
        assert_eq!(targets, vec![0, 1]);
    }

    #[test]
    fn cascade_skips_already_alerted() {
        let monsters = vec![
            (Vec2::new(1.0, 0.0), 0, 0xFF, MonsterState::Alerted),
        ];
        let targets = find_cascade_targets(Vec2::ZERO, 0, 0xFF, &monsters, 10.0);
        assert!(targets.is_empty());
    }

    #[test]
    fn friendly_fire_redirect() {
        // Monster class 0 has class 1 as friend (bit 1 set in friends mask)
        assert!(should_redirect_target(0, 1, 0b10));
        assert!(!should_redirect_target(0, 1, 0b01)); // class 1 not in friends
    }

    #[test]
    fn flying_movement_toward_target() {
        let vel = compute_flying_movement(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(10.0, 0.0, 0.0),
            1.0,
            2.0,
            0.0,
        );
        assert!(vel.x > 0.0); // moving toward target
        assert!((vel.length() - 1.0).abs() < 0.01); // at speed
    }

    #[test]
    fn monster_gravity_grounds() {
        let (z, vel, grounded) = apply_monster_gravity(0.01, -0.05, 0.0, 0.01, 1.0);
        // Should hit floor at 0.0 (0.01 + (-0.05 - 0.01) = -0.05, clamped to 0.0)
        assert_eq!(z, 0.0);
        assert_eq!(vel, 0.0);
        assert!(grounded);
    }

    #[test]
    fn monster_gravity_falls() {
        let (z, vel, grounded) = apply_monster_gravity(5.0, 0.0, 0.0, 0.01, 1.0);
        assert!(z < 5.0);
        assert!(vel < 0.0);
        assert!(!grounded);
    }

    #[test]
    fn monster_melee_attack_in_range() {
        let result = compute_monster_attack(
            MonsterState::Attacking, 0.5, 0,
            1.0, 10, 5, 0, 1.0,
            10.0, 0, Vec3::ZERO, 0.0,
        );
        matches!(result, AttackResult::Melee { .. });
    }

    #[test]
    fn monster_ranged_attack() {
        let result = compute_monster_attack(
            MonsterState::Attacking, 5.0, 0,
            1.0, 10, 5, 0, 1.0,
            10.0, 3, Vec3::new(0.0, 0.0, 0.5), 0.1,
        );
        matches!(result, AttackResult::Ranged { .. });
    }

    #[test]
    fn monster_no_attack_on_cooldown() {
        let result = compute_monster_attack(
            MonsterState::Attacking, 0.5, 5,
            1.0, 10, 5, 0, 1.0,
            10.0, 0, Vec3::ZERO, 0.0,
        );
        matches!(result, AttackResult::None);
    }
}
