use glam::Vec2;

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
}
