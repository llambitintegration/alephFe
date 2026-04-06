/// Item type constants (matching Marathon's item numbering).
pub const ITEM_FISTS: i16 = 0;
pub const ITEM_PISTOL: i16 = 1;
pub const ITEM_FUSION_PISTOL: i16 = 2;
pub const ITEM_ASSAULT_RIFLE: i16 = 3;
pub const ITEM_MISSILE_LAUNCHER: i16 = 4;
pub const ITEM_FLAMETHROWER: i16 = 5;
pub const ITEM_ALIEN_WEAPON: i16 = 6;
pub const ITEM_SHOTGUN: i16 = 7;
pub const ITEM_SMGS: i16 = 8;

pub const ITEM_PISTOL_AMMO: i16 = 10;
pub const ITEM_FUSION_AMMO: i16 = 11;
pub const ITEM_AR_AMMO: i16 = 12;
pub const ITEM_AR_GRENADES: i16 = 13;
pub const ITEM_MISSILE_AMMO: i16 = 14;
pub const ITEM_FLAMETHROWER_AMMO: i16 = 15;
pub const ITEM_ALIEN_AMMO: i16 = 16;
pub const ITEM_SHOTGUN_AMMO: i16 = 17;
pub const ITEM_SMG_AMMO: i16 = 18;

pub const ITEM_HEALTH_MINOR: i16 = 20;
pub const ITEM_HEALTH_MAJOR: i16 = 21;
pub const ITEM_OXYGEN: i16 = 22;
pub const ITEM_SHIELD_1X: i16 = 23;
pub const ITEM_SHIELD_2X: i16 = 24;
pub const ITEM_SHIELD_3X: i16 = 25;

pub const ITEM_INVINCIBILITY: i16 = 26;
pub const ITEM_INVISIBILITY: i16 = 27;
pub const ITEM_INFRAVISION: i16 = 28;
pub const ITEM_EXTRAVISION: i16 = 29;

pub const ITEM_UPLINK_CHIP: i16 = 30;
pub const ITEM_LIGHT_BLUE_BALL: i16 = 31;
pub const ITEM_THE_BALL: i16 = 32;
pub const ITEM_VIOLET_BALL: i16 = 33;
pub const ITEM_YELLOW_BALL: i16 = 34;
pub const ITEM_BROWN_BALL: i16 = 35;
pub const ITEM_ORANGE_BALL: i16 = 36;
pub const ITEM_BLUE_BALL: i16 = 37;
pub const ITEM_GREEN_BALL: i16 = 38;

/// Timer-based item respawn for multiplayer modes.
#[derive(Debug, Clone)]
pub struct ItemRespawnState {
    pub item_type: i16,
    pub remaining: u16,
}

impl ItemRespawnState {
    pub fn new(item_type: i16, delay_ticks: u16) -> Self {
        Self {
            item_type,
            remaining: delay_ticks,
        }
    }

    /// Tick the respawn timer. Returns true when the item should respawn.
    pub fn tick(&mut self) -> bool {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.remaining == 0
        } else {
            false
        }
    }
}

/// Effect of picking up an item.
#[derive(Debug, Clone)]
pub enum ItemEffect {
    /// Add a weapon to inventory.
    AddWeapon { weapon_definition_index: usize },
    /// Add ammunition to reserves.
    AddAmmo { weapon_definition_index: usize, is_primary: bool, amount: u16 },
    /// Restore health.
    RestoreHealth { amount: i16 },
    /// Restore shield.
    RestoreShield { amount: i16 },
    /// Restore oxygen.
    RestoreOxygen { amount: i16 },
    /// Add an inventory item (keycard, powerup).
    AddInventoryItem { item_type: i16 },
}

/// Determine what happens when an item is picked up.
pub fn item_effect(item_type: i16) -> Option<ItemEffect> {
    match item_type {
        ITEM_PISTOL => Some(ItemEffect::AddWeapon { weapon_definition_index: 1 }),
        ITEM_FUSION_PISTOL => Some(ItemEffect::AddWeapon { weapon_definition_index: 2 }),
        ITEM_ASSAULT_RIFLE => Some(ItemEffect::AddWeapon { weapon_definition_index: 3 }),
        ITEM_MISSILE_LAUNCHER => Some(ItemEffect::AddWeapon { weapon_definition_index: 4 }),
        ITEM_FLAMETHROWER => Some(ItemEffect::AddWeapon { weapon_definition_index: 5 }),
        ITEM_ALIEN_WEAPON => Some(ItemEffect::AddWeapon { weapon_definition_index: 6 }),
        ITEM_SHOTGUN => Some(ItemEffect::AddWeapon { weapon_definition_index: 7 }),

        ITEM_PISTOL_AMMO => Some(ItemEffect::AddAmmo { weapon_definition_index: 1, is_primary: true, amount: 8 }),
        ITEM_FUSION_AMMO => Some(ItemEffect::AddAmmo { weapon_definition_index: 2, is_primary: true, amount: 20 }),
        ITEM_AR_AMMO => Some(ItemEffect::AddAmmo { weapon_definition_index: 3, is_primary: true, amount: 52 }),
        ITEM_AR_GRENADES => Some(ItemEffect::AddAmmo { weapon_definition_index: 3, is_primary: false, amount: 7 }),
        ITEM_MISSILE_AMMO => Some(ItemEffect::AddAmmo { weapon_definition_index: 4, is_primary: true, amount: 2 }),
        ITEM_SHOTGUN_AMMO => Some(ItemEffect::AddAmmo { weapon_definition_index: 7, is_primary: true, amount: 2 }),

        ITEM_HEALTH_MINOR => Some(ItemEffect::RestoreHealth { amount: 20 }),
        ITEM_HEALTH_MAJOR => Some(ItemEffect::RestoreHealth { amount: 40 }),
        ITEM_OXYGEN => Some(ItemEffect::RestoreOxygen { amount: 600 }),

        ITEM_SHIELD_1X => Some(ItemEffect::RestoreShield { amount: 150 }),
        ITEM_SHIELD_2X => Some(ItemEffect::RestoreShield { amount: 300 }),
        ITEM_SHIELD_3X => Some(ItemEffect::RestoreShield { amount: 450 }),

        ITEM_UPLINK_CHIP | ITEM_LIGHT_BLUE_BALL..=ITEM_GREEN_BALL => {
            Some(ItemEffect::AddInventoryItem { item_type })
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_pickup_effect() {
        match item_effect(ITEM_SHOTGUN) {
            Some(ItemEffect::AddWeapon { weapon_definition_index }) => {
                assert_eq!(weapon_definition_index, 7);
            }
            other => panic!("expected AddWeapon, got {other:?}"),
        }
    }

    #[test]
    fn ammo_pickup_effect() {
        match item_effect(ITEM_AR_AMMO) {
            Some(ItemEffect::AddAmmo { amount, is_primary, .. }) => {
                assert_eq!(amount, 52);
                assert!(is_primary);
            }
            other => panic!("expected AddAmmo, got {other:?}"),
        }
    }

    #[test]
    fn health_pickup_effect() {
        match item_effect(ITEM_HEALTH_MAJOR) {
            Some(ItemEffect::RestoreHealth { amount }) => {
                assert_eq!(amount, 40);
            }
            other => panic!("expected RestoreHealth, got {other:?}"),
        }
    }

    #[test]
    fn unknown_item_type() {
        assert!(item_effect(999).is_none());
    }

    #[test]
    fn respawn_timer_counts_down() {
        let mut state = ItemRespawnState::new(5, 100);
        assert!(!state.tick());
        assert!(!state.tick());
        assert_eq!(state.remaining, 98);
    }

    #[test]
    fn respawn_timer_fires() {
        let mut state = ItemRespawnState::new(5, 2);
        assert!(!state.tick());
        assert!(state.tick());
    }
}
