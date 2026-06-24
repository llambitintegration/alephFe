use crate::components::{Platform, PlatformState};

/// Move a value toward a target by `speed`, clamping at the target.
fn move_toward(current: f32, target: f32, speed: f32) -> f32 {
    if (target - current).abs() < f32::EPSILON {
        return target;
    }
    let dir = (target - current).signum();
    let next = current + dir * speed;
    if (dir > 0.0 && next >= target) || (dir < 0.0 && next <= target) {
        target
    } else {
        next
    }
}

/// Advance a platform by one tick.
///
/// Returns the current floor and ceiling heights after this tick.
pub fn tick_platform(platform: &mut Platform) -> (f32, f32) {
    match platform.state {
        PlatformState::AtRest => (platform.current_floor, platform.current_ceiling),
        PlatformState::Extending => {
            platform.current_floor = move_toward(
                platform.current_floor,
                platform.floor_extended,
                platform.speed,
            );
            platform.current_ceiling = move_toward(
                platform.current_ceiling,
                platform.ceiling_extended,
                platform.speed,
            );

            let floor_done =
                (platform.current_floor - platform.floor_extended).abs() < f32::EPSILON;
            let ceiling_done =
                (platform.current_ceiling - platform.ceiling_extended).abs() < f32::EPSILON;

            if floor_done && ceiling_done {
                platform.state = PlatformState::AtExtended;
                platform.delay_remaining = platform.return_delay;
            }

            (platform.current_floor, platform.current_ceiling)
        }
        PlatformState::AtExtended => {
            if platform.delay_remaining > 0 {
                platform.delay_remaining -= 1;
                if platform.delay_remaining == 0 {
                    platform.state = PlatformState::Returning;
                }
            } else {
                platform.state = PlatformState::Returning;
            }
            (platform.current_floor, platform.current_ceiling)
        }
        PlatformState::Returning => {
            platform.current_floor =
                move_toward(platform.current_floor, platform.floor_rest, platform.speed);
            platform.current_ceiling = move_toward(
                platform.current_ceiling,
                platform.ceiling_rest,
                platform.speed,
            );

            let floor_done = (platform.current_floor - platform.floor_rest).abs() < f32::EPSILON;
            let ceiling_done =
                (platform.current_ceiling - platform.ceiling_rest).abs() < f32::EPSILON;

            if floor_done && ceiling_done {
                platform.state = PlatformState::AtRest;
            }

            (platform.current_floor, platform.current_ceiling)
        }
    }
}

/// Activate a platform (trigger it to start extending).
pub fn activate_platform(platform: &mut Platform) {
    match platform.state {
        PlatformState::AtRest => {
            // Doors that start extended (closed) need to return (open).
            // Check if we're at the extended position.
            let at_extended = (platform.current_floor - platform.floor_extended).abs()
                < f32::EPSILON
                && (platform.current_ceiling - platform.ceiling_extended).abs() < f32::EPSILON;
            if at_extended {
                platform.state = PlatformState::Returning;
            } else {
                platform.state = PlatformState::Extending;
            }
        }
        // Re-activation of an already-moving platform reverses its direction.
        PlatformState::Extending => {
            platform.state = PlatformState::Returning;
        }
        PlatformState::Returning => {
            platform.state = PlatformState::Extending;
        }
        // Re-activation while paused at the extended position starts the return.
        PlatformState::AtExtended => {
            platform.state = PlatformState::Returning;
        }
    }
}

/// Platform static flag bit positions (from Marathon's platform_definitions.h).
pub const PLATFORM_IS_INITIALLY_ACTIVE: u32 = 1 << 0;
pub const PLATFORM_IS_INITIALLY_EXTENDED: u32 = 1 << 1;
pub const PLATFORM_DEACTIVATES_AT_INITIAL_LEVEL: u32 = 1 << 3;
pub const PLATFORM_COMES_FROM_FLOOR: u32 = 1 << 6;
pub const PLATFORM_COMES_FROM_CEILING: u32 = 1 << 7;
pub const PLATFORM_IS_PLAYER_CONTROLLABLE: u32 = 1 << 12;
pub const PLATFORM_IS_MONSTER_CONTROLLABLE: u32 = 1 << 13;
pub const PLATFORM_REVERSES_DIRECTION_WHEN_OBSTRUCTED: u32 = 1 << 14;
pub const PLATFORM_IS_DOOR: u32 = 1 << 25;

/// Legacy aliases used by existing code.
pub const PLATFORM_ACTIVATE_ON_PLAYER_ENTRY: u32 = PLATFORM_IS_INITIALLY_ACTIVE;
pub const PLATFORM_ACTIVATE_ON_ACTION_KEY: u32 = PLATFORM_IS_PLAYER_CONTROLLABLE;
pub const PLATFORM_ACTIVATE_ON_MONSTER_ENTRY: u32 = PLATFORM_IS_MONSTER_CONTROLLABLE;
pub const PLATFORM_ACTIVATE_ON_PROJECTILE: u32 = 1 << 15; // not used yet

/// Check if a platform should be activated based on trigger type.
pub fn should_activate(platform: &Platform, trigger: PlatformTrigger) -> bool {
    if platform.state != PlatformState::AtRest {
        return false;
    }

    match trigger {
        PlatformTrigger::PlayerEntry => {
            platform.activation_flags & PLATFORM_ACTIVATE_ON_PLAYER_ENTRY != 0
        }
        PlatformTrigger::ActionKey => {
            platform.activation_flags & PLATFORM_ACTIVATE_ON_ACTION_KEY != 0
        }
        PlatformTrigger::MonsterEntry => {
            platform.activation_flags & PLATFORM_ACTIVATE_ON_MONSTER_ENTRY != 0
        }
        PlatformTrigger::ProjectileImpact => {
            platform.activation_flags & PLATFORM_ACTIVATE_ON_PROJECTILE != 0
        }
    }
}

/// Types of triggers that can activate a platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformTrigger {
    PlayerEntry,
    ActionKey,
    MonsterEntry,
    ProjectileImpact,
}

/// Damage type emitted when a platform crushes an entity. Mirrors Marathon's
/// `_damage_crushing` (index 7 in `map.h`'s damage-type enumeration), carried in
/// `SimEvent::EntityDamaged.damage_type`.
pub const PLATFORM_CRUSH_DAMAGE_TYPE: i16 = 7;

/// Check if a platform is crushing an entity.
///
/// Returns damage if crushing, or whether the platform should reverse.
pub fn check_platform_crush(
    platform: &Platform,
    entity_z: f32,
    entity_height: f32,
) -> PlatformCrushResult {
    let clearance = platform.current_ceiling - platform.current_floor;
    if clearance < entity_height && entity_z >= platform.current_floor - f32::EPSILON {
        if platform.crushes {
            PlatformCrushResult::Crush { damage: 10 }
        } else {
            PlatformCrushResult::Reverse
        }
    } else {
        PlatformCrushResult::None
    }
}

/// Result of a platform crush check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformCrushResult {
    None,
    Crush { damage: i16 },
    Reverse,
}

/// A trigger event that fires when a platform reaches a position.
#[derive(Debug, Clone)]
pub struct PlatformTriggerEvent {
    pub trigger_type: PlatformTriggerEventType,
    pub target_index: usize,
}

/// Types of events a platform can trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformTriggerEventType {
    ActivatePlatform,
    ToggleLight,
}

/// Check if a platform should fire its triggers.
///
/// Returns triggers to fire when a platform reaches its extended or resting position.
pub fn check_platform_triggers(
    platform: &Platform,
    linked_platforms: &[usize],
    linked_lights: &[usize],
) -> Vec<PlatformTriggerEvent> {
    let mut events = Vec::new();

    let at_destination =
        platform.state == PlatformState::AtExtended || platform.state == PlatformState::AtRest;

    if !at_destination {
        return events;
    }

    for &idx in linked_platforms {
        events.push(PlatformTriggerEvent {
            trigger_type: PlatformTriggerEventType::ActivatePlatform,
            target_index: idx,
        });
    }

    for &idx in linked_lights {
        events.push(PlatformTriggerEvent {
            trigger_type: PlatformTriggerEventType::ToggleLight,
            target_index: idx,
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::PlatformType;

    fn make_platform() -> Platform {
        Platform {
            polygon_index: 0,
            floor_rest: 0.0,
            floor_extended: 1.0,
            ceiling_rest: 3.0,
            ceiling_extended: 3.0,
            current_floor: 0.0,
            current_ceiling: 3.0,
            speed: 0.5,
            state: PlatformState::AtRest,
            return_delay: 30,
            delay_remaining: 0,
            activation_flags: 0,
            crushes: false,
            platform_type: PlatformType::FromFloor,
            linked_platforms: Vec::new(),
            linked_lights: Vec::new(),
            start_sound: 0,
            stop_sound: 0,
        }
    }

    #[test]
    fn platform_at_rest() {
        let mut p = make_platform();
        let (floor, ceiling) = tick_platform(&mut p);
        assert_eq!(floor, 0.0);
        assert_eq!(ceiling, 3.0);
        assert_eq!(p.state, PlatformState::AtRest);
    }

    #[test]
    fn platform_extends() {
        let mut p = make_platform();
        activate_platform(&mut p);
        assert_eq!(p.state, PlatformState::Extending);

        let (floor, _) = tick_platform(&mut p);
        assert!((floor - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn platform_reaches_extended() {
        let mut p = make_platform();
        activate_platform(&mut p);

        tick_platform(&mut p); // 0.5
        tick_platform(&mut p); // 1.0 (extended)

        assert_eq!(p.state, PlatformState::AtExtended);
        assert_eq!(p.delay_remaining, 30);
    }

    #[test]
    fn platform_delays_then_returns() {
        let mut p = make_platform();
        p.state = PlatformState::AtExtended;
        p.current_floor = 1.0;
        p.delay_remaining = 2;

        tick_platform(&mut p); // delay 1
        assert_eq!(p.state, PlatformState::AtExtended);

        tick_platform(&mut p); // delay 0 -> Returning
        assert_eq!(p.state, PlatformState::Returning);
    }

    #[test]
    fn platform_returns_to_rest() {
        let mut p = make_platform();
        p.state = PlatformState::Returning;
        p.current_floor = 1.0;

        tick_platform(&mut p); // 0.5
        assert_eq!(p.state, PlatformState::Returning);

        tick_platform(&mut p); // 0.0 -> AtRest
        assert_eq!(p.state, PlatformState::AtRest);
    }

    #[test]
    fn platform_activates_on_player_entry() {
        let mut p = make_platform();
        p.activation_flags = PLATFORM_ACTIVATE_ON_PLAYER_ENTRY;
        assert!(should_activate(&p, PlatformTrigger::PlayerEntry));
        assert!(!should_activate(&p, PlatformTrigger::ActionKey));
    }

    #[test]
    fn platform_no_activate_when_moving() {
        let mut p = make_platform();
        p.activation_flags = PLATFORM_ACTIVATE_ON_PLAYER_ENTRY;
        p.state = PlatformState::Extending;
        assert!(!should_activate(&p, PlatformTrigger::PlayerEntry));
    }

    #[test]
    fn platform_crush_damages() {
        let mut p = make_platform();
        p.current_floor = 2.5;
        p.current_ceiling = 3.0;
        p.crushes = true;
        let result = check_platform_crush(&p, 2.5, 0.8);
        assert_eq!(result, PlatformCrushResult::Crush { damage: 10 });
    }

    #[test]
    fn platform_crush_reverses() {
        let mut p = make_platform();
        p.current_floor = 2.5;
        p.current_ceiling = 3.0;
        p.crushes = false;
        let result = check_platform_crush(&p, 2.5, 0.8);
        assert_eq!(result, PlatformCrushResult::Reverse);
    }

    #[test]
    fn platform_no_crush_with_clearance() {
        let p = make_platform();
        let result = check_platform_crush(&p, 0.0, 0.8);
        assert_eq!(result, PlatformCrushResult::None);
    }

    #[test]
    fn platform_triggers_linked() {
        let mut p = make_platform();
        p.state = PlatformState::AtExtended;
        let events = check_platform_triggers(&p, &[1, 2], &[3]);
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0].trigger_type,
            PlatformTriggerEventType::ActivatePlatform
        );
        assert_eq!(events[0].target_index, 1);
        assert_eq!(
            events[2].trigger_type,
            PlatformTriggerEventType::ToggleLight
        );
        assert_eq!(events[2].target_index, 3);
    }

    #[test]
    fn platform_no_triggers_while_moving() {
        let mut p = make_platform();
        p.state = PlatformState::Extending;
        let events = check_platform_triggers(&p, &[1], &[2]);
        assert!(events.is_empty());
    }

    #[test]
    fn platform_reactivate_while_extending_reverses() {
        let mut p = make_platform();
        p.state = PlatformState::Extending;
        p.current_floor = 0.5;
        activate_platform(&mut p);
        assert_eq!(p.state, PlatformState::Returning);
    }

    #[test]
    fn platform_reactivate_while_returning_reverses() {
        let mut p = make_platform();
        p.state = PlatformState::Returning;
        p.current_floor = 0.5;
        activate_platform(&mut p);
        assert_eq!(p.state, PlatformState::Extending);
    }

    #[test]
    fn platform_reactivate_while_at_extended_returns() {
        let mut p = make_platform();
        p.state = PlatformState::AtExtended;
        p.current_floor = 1.0;
        activate_platform(&mut p);
        assert_eq!(p.state, PlatformState::Returning);
    }
}
