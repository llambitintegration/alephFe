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
        PlatformState::AtRest => {
            (platform.current_floor, platform.current_ceiling)
        }
        PlatformState::Extending => {
            platform.current_floor = move_toward(platform.current_floor, platform.floor_extended, platform.speed);
            platform.current_ceiling = move_toward(platform.current_ceiling, platform.ceiling_extended, platform.speed);

            let floor_done = (platform.current_floor - platform.floor_extended).abs() < f32::EPSILON;
            let ceiling_done = (platform.current_ceiling - platform.ceiling_extended).abs() < f32::EPSILON;

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
            platform.current_floor = move_toward(platform.current_floor, platform.floor_rest, platform.speed);
            platform.current_ceiling = move_toward(platform.current_ceiling, platform.ceiling_rest, platform.speed);

            let floor_done = (platform.current_floor - platform.floor_rest).abs() < f32::EPSILON;
            let ceiling_done = (platform.current_ceiling - platform.ceiling_rest).abs() < f32::EPSILON;

            if floor_done && ceiling_done {
                platform.state = PlatformState::AtRest;
            }

            (platform.current_floor, platform.current_ceiling)
        }
    }
}

/// Activate a platform (trigger it to start extending).
pub fn activate_platform(platform: &mut Platform) {
    if platform.state == PlatformState::AtRest {
        platform.state = PlatformState::Extending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
