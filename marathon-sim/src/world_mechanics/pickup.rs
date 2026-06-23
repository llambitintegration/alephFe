//! Item pickup eligibility and effect application.
//!
//! This module provides the two pure functions that drive item pickups:
//! [`can_pickup`] decides whether a player is close enough (and in a connected
//! polygon) to collect an item, and [`apply_item_effect`] mutates the player's
//! state for a collected item, enforcing the Marathon stat caps and reporting
//! whether the pickup actually did anything (so wasted pickups can be left in
//! the world).

use crate::components::{Health, InventoryItems, Oxygen, PowerupTimers, Shield};
use crate::player::inventory::{WeaponInventory, WeaponSlot, WeaponState};
use crate::world::MapGeometry;
use crate::world_mechanics::items::{
    ItemEffect, ITEM_EXTRAVISION, ITEM_INFRAVISION, ITEM_INVINCIBILITY, ITEM_INVISIBILITY,
};
use glam::Vec2;

/// Maximum 2D distance (in world units) at which an item can be collected.
pub const PICKUP_RANGE_WU: f32 = 1.0;

/// Maximum ammunition a single reserve (primary or secondary) can hold.
///
/// The engine does not yet carry a per-weapon ammunition maximum (weapon
/// definition tables are not wired into the sim), so box 4.7's "cap at maximum"
/// is enforced against this single shared ceiling. This is the documented
/// deviation from the box text, which assumes a per-weapon maximum.
pub const MAX_AMMO_RESERVE: u16 = 255;

/// Health ceiling (Marathon canonical).
pub const MAX_HEALTH: i16 = 150;

/// Oxygen ceiling (Marathon canonical).
pub const MAX_OXYGEN: i16 = 600;

/// Whether a player standing at `player_pos` in polygon `player_poly` can pick
/// up an item at `item_pos` in polygon `item_poly`.
///
/// Returns `true` iff the 2D distance is strictly less than [`PICKUP_RANGE_WU`]
/// **and** the item is in the same polygon or a polygon directly adjacent to
/// the player's polygon. Adjacency that crosses a solid line (a wall) does not
/// count: you cannot reach through a wall even if the polygons share an edge.
pub fn can_pickup(
    player_pos: Vec2,
    player_poly: usize,
    item_pos: Vec2,
    item_poly: usize,
    geometry: &MapGeometry,
) -> bool {
    if player_pos.distance(item_pos) >= PICKUP_RANGE_WU {
        return false;
    }
    if player_poly == item_poly {
        return true;
    }
    polygons_connected(player_poly, item_poly, geometry)
}

/// Whether `item_poly` is reachable from `player_poly` across a single shared,
/// non-solid line in the polygon adjacency graph.
fn polygons_connected(player_poly: usize, item_poly: usize, geometry: &MapGeometry) -> bool {
    let Some(neighbors) = geometry.polygon_adjacency.get(player_poly) else {
        return false;
    };
    for &(line_index, adj) in neighbors {
        if adj == Some(item_poly) {
            // Reaching across a solid line (a wall) is not allowed.
            let solid = geometry
                .line_solid
                .get(line_index)
                .copied()
                .unwrap_or(false);
            if !solid {
                return true;
            }
        }
    }
    false
}

/// Apply `effect` to the player's mutable state, enforcing Marathon stat caps.
///
/// Returns `true` if the pickup changed any state (and so should be consumed),
/// or `false` if it was wasted (e.g. health already at the cap) and should be
/// left in the world.
#[allow(clippy::too_many_arguments)]
pub fn apply_item_effect(
    health: &mut Health,
    shield: &mut Shield,
    oxygen: &mut Oxygen,
    weapons: &mut WeaponInventory,
    powerups: &mut PowerupTimers,
    inventory: &mut InventoryItems,
    effect: &ItemEffect,
) -> bool {
    match effect {
        ItemEffect::RestoreHealth { amount } => apply_health(health, *amount),
        ItemEffect::RestoreShield { amount } => apply_shield(shield, *amount),
        ItemEffect::RestoreOxygen { amount } => apply_oxygen(oxygen, *amount),
        ItemEffect::AddWeapon {
            weapon_definition_index,
        } => grant_weapon(weapons, *weapon_definition_index),
        ItemEffect::AddAmmo {
            weapon_definition_index,
            is_primary,
            amount,
        } => grant_ammo(weapons, *weapon_definition_index, *is_primary, *amount),
        ItemEffect::ActivatePowerup {
            powerup_type,
            duration_ticks,
        } => activate_powerup(powerups, *powerup_type, *duration_ticks),
        ItemEffect::AddInventoryItem { item_type } => add_inventory_item(inventory, *item_type),
    }
}

/// Box 4.3: clamp health at [`MAX_HEALTH`]. Wasted if already at the cap.
fn apply_health(health: &mut Health, amount: i16) -> bool {
    if health.0 >= MAX_HEALTH {
        return false;
    }
    health.0 = (health.0 + amount).min(MAX_HEALTH);
    true
}

/// Box 4.4: shield cap derived from the pickup amount (150/300/450). Wasted if
/// the shield is already at or above that derived cap.
fn apply_shield(shield: &mut Shield, amount: i16) -> bool {
    let cap = shield_cap(amount);
    if shield.0 >= cap {
        return false;
    }
    shield.0 = (shield.0 + amount).min(cap);
    true
}

/// Derive the shield cap from a pickup amount: 150 -> 150, 300 -> 300,
/// 450 -> 450. Any other amount caps at its own magnitude (so a custom shield
/// pickup behaves sensibly rather than being uncapped).
fn shield_cap(amount: i16) -> i16 {
    match amount {
        150 => 150,
        300 => 300,
        450 => 450,
        other => other,
    }
}

/// Box 4.5: clamp oxygen at [`MAX_OXYGEN`]. Wasted if already at the cap.
fn apply_oxygen(oxygen: &mut Oxygen, amount: i16) -> bool {
    if oxygen.0 >= MAX_OXYGEN {
        return false;
    }
    oxygen.0 = (oxygen.0 + amount).min(MAX_OXYGEN);
    true
}

/// Box 4.6: grant a weapon if the slot is empty; wasted if already held.
///
/// `WeaponInventory.weapons` is indexed by weapon definition index, with `None`
/// marking an empty slot. The vector is grown with `None` padding if the slot
/// index lies past its current end.
fn grant_weapon(weapons: &mut WeaponInventory, def_index: usize) -> bool {
    if def_index >= weapons.weapons.len() {
        weapons.weapons.resize(def_index + 1, None);
    }
    if weapons.weapons[def_index].is_some() {
        return false;
    }
    weapons.weapons[def_index] = Some(WeaponSlot {
        definition_index: def_index,
        primary_magazine: 0,
        primary_reserve: 0,
        secondary_magazine: 0,
        secondary_reserve: 0,
        state: WeaponState::Idle,
        cooldown_ticks: 0,
    });
    true
}

/// Box 4.7: add ammo to a weapon's reserve, capped at [`MAX_AMMO_RESERVE`].
/// Wasted if the targeted reserve is already at the cap or the weapon is not
/// held.
fn grant_ammo(
    weapons: &mut WeaponInventory,
    def_index: usize,
    is_primary: bool,
    amount: u16,
) -> bool {
    let Some(slot) = weapons
        .weapons
        .get_mut(def_index)
        .and_then(|slot| slot.as_mut())
    else {
        return false;
    };
    let reserve = if is_primary {
        &mut slot.primary_reserve
    } else {
        &mut slot.secondary_reserve
    };
    if *reserve >= MAX_AMMO_RESERVE {
        return false;
    }
    *reserve = reserve.saturating_add(amount).min(MAX_AMMO_RESERVE);
    true
}

/// Box 4.8: activate a timed powerup, taking the max of the current and new
/// duration. Always succeeds (powerups are always collected).
fn activate_powerup(powerups: &mut PowerupTimers, powerup_type: i16, duration: u16) -> bool {
    let field = match powerup_type {
        ITEM_INVINCIBILITY => &mut powerups.invincibility,
        ITEM_INVISIBILITY => &mut powerups.invisibility,
        ITEM_INFRAVISION => &mut powerups.infravision,
        ITEM_EXTRAVISION => &mut powerups.extravision,
        // Unknown powerup type: nothing to set, but still treated as collected.
        _ => return true,
    };
    *field = (*field).max(duration);
    true
}

/// Box 4.9: increment the per-type inventory count. Always succeeds.
fn add_inventory_item(inventory: &mut InventoryItems, item_type: i16) -> bool {
    *inventory.counts.entry(item_type).or_insert(0) += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapGeometry;

    // ── can_pickup() test fixtures (box 4.10) ─────────────────────────────

    /// Build a minimal two-polygon geometry where polygon 0 and polygon 1 share
    /// line 0. `wall` controls whether that shared line is solid.
    ///
    /// Polygon 2 is isolated (no adjacency) to model a non-adjacent polygon.
    fn two_poly_geometry(wall: bool) -> MapGeometry {
        MapGeometry {
            polygon_vertices: vec![Vec::new(), Vec::new(), Vec::new()],
            floor_heights: vec![0.0; 3],
            ceiling_heights: vec![10.0; 3],
            // poly 0 borders poly 1 via line 0; poly 1 borders poly 0 via line 0;
            // poly 2 has no neighbors.
            polygon_adjacency: vec![vec![(0, Some(1))], vec![(0, Some(0))], vec![(1, None)]],
            line_endpoints: vec![
                (Vec2::ZERO, Vec2::new(1.0, 0.0)),
                (Vec2::ZERO, Vec2::new(0.0, 1.0)),
            ],
            line_solid: vec![wall, false],
            line_transparent: vec![false, false],
            polygon_media_index: vec![-1; 3],
            polygon_floor_light_index: vec![-1; 3],
            polygon_ceiling_light_index: vec![-1; 3],
            polygon_types: vec![0; 3],
            polygon_permutations: vec![0; 3],
            line_side_indices: vec![(None, None), (None, None)],
            changed_polygons: vec![false; 3],
            has_changes: false,
        }
    }

    #[test]
    fn same_polygon_within_range() {
        let geo = two_poly_geometry(false);
        assert!(can_pickup(Vec2::ZERO, 0, Vec2::new(0.5, 0.0), 0, &geo));
    }

    #[test]
    fn same_polygon_out_of_range() {
        let geo = two_poly_geometry(false);
        // distance 1.0 is NOT < 1.0
        assert!(!can_pickup(Vec2::ZERO, 0, Vec2::new(1.0, 0.0), 0, &geo));
        // clearly out of range
        assert!(!can_pickup(Vec2::ZERO, 0, Vec2::new(5.0, 0.0), 0, &geo));
    }

    #[test]
    fn adjacent_polygon_within_range() {
        let geo = two_poly_geometry(false);
        assert!(can_pickup(Vec2::ZERO, 0, Vec2::new(0.5, 0.0), 1, &geo));
    }

    #[test]
    fn non_adjacent_polygon() {
        let geo = two_poly_geometry(false);
        // poly 2 is not adjacent to poly 0, even though item is in range.
        assert!(!can_pickup(Vec2::ZERO, 0, Vec2::new(0.5, 0.0), 2, &geo));
    }

    #[test]
    fn solid_wall_adjacent_blocks_pickup() {
        let geo = two_poly_geometry(true);
        // poly 1 is adjacent to poly 0 but the shared line is solid -> blocked.
        assert!(!can_pickup(Vec2::ZERO, 0, Vec2::new(0.5, 0.0), 1, &geo));
    }

    // ── apply_item_effect() test fixtures (box 4.11) ──────────────────────

    struct PlayerState {
        health: Health,
        shield: Shield,
        oxygen: Oxygen,
        weapons: WeaponInventory,
        powerups: PowerupTimers,
        inventory: InventoryItems,
    }

    fn player_state() -> PlayerState {
        PlayerState {
            health: Health(100),
            shield: Shield(0),
            oxygen: Oxygen(300),
            weapons: WeaponInventory::default(),
            powerups: PowerupTimers::default(),
            inventory: InventoryItems::default(),
        }
    }

    fn apply(p: &mut PlayerState, effect: &ItemEffect) -> bool {
        apply_item_effect(
            &mut p.health,
            &mut p.shield,
            &mut p.oxygen,
            &mut p.weapons,
            &mut p.powerups,
            &mut p.inventory,
            effect,
        )
    }

    #[test]
    fn health_normal_application() {
        let mut p = player_state();
        assert!(apply(&mut p, &ItemEffect::RestoreHealth { amount: 20 }));
        assert_eq!(p.health.0, 120);
    }

    #[test]
    fn health_clamps_at_cap() {
        let mut p = player_state();
        p.health = Health(140);
        assert!(apply(&mut p, &ItemEffect::RestoreHealth { amount: 40 }));
        assert_eq!(p.health.0, MAX_HEALTH);
    }

    #[test]
    fn health_wasted_at_cap() {
        let mut p = player_state();
        p.health = Health(MAX_HEALTH);
        assert!(!apply(&mut p, &ItemEffect::RestoreHealth { amount: 20 }));
        assert_eq!(p.health.0, MAX_HEALTH);
    }

    #[test]
    fn shield_normal_and_cap_derivation() {
        let mut p = player_state();
        assert!(apply(&mut p, &ItemEffect::RestoreShield { amount: 150 }));
        assert_eq!(p.shield.0, 150);
        // a 300 pickup raises the cap and tops up to 300
        assert!(apply(&mut p, &ItemEffect::RestoreShield { amount: 300 }));
        assert_eq!(p.shield.0, 300);
    }

    #[test]
    fn shield_wasted_when_at_derived_cap() {
        let mut p = player_state();
        p.shield = Shield(150);
        // 150 pickup, cap 150, already at cap -> wasted
        assert!(!apply(&mut p, &ItemEffect::RestoreShield { amount: 150 }));
        assert_eq!(p.shield.0, 150);
    }

    #[test]
    fn oxygen_normal_and_cap() {
        let mut p = player_state();
        assert!(apply(&mut p, &ItemEffect::RestoreOxygen { amount: 600 }));
        assert_eq!(p.oxygen.0, MAX_OXYGEN);
        // wasted at cap
        assert!(!apply(&mut p, &ItemEffect::RestoreOxygen { amount: 600 }));
        assert_eq!(p.oxygen.0, MAX_OXYGEN);
    }

    #[test]
    fn weapon_grant_and_reject_duplicate() {
        let mut p = player_state();
        assert!(apply(
            &mut p,
            &ItemEffect::AddWeapon {
                weapon_definition_index: 3
            }
        ));
        let slot = p.weapons.weapons[3].as_ref().expect("weapon granted");
        assert_eq!(slot.definition_index, 3);
        assert_eq!(slot.primary_magazine, 0);
        assert_eq!(slot.primary_reserve, 0);
        // picking up the same weapon again is wasted
        assert!(!apply(
            &mut p,
            &ItemEffect::AddWeapon {
                weapon_definition_index: 3
            }
        ));
    }

    #[test]
    fn ammo_grant_normal() {
        let mut p = player_state();
        apply(
            &mut p,
            &ItemEffect::AddWeapon {
                weapon_definition_index: 2,
            },
        );
        assert!(apply(
            &mut p,
            &ItemEffect::AddAmmo {
                weapon_definition_index: 2,
                is_primary: true,
                amount: 20,
            }
        ));
        assert_eq!(p.weapons.weapons[2].as_ref().unwrap().primary_reserve, 20);
    }

    #[test]
    fn ammo_grant_wasted_without_weapon() {
        let mut p = player_state();
        // no weapon at index 5 -> ammo wasted
        assert!(!apply(
            &mut p,
            &ItemEffect::AddAmmo {
                weapon_definition_index: 5,
                is_primary: true,
                amount: 60,
            }
        ));
    }

    #[test]
    fn ammo_caps_at_max_and_wastes_when_full() {
        let mut p = player_state();
        apply(
            &mut p,
            &ItemEffect::AddWeapon {
                weapon_definition_index: 1,
            },
        );
        // fill to cap
        p.weapons.weapons[1].as_mut().unwrap().primary_reserve = MAX_AMMO_RESERVE;
        assert!(!apply(
            &mut p,
            &ItemEffect::AddAmmo {
                weapon_definition_index: 1,
                is_primary: true,
                amount: 8,
            }
        ));
        assert_eq!(
            p.weapons.weapons[1].as_ref().unwrap().primary_reserve,
            MAX_AMMO_RESERVE
        );
    }

    #[test]
    fn powerup_activation_takes_max() {
        let mut p = player_state();
        assert!(apply(
            &mut p,
            &ItemEffect::ActivatePowerup {
                powerup_type: ITEM_INVINCIBILITY,
                duration_ticks: 100,
            }
        ));
        assert_eq!(p.powerups.invincibility, 100);
        // a shorter duration does not lower the timer, but still "applies"
        assert!(apply(
            &mut p,
            &ItemEffect::ActivatePowerup {
                powerup_type: ITEM_INVINCIBILITY,
                duration_ticks: 50,
            }
        ));
        assert_eq!(p.powerups.invincibility, 100);
        // a longer duration raises it
        assert!(apply(
            &mut p,
            &ItemEffect::ActivatePowerup {
                powerup_type: ITEM_INVINCIBILITY,
                duration_ticks: 200,
            }
        ));
        assert_eq!(p.powerups.invincibility, 200);
    }

    #[test]
    fn inventory_item_increments_count() {
        let mut p = player_state();
        assert!(apply(
            &mut p,
            &ItemEffect::AddInventoryItem { item_type: 30 }
        ));
        assert!(apply(
            &mut p,
            &ItemEffect::AddInventoryItem { item_type: 30 }
        ));
        assert_eq!(p.inventory.counts.get(&30).copied(), Some(2));
    }
}
