use glam::{Vec2, Vec3};

use crate::collision::segment_intersection;

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
}
