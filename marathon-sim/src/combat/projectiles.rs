use glam::Vec3;

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
}
