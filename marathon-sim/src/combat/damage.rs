use marathon_formats::DamageDefinition;
use rand::Rng;

/// Calculate the damage amount from a DamageDefinition.
///
/// damage = (base + random(0, random)) * scale
/// Then apply immunities (zero) and weaknesses (double).
pub fn calculate_damage(
    def: &DamageDefinition,
    target_immunities: u32,
    target_weaknesses: u32,
    rng: &mut impl Rng,
) -> i16 {
    let damage_type = def.damage_type;

    // Check immunity
    if damage_type >= 0 && damage_type < 32 {
        let type_bit = 1u32 << damage_type;
        if target_immunities & type_bit != 0 {
            return 0;
        }
    }

    // Base damage + random component
    let random_add = if def.random > 0 {
        rng.gen_range(0..=def.random)
    } else {
        0
    };
    let raw_damage = ((def.base + random_add) as f32 * def.scale) as i16;

    // Check weakness (double damage)
    if damage_type >= 0 && damage_type < 32 {
        let type_bit = 1u32 << damage_type;
        if target_weaknesses & type_bit != 0 {
            return raw_damage.saturating_mul(2);
        }
    }

    raw_damage
}

/// Calculate area-of-effect damage scaled by distance.
///
/// Full damage at center, zero at the edge of the radius.
pub fn calculate_aoe_damage(
    base_damage: i16,
    distance: f32,
    aoe_radius: f32,
) -> i16 {
    if distance >= aoe_radius || aoe_radius <= 0.0 {
        return 0;
    }
    let scale = 1.0 - (distance / aoe_radius);
    (base_damage as f32 * scale) as i16
}

/// Result of applying damage to an entity.
#[derive(Debug, Clone)]
pub struct DamageResult {
    /// Amount actually applied to shield.
    pub shield_damage: i16,
    /// Amount actually applied to health.
    pub health_damage: i16,
    /// Whether the entity was killed (health reached 0 or below).
    pub killed: bool,
}

/// Apply damage to an entity, subtracting from shield first, then health.
pub fn apply_damage(
    damage: i16,
    current_health: i16,
    current_shield: i16,
) -> (i16, i16, DamageResult) {
    if damage <= 0 {
        return (
            current_health,
            current_shield,
            DamageResult {
                shield_damage: 0,
                health_damage: 0,
                killed: false,
            },
        );
    }

    let mut remaining = damage;
    let mut shield = current_shield;
    let mut health = current_health;
    let mut shield_damage = 0i16;
    let mut health_damage = 0i16;

    // Shield absorbs damage first
    if shield > 0 {
        let absorbed = remaining.min(shield);
        shield -= absorbed;
        remaining -= absorbed;
        shield_damage = absorbed;
    }

    // Remaining damage goes to health
    if remaining > 0 {
        health_damage = remaining;
        health -= remaining;
    }

    let killed = health <= 0;

    (
        health,
        shield,
        DamageResult {
            shield_damage,
            health_damage,
            killed,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_damage(base: i16, random: i16, scale: f32, damage_type: i16) -> DamageDefinition {
        DamageDefinition {
            damage_type,
            flags: 0,
            base,
            random,
            scale,
        }
    }

    #[test]
    fn basic_damage_calculation() {
        let mut rng = StdRng::seed_from_u64(42);
        let def = make_damage(20, 0, 1.0, 0);
        let dmg = calculate_damage(&def, 0, 0, &mut rng);
        assert_eq!(dmg, 20);
    }

    #[test]
    fn damage_with_random() {
        let mut rng = StdRng::seed_from_u64(42);
        let def = make_damage(20, 10, 1.0, 0);
        let dmg = calculate_damage(&def, 0, 0, &mut rng);
        assert!(dmg >= 20 && dmg <= 30);
    }

    #[test]
    fn damage_with_scale() {
        let mut rng = StdRng::seed_from_u64(42);
        let def = make_damage(20, 0, 2.0, 0);
        let dmg = calculate_damage(&def, 0, 0, &mut rng);
        assert_eq!(dmg, 40);
    }

    #[test]
    fn immune_to_damage_type() {
        let mut rng = StdRng::seed_from_u64(42);
        let def = make_damage(50, 0, 1.0, 3);
        let dmg = calculate_damage(&def, 1 << 3, 0, &mut rng);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn weak_to_damage_type() {
        let mut rng = StdRng::seed_from_u64(42);
        let def = make_damage(20, 0, 1.0, 5);
        let dmg = calculate_damage(&def, 0, 1 << 5, &mut rng);
        assert_eq!(dmg, 40); // doubled
    }

    #[test]
    fn aoe_full_damage_at_center() {
        assert_eq!(calculate_aoe_damage(100, 0.0, 5.0), 100);
    }

    #[test]
    fn aoe_half_damage_at_half_radius() {
        assert_eq!(calculate_aoe_damage(100, 2.5, 5.0), 50);
    }

    #[test]
    fn aoe_zero_at_edge() {
        assert_eq!(calculate_aoe_damage(100, 5.0, 5.0), 0);
    }

    #[test]
    fn aoe_zero_beyond_radius() {
        assert_eq!(calculate_aoe_damage(100, 8.0, 5.0), 0);
    }

    #[test]
    fn damage_absorbed_by_shield() {
        let (health, shield, result) = apply_damage(30, 100, 50);
        assert_eq!(shield, 20);
        assert_eq!(health, 100);
        assert_eq!(result.shield_damage, 30);
        assert_eq!(result.health_damage, 0);
        assert!(!result.killed);
    }

    #[test]
    fn damage_spills_to_health() {
        let (health, shield, result) = apply_damage(80, 100, 50);
        assert_eq!(shield, 0);
        assert_eq!(health, 70);
        assert_eq!(result.shield_damage, 50);
        assert_eq!(result.health_damage, 30);
        assert!(!result.killed);
    }

    #[test]
    fn lethal_damage() {
        let (health, shield, result) = apply_damage(200, 100, 50);
        assert_eq!(shield, 0);
        assert!(health <= 0);
        assert!(result.killed);
    }

    #[test]
    fn zero_damage_no_effect() {
        let (health, shield, result) = apply_damage(0, 100, 50);
        assert_eq!(health, 100);
        assert_eq!(shield, 50);
        assert!(!result.killed);
    }
}
