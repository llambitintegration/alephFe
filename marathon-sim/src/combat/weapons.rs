use crate::player::inventory::{WeaponSlot, WeaponState};

/// Result of a weapon fire attempt, including burst fire info.
#[derive(Debug, Clone)]
pub struct FireResult {
    /// Whether the weapon fired at all.
    pub fired: bool,
    /// Number of projectiles to spawn (1 normally, burst_count for burst fire).
    pub projectile_count: u16,
    /// Spread angle per projectile for burst fire (theta error).
    pub theta_error: f32,
}

impl FireResult {
    pub fn none() -> Self {
        Self { fired: false, projectile_count: 0, theta_error: 0.0 }
    }

    pub fn single() -> Self {
        Self { fired: true, projectile_count: 1, theta_error: 0.0 }
    }

    pub fn burst(count: u16, theta_error: f32) -> Self {
        Self { fired: true, projectile_count: count, theta_error }
    }
}

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

/// Advance weapon with burst fire support.
///
/// Returns a FireResult indicating how many projectiles to spawn.
pub fn tick_weapon_burst(
    weapon: &mut WeaponSlot,
    fire_requested: bool,
    ticks_per_round: u16,
    recovery_ticks: u16,
    burst_count: u16,
    theta_error: f32,
) -> FireResult {
    if weapon.cooldown_ticks > 0 {
        weapon.cooldown_ticks -= 1;
        if weapon.cooldown_ticks == 0 && weapon.state == WeaponState::Recovering {
            weapon.state = WeaponState::Idle;
        }
        if weapon.cooldown_ticks == 0 && weapon.state == WeaponState::Reloading {
            weapon.state = WeaponState::Idle;
        }
        return FireResult::none();
    }

    match weapon.state {
        WeaponState::Idle => {
            if fire_requested {
                if burst_count > 1 {
                    // Burst fire: consume one round, spawn multiple projectiles
                    if weapon.consume_primary() {
                        weapon.state = WeaponState::Firing;
                        weapon.cooldown_ticks = ticks_per_round;
                        return FireResult::burst(burst_count, theta_error);
                    }
                } else if weapon.consume_primary() {
                    weapon.state = WeaponState::Firing;
                    weapon.cooldown_ticks = ticks_per_round;
                    return FireResult::single();
                }
                if weapon.needs_primary_reload() {
                    weapon.state = WeaponState::Reloading;
                }
            }
            FireResult::none()
        }
        WeaponState::Firing => {
            weapon.state = WeaponState::Recovering;
            weapon.cooldown_ticks = recovery_ticks;
            FireResult::none()
        }
        _ => FireResult::none(),
    }
}

/// State for dual-wielded weapons (twofisted pistol class).
#[derive(Debug, Clone)]
pub struct DualWieldState {
    pub left: WeaponSlot,
    pub right: WeaponSlot,
}

impl DualWieldState {
    pub fn new(left: WeaponSlot, right: WeaponSlot) -> Self {
        Self { left, right }
    }

    /// Tick both weapons independently.
    /// `fire_primary` fires the right weapon, `fire_secondary` fires the left.
    /// Returns (right_fired, left_fired).
    pub fn tick(
        &mut self,
        fire_primary: bool,
        fire_secondary: bool,
        ticks_per_round: u16,
        recovery_ticks: u16,
    ) -> (bool, bool) {
        let right = tick_weapon(&mut self.right, fire_primary, ticks_per_round, recovery_ticks);
        let left = tick_weapon(&mut self.left, fire_secondary, ticks_per_round, recovery_ticks);
        (right, left)
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

    #[test]
    fn burst_fire_spawns_multiple() {
        let mut weapon = make_weapon(8, 0);
        let result = tick_weapon_burst(&mut weapon, true, 2, 3, 3, 0.05);
        assert!(result.fired);
        assert_eq!(result.projectile_count, 3);
        assert!((result.theta_error - 0.05).abs() < f32::EPSILON);
        assert_eq!(weapon.primary_magazine, 7); // only consumed 1 round
    }

    #[test]
    fn single_fire_no_burst() {
        let mut weapon = make_weapon(8, 0);
        let result = tick_weapon_burst(&mut weapon, true, 2, 3, 1, 0.0);
        assert!(result.fired);
        assert_eq!(result.projectile_count, 1);
    }

    #[test]
    fn dual_wield_independent_firing() {
        let mut dual = DualWieldState::new(
            make_weapon(8, 0), // left
            make_weapon(8, 0), // right
        );
        let (right, left) = dual.tick(true, false, 2, 3);
        assert!(right);
        assert!(!left);
        assert_eq!(dual.right.primary_magazine, 7);
        assert_eq!(dual.left.primary_magazine, 8);
    }

    #[test]
    fn dual_wield_both_fire() {
        let mut dual = DualWieldState::new(
            make_weapon(8, 0),
            make_weapon(8, 0),
        );
        let (right, left) = dual.tick(true, true, 2, 3);
        assert!(right);
        assert!(left);
        assert_eq!(dual.right.primary_magazine, 7);
        assert_eq!(dual.left.primary_magazine, 7);
    }
}
