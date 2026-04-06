use crate::player::inventory::{WeaponSlot, WeaponState};

/// Advance the weapon state machine by one tick.
///
/// Returns true if the weapon fired this tick.
pub fn tick_weapon(weapon: &mut WeaponSlot, fire_requested: bool, ticks_per_round: u16, recovery_ticks: u16) -> bool {
    // Decrement cooldown
    if weapon.cooldown_ticks > 0 {
        weapon.cooldown_ticks -= 1;
        if weapon.cooldown_ticks == 0 && weapon.state == WeaponState::Recovering {
            weapon.state = WeaponState::Idle;
        }
        if weapon.cooldown_ticks == 0 && weapon.state == WeaponState::Reloading {
            weapon.state = WeaponState::Idle;
        }
        return false;
    }

    match weapon.state {
        WeaponState::Idle => {
            if fire_requested {
                if weapon.consume_primary() {
                    weapon.state = WeaponState::Firing;
                    weapon.cooldown_ticks = ticks_per_round;
                    return true;
                } else if weapon.needs_primary_reload() {
                    weapon.state = WeaponState::Reloading;
                    // Reload timing handled externally
                }
            }
            false
        }
        WeaponState::Firing => {
            weapon.state = WeaponState::Recovering;
            weapon.cooldown_ticks = recovery_ticks;
            false
        }
        WeaponState::Switching => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_weapon(mag: u16, reserve: u16) -> WeaponSlot {
        WeaponSlot {
            definition_index: 0,
            primary_magazine: mag,
            primary_reserve: reserve,
            secondary_magazine: 0,
            secondary_reserve: 0,
            state: WeaponState::Idle,
            cooldown_ticks: 0,
        }
    }

    #[test]
    fn fire_when_ready() {
        let mut weapon = make_weapon(8, 0);
        let fired = tick_weapon(&mut weapon, true, 2, 3);
        assert!(fired);
        assert_eq!(weapon.primary_magazine, 7);
        assert_eq!(weapon.state, WeaponState::Firing);
        assert_eq!(weapon.cooldown_ticks, 2);
    }

    #[test]
    fn no_fire_when_empty() {
        let mut weapon = make_weapon(0, 0);
        let fired = tick_weapon(&mut weapon, true, 2, 3);
        assert!(!fired);
    }

    #[test]
    fn auto_reload_on_empty_with_reserves() {
        let mut weapon = make_weapon(0, 16);
        let fired = tick_weapon(&mut weapon, true, 2, 3);
        assert!(!fired);
        assert_eq!(weapon.state, WeaponState::Reloading);
    }

    #[test]
    fn cooldown_prevents_firing() {
        let mut weapon = make_weapon(8, 0);
        weapon.cooldown_ticks = 5;
        weapon.state = WeaponState::Recovering;
        let fired = tick_weapon(&mut weapon, true, 2, 3);
        assert!(!fired);
        assert_eq!(weapon.cooldown_ticks, 4);
    }
}
