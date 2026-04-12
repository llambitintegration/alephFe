use glam::{Vec2, Vec3};

use crate::collision::segment_intersection;

/// Projectile behavior flag bit constants from Marathon's weapon_definitions.h.
/// These are tested against `ProjectileDefinition.flags` (a u32 bitmask).
#[allow(non_snake_case)]
pub mod ProjectileFlags {
    pub const GUIDED: u32 = 1 << 0;
    pub const STOP_WHEN_ANIMATION_LOOPS: u32 = 1 << 1;
    pub const PERSISTENT: u32 = 1 << 2;
    pub const ALIEN_PROJECTILE: u32 = 1 << 3;
    pub const AFFECTED_BY_GRAVITY: u32 = 1 << 4;
    pub const REBOUNDS_FROM_FLOOR: u32 = 1 << 5;
    pub const BLEEDING: u32 = 1 << 6;
    pub const USUALLY_PASS_TRANSPARENT_SIDE: u32 = 1 << 7;
    pub const SOMETIMES_PASS_TRANSPARENT_SIDE: u32 = 1 << 8;
    pub const DOUBLE_GRAVITY: u32 = 1 << 9;
    pub const REBOUNDS_FROM_WALLS: u32 = 1 << 10;
    pub const CAN_TOGGLE_CONTROL_PANELS: u32 = 1 << 11;
    pub const POSITIVE_VERTICAL_ERROR: u32 = 1 << 12;
    pub const MELEE_PROJECTILE: u32 = 1 << 13;
    pub const PERSISTENT_AND_VIRULENT: u32 = 1 << 14;
    pub const BECOMES_ITEM_ON_DETONATION: u32 = 1 << 15;
    pub const BLEEDING_PROJECTILE: u32 = 1 << 16;
    pub const HORIZONTAL_WANDER: u32 = 1 << 17;
    pub const VERTICAL_WANDER: u32 = 1 << 18;
    pub const HALF_GRAVITY: u32 = 1 << 19;
    pub const PASSES_MEDIA_BOUNDARY: u32 = 1 << 20;
}

/// Advance a projectile's position by its velocity for one tick.
/// Returns the new position and distance traveled this tick.
pub fn advance_projectile(position: Vec3, velocity: Vec3) -> (Vec3, f32) {
    let distance = velocity.length();
    (position + velocity, distance)
}

/// Apply gravity to a projectile's velocity (Z-axis downward).
pub fn apply_projectile_gravity(velocity: Vec3, gravity: f32) -> Vec3 {
    Vec3::new(velocity.x, velocity.y, velocity.z - gravity)
}

/// Adjust a homing projectile's velocity toward a target.
///
/// `turning_rate`: maximum angle change per tick in radians.
pub fn apply_homing(velocity: Vec3, position: Vec3, target: Vec3, turning_rate: f32) -> Vec3 {
    let to_target = target - position;
    let speed = velocity.length();

    if speed < 1e-6 || to_target.length() < 1e-6 {
        return velocity;
    }

    let desired_dir = to_target.normalize();
    let current_dir = velocity.normalize();

    // Compute the angle between current direction and desired
    let dot = current_dir.dot(desired_dir).clamp(-1.0, 1.0);
    let angle = dot.acos();

    if angle < 1e-6 {
        return velocity;
    }

    // Clamp the turn to turning_rate
    let t = (turning_rate / angle).min(1.0);
    let new_dir = current_dir.lerp(desired_dir, t).normalize();

    new_dir * speed
}

/// Reflect a projectile's velocity off a wall defined by two endpoints.
/// The XY velocity is reflected across the wall normal; Z is preserved.
pub fn reflect_velocity_wall(velocity: Vec3, wall_a: Vec2, wall_b: Vec2) -> Vec3 {
    let wall_dir = (wall_b - wall_a).normalize_or_zero();
    // Wall normal (perpendicular in 2D)
    let normal = Vec2::new(-wall_dir.y, wall_dir.x);
    let vel_2d = Vec2::new(velocity.x, velocity.y);
    // Reflect: v' = v - 2(v·n)n
    let reflected = vel_2d - 2.0 * vel_2d.dot(normal) * normal;
    Vec3::new(reflected.x, reflected.y, velocity.z)
}

/// Reflect a projectile's velocity off a floor (negate Z with energy loss).
/// `energy_loss` is 0.0..1.0 where 0.0 = perfect bounce, 1.0 = full absorption.
pub fn reflect_velocity_floor(velocity: Vec3, energy_loss: f32) -> Vec3 {
    Vec3::new(velocity.x, velocity.y, -velocity.z * (1.0 - energy_loss))
}

/// Result of checking a projectile against walls.
#[derive(Debug, Clone)]
pub enum WallHitResult {
    /// No wall hit.
    Clear,
    /// Hit a wall at the given position.
    Hit {
        /// Position where the projectile hit the wall.
        hit_point: Vec3,
        /// Line index of the wall hit.
        line_index: usize,
    },
}

/// Check if a projectile's movement crosses any solid wall in the current polygon.
///
/// Returns the closest wall hit, if any.
pub fn check_projectile_wall_collision(
    old_pos: Vec2,
    new_pos: Vec2,
    old_z: f32,
    new_z: f32,
    current_polygon: usize,
    polygon_adjacency: &[Vec<(usize, Option<usize>)>],
    line_endpoints: &[(Vec2, Vec2)],
    line_solid: &[bool],
) -> WallHitResult {
    let mut closest_t = f32::MAX;
    let mut closest_hit = WallHitResult::Clear;

    for &(line_idx, adj) in &polygon_adjacency[current_polygon] {
        let (la, lb) = line_endpoints[line_idx];

        if let Some(hit) = segment_intersection(old_pos, new_pos, la, lb) {
            let is_passable = adj.is_some() && !line_solid[line_idx];
            if !is_passable && hit.t < closest_t {
                closest_t = hit.t;
                let hit_z = old_z + (new_z - old_z) * hit.t;
                closest_hit = WallHitResult::Hit {
                    hit_point: Vec3::new(hit.point.x, hit.point.y, hit_z),
                    line_index: line_idx,
                };
            }
        }
    }

    closest_hit
}

/// Result of checking a projectile against entities.
#[derive(Debug, Clone)]
pub struct EntityHitResult {
    /// Index into the entities slice.
    pub entity_index: usize,
    /// Hit position.
    pub hit_point: Vec3,
}

/// Check if a projectile's movement path intersects any entity's collision radius.
///
/// Uses 2D circle-line intersection (XY plane) then checks Z overlap.
pub fn check_projectile_entity_collision(
    old_pos: Vec3,
    new_pos: Vec3,
    entities: &[(Vec2, f32, f32, f32)], // (center_2d, radius, z_bottom, z_top)
) -> Option<EntityHitResult> {
    let dir = new_pos - old_pos;
    let len = dir.length();
    if len < 1e-6 {
        return None;
    }

    let dir_2d = Vec2::new(dir.x, dir.y);
    let dir_2d_len = dir_2d.length();

    let mut closest_t = f32::MAX;
    let mut result = None;

    for (i, &(center, radius, z_bot, z_top)) in entities.iter().enumerate() {
        // 2D closest point on ray to entity center
        let to_center = center - Vec2::new(old_pos.x, old_pos.y);

        if dir_2d_len < 1e-6 {
            // Projectile not moving in XY; check direct overlap
            if to_center.length() <= radius {
                let t = 0.0;
                if t < closest_t {
                    let hit_z = old_pos.z;
                    if hit_z >= z_bot && hit_z <= z_top {
                        closest_t = t;
                        result = Some(EntityHitResult {
                            entity_index: i,
                            hit_point: old_pos,
                        });
                    }
                }
            }
            continue;
        }

        let dir_norm = dir_2d / dir_2d_len;
        let t_closest = to_center.dot(dir_norm);

        // Clamp to segment
        let t_clamped = t_closest.clamp(0.0, dir_2d_len);
        let closest_point = Vec2::new(old_pos.x, old_pos.y) + dir_norm * t_clamped;
        let dist = closest_point.distance(center);

        if dist <= radius {
            // Find the entry point: t_closest - sqrt(r^2 - dist^2)
            let offset = (radius * radius - dist * dist).sqrt();
            let t_entry = (t_closest - offset).max(0.0);

            if t_entry <= dir_2d_len && t_entry < closest_t {
                let frac = t_entry / dir_2d_len;
                let hit_z = old_pos.z + dir.z * frac;
                if hit_z >= z_bot && hit_z <= z_top {
                    closest_t = t_entry;
                    let hit_pos = old_pos + dir * frac;
                    result = Some(EntityHitResult {
                        entity_index: i,
                        hit_point: hit_pos,
                    });
                }
            }
        }
    }

    result
}

/// Result of a projectile detonation computation.
/// Separates "compute what happens" from "mutate the world".
#[derive(Debug, Clone)]
pub struct DetonationResult {
    /// Effect entity to spawn at detonation point (position, effect definition index).
    pub effect_to_spawn: Option<(Vec3, usize)>,
    /// AoE damage to apply: (entity_index in the provided list, damage amount).
    pub aoe_damages: Vec<(usize, i16)>,
    /// Direct hit damage to the entity that was hit (entity_index, damage amount).
    pub direct_hit_damage: Option<(usize, i16)>,
    /// Sound event to emit (sound_index, position).
    pub sound_event: Option<(usize, Vec3)>,
}

/// Compute the results of a projectile detonation without mutating world state.
///
/// * `hit_point` - Where the projectile detonated.
/// * `def` - The ProjectileDefinition for this projectile type.
/// * `hit_entity_idx` - Index into `entities_in_radius` of the directly-hit entity, if any.
/// * `entities_in_radius` - List of (position, immunities, weaknesses) for AoE candidates.
/// * `is_submerged` - Whether the detonation point is below a media surface.
/// * `rng` - Deterministic RNG for damage randomness.
pub fn compute_detonation(
    hit_point: Vec3,
    def: &marathon_formats::physics::ProjectileDefinition,
    hit_entity_idx: Option<usize>,
    entities_in_radius: &[(Vec3, u32, u32)], // (position, immunities, weaknesses)
    is_submerged: bool,
    rng: &mut impl rand::Rng,
) -> DetonationResult {
    use crate::combat::damage::{calculate_aoe_damage, calculate_damage};

    // Select detonation effect
    let effect_idx = if is_submerged && def.media_detonation_effect >= 0 {
        def.media_detonation_effect
    } else {
        def.detonation_effect
    };
    let effect_to_spawn = if effect_idx >= 0 {
        Some((hit_point, effect_idx as usize))
    } else {
        None
    };

    // Direct hit damage
    let direct_hit_damage = hit_entity_idx.map(|idx| {
        let (_, immunities, weaknesses) = entities_in_radius[idx];
        let dmg = calculate_damage(&def.damage, immunities, weaknesses, rng);
        (idx, dmg)
    });

    // AoE damage
    let mut aoe_damages = Vec::new();
    let aoe_radius = def.area_of_effect as f32 / 1024.0;
    if aoe_radius > 0.0 {
        for (i, (entity_pos, immunities, weaknesses)) in entities_in_radius.iter().enumerate() {
            let distance = entity_pos.distance(hit_point);
            if distance < aoe_radius {
                let base_dmg = calculate_damage(&def.damage, *immunities, *weaknesses, rng);
                let scaled = calculate_aoe_damage(base_dmg, distance, aoe_radius);
                if scaled > 0 {
                    aoe_damages.push((i, scaled));
                }
            }
        }
    }

    DetonationResult {
        effect_to_spawn,
        aoe_damages,
        direct_hit_damage,
        sound_event: None, // Sound events handled at integration layer
    }
}

/// Check if a projectile has exceeded its maximum range.
pub fn check_range_limit(distance_traveled: f32, maximum_range: f32) -> bool {
    maximum_range > 0.0 && distance_traveled >= maximum_range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_straight_line() {
        let (new_pos, dist) = advance_projectile(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert_eq!(new_pos, Vec3::new(1.0, 0.0, 0.0));
        assert!((dist - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gravity_decreases_z_velocity() {
        let vel = apply_projectile_gravity(Vec3::new(1.0, 0.0, 0.0), 0.1);
        assert_eq!(vel.x, 1.0);
        assert!((vel.z - (-0.1)).abs() < f32::EPSILON);
    }

    #[test]
    fn homing_turns_toward_target() {
        let vel = Vec3::new(1.0, 0.0, 0.0); // moving east
        let pos = Vec3::ZERO;
        let target = Vec3::new(0.0, 10.0, 0.0); // target to the north

        let new_vel = apply_homing(vel, pos, target, 0.5);
        // Should have turned toward north (positive Y)
        assert!(new_vel.y > 0.0);
        // Speed should be preserved
        assert!((new_vel.length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn projectile_hits_wall() {
        // Polygon 0 has a solid wall (line 0) at x=1
        let adjacency = vec![
            vec![(0, None)],
        ];
        let endpoints = vec![(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0))];
        let solid = vec![true];

        let result = check_projectile_wall_collision(
            Vec2::new(0.5, 0.5),
            Vec2::new(1.5, 0.5),
            0.5, 0.5,
            0,
            &adjacency,
            &endpoints,
            &solid,
        );

        match result {
            WallHitResult::Hit { hit_point, .. } => {
                assert!((hit_point.x - 1.0).abs() < 0.01);
            }
            WallHitResult::Clear => panic!("expected wall hit"),
        }
    }

    #[test]
    fn projectile_passes_through_passable() {
        let adjacency = vec![
            vec![(0, Some(1))],
        ];
        let endpoints = vec![(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0))];
        let solid = vec![false];

        let result = check_projectile_wall_collision(
            Vec2::new(0.5, 0.5),
            Vec2::new(1.5, 0.5),
            0.5, 0.5,
            0,
            &adjacency,
            &endpoints,
            &solid,
        );

        matches!(result, WallHitResult::Clear);
    }

    #[test]
    fn projectile_hits_entity() {
        let entities = vec![
            (Vec2::new(5.0, 0.0), 0.5, 0.0, 2.0), // entity at x=5, radius=0.5
        ];
        let result = check_projectile_entity_collision(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(10.0, 0.0, 1.0),
            &entities,
        );
        assert!(result.is_some());
        let hit = result.unwrap();
        assert_eq!(hit.entity_index, 0);
        assert!((hit.hit_point.x - 4.5).abs() < 0.1);
    }

    #[test]
    fn projectile_misses_entity() {
        let entities = vec![
            (Vec2::new(5.0, 5.0), 0.5, 0.0, 2.0), // entity far away
        ];
        let result = check_projectile_entity_collision(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(10.0, 0.0, 1.0),
            &entities,
        );
        assert!(result.is_none());
    }

    #[test]
    fn range_limit_exceeded() {
        assert!(check_range_limit(100.0, 50.0));
        assert!(!check_range_limit(30.0, 50.0));
        assert!(!check_range_limit(30.0, 0.0)); // 0 = unlimited range
    }

    #[test]
    fn wall_reflection_reverses_approach() {
        // Wall along Y axis at x=5 (vertical wall)
        let wall_a = Vec2::new(5.0, 0.0);
        let wall_b = Vec2::new(5.0, 10.0);
        // Projectile moving east (positive X) should bounce back west
        let vel = Vec3::new(1.0, 0.0, 0.5);
        let reflected = reflect_velocity_wall(vel, wall_a, wall_b);
        assert!((reflected.x - (-1.0)).abs() < 0.01, "X should be negated");
        assert!((reflected.y - 0.0).abs() < 0.01, "Y should stay 0");
        assert!((reflected.z - 0.5).abs() < 0.01, "Z should be preserved");
    }

    #[test]
    fn wall_reflection_angled() {
        // 45-degree wall
        let wall_a = Vec2::new(0.0, 0.0);
        let wall_b = Vec2::new(1.0, 1.0);
        // Moving east (1,0) should reflect to moving north (0,1)
        let vel = Vec3::new(1.0, 0.0, 0.0);
        let reflected = reflect_velocity_wall(vel, wall_a, wall_b);
        assert!((reflected.x - 0.0).abs() < 0.01);
        assert!((reflected.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn floor_reflection_reverses_z() {
        let vel = Vec3::new(1.0, 2.0, -3.0);
        let reflected = reflect_velocity_floor(vel, 0.0);
        assert_eq!(reflected.x, 1.0);
        assert_eq!(reflected.y, 2.0);
        assert!((reflected.z - 3.0).abs() < 0.01, "Z should be negated");
    }

    #[test]
    fn floor_reflection_energy_loss() {
        let vel = Vec3::new(1.0, 0.0, -4.0);
        let reflected = reflect_velocity_floor(vel, 0.5);
        assert_eq!(reflected.x, 1.0);
        assert!((reflected.z - 2.0).abs() < 0.01, "Z should be halved after 50% energy loss");
    }

    fn make_projectile_def(
        detonation_effect: i16,
        area_of_effect: i16,
        damage_base: i16,
        media_detonation_effect: i16,
    ) -> marathon_formats::physics::ProjectileDefinition {
        marathon_formats::physics::ProjectileDefinition {
            collection: 0,
            shape: 0,
            detonation_effect,
            media_detonation_effect,
            contrail_effect: -1,
            ticks_between_contrails: 0,
            maximum_contrails: 0,
            media_projectile_promotion: -1,
            radius: 0,
            area_of_effect,
            damage: marathon_formats::DamageDefinition {
                damage_type: 0,
                flags: 0,
                base: damage_base,
                random: 0,
                scale: 1.0,
            },
            flags: 0,
            speed: 512,
            maximum_range: 0,
            sound_pitch: 1.0,
            flyby_sound: -1,
            rebound_sound: -1,
        }
    }

    #[test]
    fn detonation_no_aoe_despawns_and_spawns_effect() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let def = make_projectile_def(5, 0, 20, -1);
        let hit_point = Vec3::new(1.0, 2.0, 3.0);

        let result = compute_detonation(hit_point, &def, None, &[], false, &mut rng);
        assert_eq!(result.effect_to_spawn, Some((hit_point, 5)));
        assert!(result.aoe_damages.is_empty());
        assert!(result.direct_hit_damage.is_none());
    }

    #[test]
    fn detonation_no_effect_when_minus_one() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let def = make_projectile_def(-1, 0, 20, -1);

        let result = compute_detonation(Vec3::ZERO, &def, None, &[], false, &mut rng);
        assert!(result.effect_to_spawn.is_none());
    }

    #[test]
    fn detonation_with_aoe_applies_scaled_damage() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // area_of_effect = 2048 = 2.0 WU radius
        let def = make_projectile_def(5, 2048, 100, -1);
        let hit_point = Vec3::ZERO;

        let entities = vec![
            (Vec3::new(0.0, 0.0, 0.0), 0u32, 0u32), // at center
            (Vec3::new(1.0, 0.0, 0.0), 0u32, 0u32), // at half radius
            (Vec3::new(3.0, 0.0, 0.0), 0u32, 0u32), // beyond radius
        ];

        let result = compute_detonation(hit_point, &def, None, &entities, false, &mut rng);
        assert_eq!(result.aoe_damages.len(), 2); // center + half radius
        // Center entity gets full damage
        assert_eq!(result.aoe_damages[0].0, 0);
        assert_eq!(result.aoe_damages[0].1, 100);
        // Half radius entity gets ~50% damage
        assert_eq!(result.aoe_damages[1].0, 1);
        assert_eq!(result.aoe_damages[1].1, 50);
    }

    #[test]
    fn detonation_media_uses_media_effect() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let def = make_projectile_def(5, 0, 20, 10);

        let result = compute_detonation(Vec3::ZERO, &def, None, &[], true, &mut rng);
        assert_eq!(result.effect_to_spawn, Some((Vec3::ZERO, 10)));
    }
}
