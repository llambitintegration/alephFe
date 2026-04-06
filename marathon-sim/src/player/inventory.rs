use serde::{Deserialize, Serialize};

/// Player weapon inventory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeaponInventory {
    /// Weapons the player has (by definition index). None = empty slot.
    pub weapons: Vec<Option<WeaponSlot>>,
    /// Currently equipped weapon index into `weapons`.
    pub current_weapon: usize,
    /// Weapon switch cooldown (ticks remaining).
    pub switch_cooldown: u16,
}

/// State of a single weapon in inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponSlot {
    pub definition_index: usize,
    pub primary_magazine: u16,
    pub primary_reserve: u16,
    pub secondary_magazine: u16,
    pub secondary_reserve: u16,
    pub state: WeaponState,
    pub cooldown_ticks: u16,
}

/// Weapon operational state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponState {
    Idle,
    Firing,
    Recovering,
    Reloading,
    Switching,
}

impl WeaponInventory {
    /// Cycle to the next available weapon.
    pub fn cycle_forward(&mut self, ready_ticks: u16) {
        if self.weapons.is_empty() {
            return;
        }
        let start = self.current_weapon;
        let mut idx = (start + 1) % self.weapons.len();
        while idx != start {
            if self.weapons[idx].is_some() {
                self.current_weapon = idx;
                self.switch_cooldown = ready_ticks;
                return;
            }
            idx = (idx + 1) % self.weapons.len();
        }
    }

    /// Cycle to the previous available weapon.
    pub fn cycle_backward(&mut self, ready_ticks: u16) {
        if self.weapons.is_empty() {
            return;
        }
        let start = self.current_weapon;
        let len = self.weapons.len();
        let mut idx = (start + len - 1) % len;
        while idx != start {
            if self.weapons[idx].is_some() {
                self.current_weapon = idx;
                self.switch_cooldown = ready_ticks;
                return;
            }
            idx = (idx + len - 1) % len;
        }
    }

    /// Get the currently equipped weapon, if any.
    pub fn current(&self) -> Option<&WeaponSlot> {
        self.weapons.get(self.current_weapon)?.as_ref()
    }

    /// Get a mutable ref to the current weapon.
    pub fn current_mut(&mut self) -> Option<&mut WeaponSlot> {
        self.weapons.get_mut(self.current_weapon)?.as_mut()
    }
}

impl WeaponSlot {
    /// Try to consume one round from the primary magazine. Returns true if successful.
    pub fn consume_primary(&mut self) -> bool {
        if self.primary_magazine > 0 {
            self.primary_magazine -= 1;
            true
        } else {
            false
        }
    }

    /// Try to reload primary from reserves.
    pub fn reload_primary(&mut self, rounds_per_magazine: u16) {
        let needed = rounds_per_magazine.saturating_sub(self.primary_magazine);
        let available = needed.min(self.primary_reserve);
        self.primary_magazine += available;
        self.primary_reserve -= available;
    }

    /// Whether primary magazine is empty and reserves exist.
    pub fn needs_primary_reload(&self) -> bool {
        self.primary_magazine == 0 && self.primary_reserve > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_weapon(def_idx: usize, primary_mag: u16, primary_reserve: u16) -> WeaponSlot {
        WeaponSlot {
            definition_index: def_idx,
            primary_magazine: primary_mag,
            primary_reserve,
            secondary_magazine: 0,
            secondary_reserve: 0,
            state: WeaponState::Idle,
            cooldown_ticks: 0,
        }
    }

    #[test]
    fn consume_primary_ammo() {
        let mut weapon = make_weapon(0, 8, 16);
        assert!(weapon.consume_primary());
        assert_eq!(weapon.primary_magazine, 7);
    }

    #[test]
    fn consume_empty_magazine_fails() {
        let mut weapon = make_weapon(0, 0, 16);
        assert!(!weapon.consume_primary());
    }

    #[test]
    fn reload_from_reserve() {
        let mut weapon = make_weapon(0, 0, 16);
        weapon.reload_primary(8);
        assert_eq!(weapon.primary_magazine, 8);
        assert_eq!(weapon.primary_reserve, 8);
    }

    #[test]
    fn reload_partial_reserve() {
        let mut weapon = make_weapon(0, 0, 3);
        weapon.reload_primary(8);
        assert_eq!(weapon.primary_magazine, 3);
        assert_eq!(weapon.primary_reserve, 0);
    }

    #[test]
    fn cycle_weapons() {
        let mut inv = WeaponInventory {
            weapons: vec![
                Some(make_weapon(0, 8, 0)),
                None,
                Some(make_weapon(2, 4, 0)),
            ],
            current_weapon: 0,
            switch_cooldown: 0,
        };

        inv.cycle_forward(10);
        assert_eq!(inv.current_weapon, 2); // skips None at index 1
        assert_eq!(inv.switch_cooldown, 10);

        inv.cycle_backward(5);
        assert_eq!(inv.current_weapon, 0);
    }
}
