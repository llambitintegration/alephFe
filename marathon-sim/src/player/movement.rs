use glam::{Vec2, Vec3};

use crate::tick::ActionFlags;

/// Player movement parameters extracted from PhysicsConstants.
#[derive(Debug, Clone)]
pub struct PlayerPhysicsParams {
    pub max_forward_velocity: f32,
    pub max_backward_velocity: f32,
    pub max_perpendicular_velocity: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub airborne_deceleration: f32,
    pub gravitational_acceleration: f32,
    pub terminal_velocity: f32,
    pub angular_acceleration: f32,
    pub angular_deceleration: f32,
    pub max_angular_velocity: f32,
    pub maximum_elevation: f32,
    pub step_delta: f32,
    pub height: f32,
    pub radius: f32,
}

impl PlayerPhysicsParams {
    pub fn from_physics_constants(pc: &marathon_formats::PhysicsConstants) -> Self {
        Self {
            max_forward_velocity: pc.maximum_forward_velocity,
            max_backward_velocity: pc.maximum_backward_velocity,
            max_perpendicular_velocity: pc.maximum_perpendicular_velocity,
            acceleration: pc.acceleration,
            deceleration: pc.deceleration,
            airborne_deceleration: pc.airborne_deceleration,
            gravitational_acceleration: pc.gravitational_acceleration,
            terminal_velocity: pc.terminal_velocity,
            angular_acceleration: pc.angular_acceleration,
            angular_deceleration: pc.angular_deceleration,
            max_angular_velocity: pc.maximum_angular_velocity,
            maximum_elevation: pc.maximum_elevation,
            step_delta: pc.step_delta,
            height: pc.height,
            radius: pc.radius,
        }
    }
}

/// Compute player velocity change for one tick based on action flags.
///
/// Returns the new velocity (XY movement + Z gravity).
pub fn compute_player_velocity(
    current_velocity: Vec3,
    facing: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
    grounded: bool,
) -> Vec3 {
    let forward_dir = Vec2::new(facing.cos(), facing.sin());
    let right_dir = Vec2::new(-facing.sin(), facing.cos());

    let mut accel = Vec2::ZERO;

    // Forward/backward
    if action_flags.contains(ActionFlags::MOVE_FORWARD) {
        accel += forward_dir * params.acceleration;
    }
    if action_flags.contains(ActionFlags::MOVE_BACKWARD) {
        accel -= forward_dir * params.acceleration;
    }

    // Strafe
    if action_flags.contains(ActionFlags::STRAFE_RIGHT) {
        accel += right_dir * params.acceleration;
    }
    if action_flags.contains(ActionFlags::STRAFE_LEFT) {
        accel -= right_dir * params.acceleration;
    }

    let mut vel_xy = Vec2::new(current_velocity.x, current_velocity.y);

    if accel.length_squared() > 0.0 {
        vel_xy += accel;
    } else {
        // Decelerate when no input
        let decel = if grounded {
            params.deceleration
        } else {
            params.airborne_deceleration
        };
        let speed = vel_xy.length();
        if speed > decel {
            vel_xy = vel_xy.normalize() * (speed - decel);
        } else {
            vel_xy = Vec2::ZERO;
        }
    }

    // Clamp XY speed
    let forward_component = vel_xy.dot(forward_dir);
    let perpendicular_component = vel_xy.dot(right_dir);

    let clamped_forward = if forward_component > 0.0 {
        forward_component.min(params.max_forward_velocity)
    } else {
        forward_component.max(-params.max_backward_velocity)
    };
    let clamped_perp =
        perpendicular_component.clamp(-params.max_perpendicular_velocity, params.max_perpendicular_velocity);

    vel_xy = forward_dir * clamped_forward + right_dir * clamped_perp;

    // Gravity (Z axis)
    let mut vel_z = current_velocity.z;
    if !grounded {
        vel_z -= params.gravitational_acceleration;
        vel_z = vel_z.max(-params.terminal_velocity);
    }

    Vec3::new(vel_xy.x, vel_xy.y, vel_z)
}

/// Compute facing angle change for one tick based on turn input.
pub fn compute_facing(
    current_facing: f32,
    current_angular_velocity: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
) -> (f32, f32) {
    let mut angular_vel = current_angular_velocity;

    if action_flags.contains(ActionFlags::TURN_LEFT) {
        angular_vel += params.angular_acceleration;
    } else if action_flags.contains(ActionFlags::TURN_RIGHT) {
        angular_vel -= params.angular_acceleration;
    } else {
        // Decelerate angular velocity
        if angular_vel.abs() > params.angular_deceleration {
            angular_vel -= angular_vel.signum() * params.angular_deceleration;
        } else {
            angular_vel = 0.0;
        }
    }

    angular_vel = angular_vel.clamp(-params.max_angular_velocity, params.max_angular_velocity);

    let new_facing = (current_facing + angular_vel) % std::f32::consts::TAU;

    (new_facing, angular_vel)
}

/// Compute vertical look angle change.
pub fn compute_vertical_look(
    current_look: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
) -> f32 {
    let mut look = current_look;
    let look_speed = params.angular_acceleration; // Use same rate for simplicity

    if action_flags.contains(ActionFlags::LOOK_UP) {
        look += look_speed;
    }
    if action_flags.contains(ActionFlags::LOOK_DOWN) {
        look -= look_speed;
    }

    look.clamp(-params.maximum_elevation, params.maximum_elevation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> PlayerPhysicsParams {
        PlayerPhysicsParams {
            max_forward_velocity: 0.1,
            max_backward_velocity: 0.05,
            max_perpendicular_velocity: 0.08,
            acceleration: 0.01,
            deceleration: 0.005,
            airborne_deceleration: 0.002,
            gravitational_acceleration: 0.005,
            terminal_velocity: 0.5,
            angular_acceleration: 0.05,
            angular_deceleration: 0.03,
            max_angular_velocity: 0.2,
            maximum_elevation: 0.5,
            step_delta: 0.25,
            height: 0.8,
            radius: 0.25,
        }
    }

    #[test]
    fn forward_movement_accelerates() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD);
        let vel = compute_player_velocity(Vec3::ZERO, 0.0, &flags, &params, true);
        assert!(vel.x > 0.0); // facing 0 = east, so forward is +X
    }

    #[test]
    fn no_input_decelerates() {
        let params = test_params();
        let flags = ActionFlags::default();
        let initial_vel = Vec3::new(0.05, 0.0, 0.0);
        let vel = compute_player_velocity(initial_vel, 0.0, &flags, &params, true);
        assert!(vel.x < initial_vel.x);
    }

    #[test]
    fn gravity_when_airborne() {
        let params = test_params();
        let flags = ActionFlags::default();
        let vel = compute_player_velocity(Vec3::ZERO, 0.0, &flags, &params, false);
        assert!(vel.z < 0.0); // falling
    }

    #[test]
    fn no_gravity_when_grounded() {
        let params = test_params();
        let flags = ActionFlags::default();
        let vel = compute_player_velocity(Vec3::new(0.0, 0.0, 0.0), 0.0, &flags, &params, true);
        assert_eq!(vel.z, 0.0);
    }

    #[test]
    fn turn_changes_facing() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::TURN_LEFT);
        let (new_facing, _) = compute_facing(0.0, 0.0, &flags, &params);
        assert!(new_facing > 0.0);
    }

    #[test]
    fn vertical_look_clamped() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::LOOK_UP);
        let mut look = 0.0;
        for _ in 0..100 {
            look = compute_vertical_look(look, &flags, &params);
        }
        assert!(look <= params.maximum_elevation + f32::EPSILON);
    }
}
