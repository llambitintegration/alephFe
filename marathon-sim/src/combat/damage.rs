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
}
