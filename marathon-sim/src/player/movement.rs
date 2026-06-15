use glam::{Vec2, Vec3};

use crate::collision::{
    find_polygon_for_point, point_to_segment_distance, segment_intersection, slide_along_wall,
    wall_normal,
};
use crate::tick::ActionFlags;
use crate::world::MapGeometry;

/// Player movement parameters extracted from PhysicsConstants.
#[derive(Debug, Clone, bevy_ecs::prelude::Resource)]
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

/// Conversion factor from Marathon angle units (512 = full circle) to radians.
pub const MARATHON_ANGLE_TO_RAD: f32 = std::f32::consts::TAU / 512.0;

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
            // Angular fields in PhysicsConstants are Marathon angle units
            // (512 = full circle). Convert to radians so the sim can use them
            // directly with glam trig functions.
            angular_acceleration: pc.angular_acceleration * MARATHON_ANGLE_TO_RAD,
            angular_deceleration: pc.angular_deceleration * MARATHON_ANGLE_TO_RAD,
            max_angular_velocity: pc.maximum_angular_velocity * MARATHON_ANGLE_TO_RAD,
            maximum_elevation: pc.maximum_elevation * MARATHON_ANGLE_TO_RAD,
            step_delta: pc.step_delta,
            height: pc.height,
            radius: pc.radius,
        }
    }
}

/// Compute player velocity change for one tick using Marathon's axis-decomposed
/// physics model.
///
/// Velocity is tracked in **player-local coordinates**:
///   - `current_velocity.x` = forward velocity (positive = forward)
///   - `current_velocity.y` = perpendicular velocity (positive = strafing right)
///   - `current_velocity.z` = vertical velocity (positive = up)
///
/// The two horizontal axes decelerate independently (stopping strafe does not
/// affect forward speed). When input opposes current velocity on an axis,
/// `acceleration + deceleration` are applied together for snappier reversals —
/// matching Aleph One's `physics.cpp` behavior.
///
/// `facing` is unused by this function (physics operates in player-local frame)
/// but is kept in the signature for API compatibility with collision code that
/// may need to project into world space.
pub fn compute_player_velocity(
    current_velocity: Vec3,
    _facing: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
    grounded: bool,
) -> Vec3 {
    let decel_rate = if grounded {
        params.deceleration
    } else {
        params.airborne_deceleration
    };

    // ── Forward / backward axis ──────────────────────────────────────────
    let mut forward_vel = current_velocity.x;
    let move_fwd = action_flags.contains(ActionFlags::MOVE_FORWARD);
    let move_back = action_flags.contains(ActionFlags::MOVE_BACKWARD);

    if move_fwd && !move_back {
        // Positive input. If currently moving backward, apply both accel and
        // decel for a snappier reversal.
        let delta = if forward_vel < 0.0 {
            params.acceleration + decel_rate
        } else {
            params.acceleration
        };
        forward_vel += delta;
    } else if move_back && !move_fwd {
        let delta = if forward_vel > 0.0 {
            params.acceleration + decel_rate
        } else {
            params.acceleration
        };
        forward_vel -= delta;
    } else {
        // No forward/backward input: decelerate this axis independently
        // toward zero.
        if forward_vel > decel_rate {
            forward_vel -= decel_rate;
        } else if forward_vel < -decel_rate {
            forward_vel += decel_rate;
        } else {
            forward_vel = 0.0;
        }
    }

    // Clamp forward velocity to asymmetric limits.
    if forward_vel > params.max_forward_velocity {
        forward_vel = params.max_forward_velocity;
    } else if forward_vel < -params.max_backward_velocity {
        forward_vel = -params.max_backward_velocity;
    }

    // ── Perpendicular (strafe) axis ──────────────────────────────────────
    let mut perp_vel = current_velocity.y;
    let strafe_right = action_flags.contains(ActionFlags::STRAFE_RIGHT);
    let strafe_left = action_flags.contains(ActionFlags::STRAFE_LEFT);

    if strafe_right && !strafe_left {
        let delta = if perp_vel < 0.0 {
            params.acceleration + decel_rate
        } else {
            params.acceleration
        };
        perp_vel += delta;
    } else if strafe_left && !strafe_right {
        let delta = if perp_vel > 0.0 {
            params.acceleration + decel_rate
        } else {
            params.acceleration
        };
        perp_vel -= delta;
    } else {
        // No strafe input: decelerate independently from forward axis.
        if perp_vel > decel_rate {
            perp_vel -= decel_rate;
        } else if perp_vel < -decel_rate {
            perp_vel += decel_rate;
        } else {
            perp_vel = 0.0;
        }
    }

    // Clamp perpendicular velocity (symmetric).
    perp_vel = perp_vel.clamp(
        -params.max_perpendicular_velocity,
        params.max_perpendicular_velocity,
    );

    // ── Vertical axis (gravity) ──────────────────────────────────────────
    let mut vel_z = current_velocity.z;
    if !grounded {
        vel_z -= params.gravitational_acceleration;
        vel_z = vel_z.max(-params.terminal_velocity);
    }

    Vec3::new(forward_vel, perp_vel, vel_z)
}

/// Project player-local velocity `(forward, perp, vert)` into world-space
/// `(dx, dy, dz)` using the current facing angle.
///
/// Matches the convention used by `compute_player_velocity`'s caller:
///   `forward_dir = (cos(facing), sin(facing))`
///   `right_dir   = (-sin(facing), cos(facing))`
pub fn velocity_local_to_world(local: Vec3, facing: f32) -> Vec3 {
    let (sin_f, cos_f) = facing.sin_cos();
    let world_x = local.x * cos_f - local.y * sin_f;
    let world_y = local.x * sin_f + local.y * cos_f;
    Vec3::new(world_x, world_y, local.z)
}

/// Inverse of `velocity_local_to_world`: convert a world-space velocity to
/// player-local `(forward, perp, vert)` given the current facing.
pub fn velocity_world_to_local(world: Vec3, facing: f32) -> Vec3 {
    let (sin_f, cos_f) = facing.sin_cos();
    let forward = world.x * cos_f + world.y * sin_f;
    let perp = -world.x * sin_f + world.y * cos_f;
    Vec3::new(forward, perp, world.z)
}

/// Compute facing angle change for one tick based on turn input and mouse yaw.
///
/// When `mouse_yaw` is non-zero, it is added directly to facing (1:1 feel).
/// Keyboard turning via ActionFlags still uses the angular velocity system.
/// Both compose additively.
pub fn compute_facing(
    current_facing: f32,
    current_angular_velocity: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
    mouse_yaw: f32,
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

    // Apply both angular velocity from keyboard and direct mouse yaw
    let new_facing = (current_facing + angular_vel + mouse_yaw) % std::f32::consts::TAU;

    (new_facing, angular_vel)
}

/// Compute vertical look angle change from keyboard and mouse pitch.
///
/// When `mouse_pitch` is non-zero, it is added directly to the look angle.
/// Keyboard look via ActionFlags uses a fixed rate. Both compose additively.
/// Result is clamped to elevation limits.
pub fn compute_vertical_look(
    current_look: f32,
    action_flags: &ActionFlags,
    params: &PlayerPhysicsParams,
    mouse_pitch: f32,
) -> f32 {
    let mut look = current_look;
    let look_speed = params.angular_acceleration; // Use same rate for simplicity

    if action_flags.contains(ActionFlags::LOOK_UP) {
        look += look_speed;
    }
    if action_flags.contains(ActionFlags::LOOK_DOWN) {
        look -= look_speed;
    }

    // Apply direct mouse pitch
    look += mouse_pitch;

    look.clamp(-params.maximum_elevation, params.maximum_elevation)
}

/// Result of applying collision response to player movement.
#[derive(Debug, Clone)]
pub struct CollisionResult {
    /// Final position after collision.
    pub position: Vec3,
    /// Final velocity after collision (may be zeroed or projected).
    pub velocity: Vec3,
    /// New polygon index.
    pub polygon_index: usize,
    /// Whether the player is grounded.
    pub grounded: bool,
}

/// Apply wall collision, step climbing, and ceiling checks to player movement.
///
/// Given an attempted new position (old_pos + velocity), this function:
/// 1. Checks each line in the current polygon for crossings
/// 2. If a line is solid (or too tall to step/too low ceiling), slides along it
/// 3. If passable with valid step/ceiling, allows crossing and updates polygon
/// 4. Handles gravity grounding (Z clamped to floor)
pub fn apply_player_collision(
    old_pos: Vec3,
    new_pos: Vec3,
    velocity: Vec3,
    current_polygon: usize,
    params: &PlayerPhysicsParams,
    geometry: &MapGeometry,
) -> CollisionResult {
    let old_2d = Vec2::new(old_pos.x, old_pos.y);
    let mut pos_2d = Vec2::new(new_pos.x, new_pos.y);
    let mut vel = velocity;
    let mut poly = current_polygon;
    let mut z = new_pos.z;

    let radius = params.radius;

    // Determine if the player was grounded before this movement
    let was_grounded = old_pos.z <= geometry.floor_heights[current_polygon] + f32::EPSILON;

    // Helper closure: check if a line can be passed through
    let can_pass_line = |adj: Option<usize>,
                         z: f32,
                         poly: usize,
                         params: &PlayerPhysicsParams,
                         geometry: &MapGeometry|
     -> bool {
        if let Some(adj_idx) = adj {
            let adj_floor = geometry.floor_heights[adj_idx];
            let adj_ceiling = geometry.ceiling_heights[adj_idx];
            let cur_floor = geometry.floor_heights[poly];
            let floor_diff = adj_floor - cur_floor;
            let player_z = z.max(cur_floor);
            let clearance = adj_ceiling - adj_floor;

            floor_diff <= params.step_delta
                && clearance >= params.height
                && (adj_ceiling - player_z.max(adj_floor)) >= params.height
        } else {
            false
        }
    };

    // Iterate up to 3 times for multi-wall slides
    for _ in 0..3 {
        let mut blocked = false;

        for &(line_idx, adj) in &geometry.polygon_adjacency[poly] {
            let (la, lb) = geometry.line_endpoints[line_idx];

            // Check if movement crosses this line
            if let Some(_hit) = segment_intersection(old_2d, pos_2d, la, lb) {
                if can_pass_line(adj, z, poly, params, geometry) {
                    let adj_idx = adj.unwrap();
                    let adj_floor = geometry.floor_heights[adj_idx];

                    if was_grounded {
                        // Grounded players snap to the adjacent floor (up or down)
                        z = adj_floor;
                    } else if adj_floor > geometry.floor_heights[poly] {
                        // Airborne players only get pushed up by step-ups
                        z = z.max(adj_floor);
                    }
                    poly = adj_idx;
                    continue;
                } else {
                    // Crossed a wall we cannot pass -- slide movement along the wall,
                    // then push the position to be at least `radius` from the segment.
                    let normal = wall_normal(la, lb);
                    let movement = pos_2d - old_2d;
                    let slid = slide_along_wall(movement, normal);
                    pos_2d = old_2d + slid;

                    // Project velocity along the wall
                    let vel_2d = Vec2::new(vel.x, vel.y);
                    let slid_vel = slide_along_wall(vel_2d, normal);
                    vel = Vec3::new(slid_vel.x, slid_vel.y, vel.z);

                    // After sliding, ensure we maintain radius distance from the wall
                    let (dist, closest) = point_to_segment_distance(pos_2d, la, lb);
                    if dist < radius {
                        let push_dir = if dist > 1e-6 {
                            (pos_2d - closest).normalize()
                        } else {
                            normal
                        };
                        pos_2d += push_dir * (radius - dist);
                    }

                    blocked = true;
                    break;
                }
            }

            // Radius-based collision: push player out of impassable walls when the
            // player's circular body overlaps the wall segment, even without crossing.
            let (dist, closest) = point_to_segment_distance(pos_2d, la, lb);
            if dist < radius && !can_pass_line(adj, z, poly, params, geometry) {
                // Push the player center outward so it is exactly radius away
                let push_dir = if dist > 1e-6 {
                    (pos_2d - closest).normalize()
                } else {
                    // Player center is exactly on the wall; use wall normal as fallback
                    wall_normal(la, lb)
                };
                let penetration = radius - dist;
                pos_2d += push_dir * penetration;

                // Project velocity along the wall
                let normal = wall_normal(la, lb);
                let vel_2d = Vec2::new(vel.x, vel.y);
                let slid_vel = slide_along_wall(vel_2d, normal);
                vel = Vec3::new(slid_vel.x, slid_vel.y, vel.z);

                blocked = true;
                break;
            }
        }

        if !blocked {
            break;
        }
    }

    // Update polygon index based on final position
    poly = find_polygon_for_point(
        pos_2d,
        poly,
        &geometry.polygon_vertices,
        &geometry.polygon_adjacency,
    );

    // Ceiling collision: prevent head from going through ceiling
    let ceiling = geometry.ceiling_heights[poly];
    let max_z = ceiling - params.height;
    if z > max_z {
        z = max_z;
        if vel.z > 0.0 {
            vel.z = 0.0;
        }
    }

    // Ground the player
    let floor = geometry.floor_heights[poly];
    let grounded = z <= floor + f32::EPSILON;
    if grounded {
        z = floor;
        vel.z = 0.0;
    }

    CollisionResult {
        position: Vec3::new(pos_2d.x, pos_2d.y, z),
        velocity: vel,
        polygon_index: poly,
        grounded,
    }
}

/// Apply media submersion effects to the player.
///
/// Returns (new_velocity, oxygen_change, drowning_damage).
/// - velocity is reduced by drag when submerged
/// - oxygen decreases when submerged, increases when above surface
/// - drowning damage applied when oxygen <= 0
pub fn apply_media_effects(
    velocity: Vec3,
    player_z: f32,
    media_height: Option<f32>,
    media_type: Option<i16>,
    current_oxygen: i16,
    max_oxygen: i16,
) -> (Vec3, i16, i16) {
    let Some(surface_height) = media_height else {
        // No media — recharge oxygen
        let oxygen_change = if current_oxygen < max_oxygen { 1 } else { 0 };
        return (velocity, oxygen_change, 0);
    };

    if player_z >= surface_height {
        // Above surface — recharge oxygen
        let oxygen_change = if current_oxygen < max_oxygen { 1 } else { 0 };
        return (velocity, oxygen_change, 0);
    }

    // Submerged
    let drag = crate::world_mechanics::media::media_drag_factor(media_type.unwrap_or(0));
    let new_vel = Vec3::new(velocity.x * drag, velocity.y * drag, velocity.z);

    let oxygen_change: i16 = -2; // deplete 2 per tick when submerged
    let drowning_damage = if current_oxygen <= 0 { 5 } else { 0 };

    (new_vel, oxygen_change, drowning_damage)
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
        // Velocity is now player-local: x = forward velocity.
        let vel = compute_player_velocity(Vec3::ZERO, 0.0, &flags, &params, true);
        assert!(vel.x > 0.0);
        assert_eq!(vel.y, 0.0);
    }

    #[test]
    fn no_input_decelerates() {
        let params = test_params();
        let flags = ActionFlags::default();
        // Player-local frame: forward vel = 0.05
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
    fn forward_and_strafe_decelerate_independently() {
        let params = test_params();
        // Start with both forward and perpendicular velocity
        let initial = Vec3::new(0.05, 0.03, 0.0);
        // Only forward input — perpendicular should decelerate while forward stays
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD);
        let vel = compute_player_velocity(initial, 0.0, &flags, &params, true);
        // Forward should accelerate (or stay), perpendicular should decelerate
        assert!(
            vel.x >= initial.x,
            "forward should not decelerate when forward input held"
        );
        assert!(
            vel.y.abs() < initial.y.abs(),
            "perp should decelerate when no strafe input"
        );
    }

    #[test]
    fn direction_reversal_boost_applies_accel_plus_decel() {
        let params = test_params();
        // Player moving backward, then presses forward
        let initial = Vec3::new(-0.03, 0.0, 0.0);
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD);
        let vel = compute_player_velocity(initial, 0.0, &flags, &params, true);
        // Expected change: +(accel + decel) = +(0.01 + 0.005) = +0.015
        let expected = -0.03 + params.acceleration + params.deceleration;
        assert!((vel.x - expected).abs() < 1e-5);
    }

    #[test]
    fn strafe_does_not_affect_forward_velocity() {
        let params = test_params();
        let initial = Vec3::new(0.08, 0.0, 0.0); // moving forward at 0.08
        let flags = ActionFlags::new(ActionFlags::STRAFE_RIGHT);
        let vel = compute_player_velocity(initial, 0.0, &flags, &params, true);
        // Forward velocity should decelerate by decel_rate (no forward input)
        assert!((vel.x - (0.08 - params.deceleration)).abs() < 1e-5);
        // But perpendicular should have accelerated
        assert!(vel.y > 0.0);
    }

    #[test]
    fn forward_velocity_clamped_to_max() {
        let params = test_params();
        let initial = Vec3::new(0.5, 0.0, 0.0); // way above max
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD);
        let vel = compute_player_velocity(initial, 0.0, &flags, &params, true);
        assert!((vel.x - params.max_forward_velocity).abs() < 1e-5);
    }

    #[test]
    fn backward_velocity_clamped_to_max_backward() {
        let params = test_params();
        let initial = Vec3::new(-0.5, 0.0, 0.0);
        let flags = ActionFlags::new(ActionFlags::MOVE_BACKWARD);
        let vel = compute_player_velocity(initial, 0.0, &flags, &params, true);
        assert!((vel.x - (-params.max_backward_velocity)).abs() < 1e-5);
    }

    #[test]
    fn velocity_local_to_world_round_trip() {
        let local = Vec3::new(0.1, 0.05, 0.02);
        let facing = 1.2_f32;
        let world = velocity_local_to_world(local, facing);
        let back = velocity_world_to_local(world, facing);
        assert!((back.x - local.x).abs() < 1e-5);
        assert!((back.y - local.y).abs() < 1e-5);
        assert!((back.z - local.z).abs() < 1e-5);
    }

    #[test]
    fn velocity_local_to_world_east_facing() {
        // Facing east (0 radians): local forward = (1,0,0) should map to world (1,0,0)
        let local = Vec3::new(1.0, 0.0, 0.0);
        let world = velocity_local_to_world(local, 0.0);
        assert!((world.x - 1.0).abs() < 1e-5);
        assert!(world.y.abs() < 1e-5);
    }

    #[test]
    fn turn_changes_facing() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::TURN_LEFT);
        let (new_facing, _) = compute_facing(0.0, 0.0, &flags, &params, 0.0);
        assert!(new_facing > 0.0);
    }

    #[test]
    fn vertical_look_clamped() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::LOOK_UP);
        let mut look = 0.0;
        for _ in 0..100 {
            look = compute_vertical_look(look, &flags, &params, 0.0);
        }
        assert!(look <= params.maximum_elevation + f32::EPSILON);
    }

    #[test]
    fn angular_constants_converted_to_radians() {
        // Marathon running defaults (per Aleph One physics_models.h):
        //   angular_acceleration = 5*FIXED_ONE/4 -> 1.25 angle units
        //   maximum_angular_velocity = 10*FIXED_ONE -> 10.0 angle units
        //   maximum_elevation = QUARTER_CIRCLE*FIXED_ONE/3 -> ~42.67 angle units
        // After conversion (factor 2π/512), these become:
        //   angular_acceleration ≈ 0.01534 rad
        //   maximum_angular_velocity ≈ 0.1227 rad
        //   maximum_elevation ≈ 0.5236 rad (~30°)
        let pc = marathon_formats::PhysicsConstants {
            maximum_forward_velocity: 0.125,
            maximum_backward_velocity: 0.083,
            maximum_perpendicular_velocity: 0.077,
            acceleration: 0.01,
            deceleration: 0.02,
            airborne_deceleration: 0.00556,
            gravitational_acceleration: 0.0025,
            climbing_acceleration: 0.005,
            terminal_velocity: 0.143,
            external_deceleration: 0.01,
            angular_acceleration: 1.25,
            angular_deceleration: 2.5,
            maximum_angular_velocity: 10.0,
            angular_recentering_velocity: 0.5,
            fast_angular_velocity: 20.0,
            fast_angular_maximum: 25.0,
            maximum_elevation: 42.667,
            external_angular_deceleration: 1.0,
            step_delta: 0.05,
            step_amplitude: 0.02,
            radius: 0.25,
            height: 0.8,
            dead_height: 0.3,
            camera_height: 0.2,
            splash_height: 0.1,
            half_camera_separation: 0.0,
        };
        let params = PlayerPhysicsParams::from_physics_constants(&pc);
        // Radian conversion: angle_units * (2π / 512)
        let f = MARATHON_ANGLE_TO_RAD;
        assert!((params.angular_acceleration - 1.25 * f).abs() < 1e-5);
        assert!((params.angular_deceleration - 2.5 * f).abs() < 1e-5);
        assert!((params.max_angular_velocity - 10.0 * f).abs() < 1e-5);
        // ~30 degrees = π/6 ≈ 0.5236
        assert!((params.maximum_elevation - 42.667 * f).abs() < 1e-3);
        let thirty_deg_rad = std::f32::consts::FRAC_PI_6;
        assert!((params.maximum_elevation - thirty_deg_rad).abs() < 0.01);
        // Velocity fields should be unchanged (not angular)
        assert!((params.max_forward_velocity - 0.125).abs() < 1e-5);
    }

    #[test]
    fn mouse_yaw_changes_facing_directly() {
        let params = test_params();
        let flags = ActionFlags::default();
        let (new_facing, angular_vel) = compute_facing(0.0, 0.0, &flags, &params, 0.1);
        assert!((new_facing - 0.1).abs() < f32::EPSILON);
        assert_eq!(angular_vel, 0.0);
    }

    #[test]
    fn mouse_yaw_composes_with_keyboard_turn() {
        let params = test_params();
        let flags = ActionFlags::new(ActionFlags::TURN_RIGHT);
        let (new_facing, angular_vel) = compute_facing(0.0, 0.0, &flags, &params, 0.1);
        assert!((angular_vel - (-params.angular_acceleration)).abs() < f32::EPSILON);
        let expected = angular_vel + 0.1;
        assert!((new_facing - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn mouse_pitch_changes_vertical_look() {
        let params = test_params();
        let flags = ActionFlags::default();
        let new_look = compute_vertical_look(0.0, &flags, &params, -0.05);
        assert!((new_look - (-0.05)).abs() < f32::EPSILON);
    }

    #[test]
    fn mouse_pitch_clamped_to_limits() {
        let params = test_params();
        let flags = ActionFlags::default();
        let new_look = compute_vertical_look(0.0, &flags, &params, 10.0);
        assert!((new_look - params.maximum_elevation).abs() < f32::EPSILON);
        let new_look = compute_vertical_look(0.0, &flags, &params, -10.0);
        assert!((new_look - (-params.maximum_elevation)).abs() < f32::EPSILON);
    }

    fn two_polygon_geometry() -> MapGeometry {
        // Two adjacent 1x1 squares side by side: poly 0 (0,0)-(1,1), poly 1 (1,0)-(2,1)
        // Line 0 is the shared line between them at x=1
        // Line 1-4 are outer walls of poly 0
        // Line 5-7 are outer walls of poly 1
        MapGeometry {
            polygon_vertices: vec![
                vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
                vec![
                    Vec2::new(1.0, 0.0),
                    Vec2::new(2.0, 0.0),
                    Vec2::new(2.0, 1.0),
                    Vec2::new(1.0, 1.0),
                ],
            ],
            floor_heights: vec![0.0, 0.0],
            ceiling_heights: vec![2.0, 2.0],
            polygon_adjacency: vec![
                vec![
                    (1, None),    // bottom wall
                    (0, Some(1)), // shared line -> poly 1
                    (2, None),    // top wall
                    (3, None),    // left wall
                ],
                vec![
                    (4, None),    // bottom wall
                    (5, None),    // right wall
                    (6, None),    // top wall
                    (0, Some(0)), // shared line -> poly 0
                ],
            ],
            line_endpoints: vec![
                (Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)), // shared
                (Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)), // bottom 0
                (Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0)), // top 0
                (Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0)), // left 0
                (Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)), // bottom 1
                (Vec2::new(2.0, 0.0), Vec2::new(2.0, 1.0)), // right 1
                (Vec2::new(1.0, 1.0), Vec2::new(2.0, 1.0)), // top 1
            ],
            line_solid: vec![false, true, true, true, true, true, true],
            line_transparent: vec![true, false, false, false, false, false, false],
            polygon_media_index: vec![-1, -1],
            polygon_floor_light_index: vec![-1, -1],
            polygon_ceiling_light_index: vec![-1, -1],
            polygon_types: vec![0, 0],
            polygon_permutations: vec![-1, -1],
            line_side_indices: vec![
                (None, None),
                (None, None),
                (None, None),
                (None, None),
                (None, None),
                (None, None),
                (None, None),
            ],
        }
    }

    #[test]
    fn collision_passes_through_adjacent_polygon() {
        let params = test_params();
        let geometry = two_polygon_geometry();
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 0.0),
            Vec3::new(1.2, 0.5, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 1);
        assert!(result.position.x > 1.0);
    }

    #[test]
    fn collision_slides_along_solid_wall() {
        let params = test_params();
        let geometry = two_polygon_geometry();
        // Try to walk into the left wall (solid, line index 3 at x=0)
        // Start well inside, attempt to move past wall
        let result = apply_player_collision(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(-0.2, 0.6, 0.0),
            Vec3::new(-0.7, 0.1, 0.0),
            0,
            &params,
            &geometry,
        );
        // Should be pushed back to at least radius distance from the wall
        assert!(result.position.x >= params.radius - 0.01);
        assert_eq!(result.polygon_index, 0);
    }

    #[test]
    fn step_climbing_small_ledge() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.floor_heights[1] = 0.2; // Small step up (within step_delta=0.25)
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 0.0),
            Vec3::new(1.2, 0.5, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 1);
        assert!(result.position.z >= 0.2 - f32::EPSILON);
    }

    #[test]
    fn blocked_by_tall_ledge() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.floor_heights[1] = 0.5; // Too tall for step_delta=0.25
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 0.0),
            Vec3::new(1.2, 0.5, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 0);
    }

    #[test]
    fn blocked_by_low_ceiling() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.ceiling_heights[1] = 0.5; // Too low for player height=0.8
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 0.0),
            Vec3::new(1.2, 0.5, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 0);
    }

    #[test]
    fn media_submersion_applies_drag() {
        let vel = Vec3::new(1.0, 1.0, 0.0);
        let (new_vel, oxy_change, dmg) =
            apply_media_effects(vel, 0.0, Some(1.0), Some(0), 600, 600);
        assert!(new_vel.x < 1.0);
        assert!(new_vel.y < 1.0);
        assert!(oxy_change < 0);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn media_drowning_damage_at_zero_oxygen() {
        let vel = Vec3::new(0.0, 0.0, 0.0);
        let (_, _, dmg) = apply_media_effects(vel, 0.0, Some(1.0), Some(0), 0, 600);
        assert!(dmg > 0);
    }

    #[test]
    fn above_media_recharges_oxygen() {
        let vel = Vec3::new(1.0, 0.0, 0.0);
        let (new_vel, oxy_change, dmg) =
            apply_media_effects(vel, 2.0, Some(1.0), Some(0), 500, 600);
        assert_eq!(new_vel, vel); // no drag
        assert!(oxy_change > 0);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn radius_collision_prevents_wall_overlap() {
        let params = test_params();
        let geometry = two_polygon_geometry();
        // Player moves parallel to the left wall (x=0) but ends up within radius
        let result = apply_player_collision(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.1, 0.5, 0.0),
            Vec3::new(-0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert!(
            result.position.x >= params.radius - 0.01,
            "Player at x={} should be >= radius {} from wall at x=0",
            result.position.x,
            params.radius,
        );
        assert_eq!(result.polygon_index, 0);
    }

    #[test]
    fn radius_collision_no_effect_when_far() {
        let params = test_params();
        let geometry = two_polygon_geometry();
        let result = apply_player_collision(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::ZERO,
            0,
            &params,
            &geometry,
        );
        assert!((result.position.x - 0.5).abs() < 0.01);
        assert!((result.position.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn grounded_player_snaps_down_to_lower_floor() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.floor_heights[0] = 0.5;
        geometry.floor_heights[1] = 0.0;
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 0.5),
            Vec3::new(1.2, 0.5, 0.5),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 1);
        assert!(
            (result.position.z - 0.0).abs() < f32::EPSILON,
            "Expected z=0.0, got z={}",
            result.position.z
        );
    }

    #[test]
    fn airborne_player_does_not_snap_down() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.floor_heights[0] = 0.5;
        geometry.floor_heights[1] = 0.0;
        let result = apply_player_collision(
            Vec3::new(0.8, 0.5, 1.0),
            Vec3::new(1.2, 0.5, 1.0),
            Vec3::new(0.4, 0.0, 0.0),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 1);
        assert!(
            result.position.z > 0.5,
            "Airborne player should not snap down, got z={}",
            result.position.z
        );
    }

    #[test]
    fn ceiling_collision_clamps_z() {
        let params = test_params();
        let mut geometry = two_polygon_geometry();
        geometry.ceiling_heights[0] = 1.5;
        let result = apply_player_collision(
            Vec3::new(0.5, 0.5, 1.0),
            Vec3::new(0.5, 0.5, 1.0),
            Vec3::new(0.0, 0.0, 0.5),
            0,
            &params,
            &geometry,
        );
        assert_eq!(result.polygon_index, 0);
        let max_z = 1.5 - params.height;
        assert!(
            result.position.z <= max_z + f32::EPSILON,
            "Expected z <= {}, got z={}",
            max_z,
            result.position.z
        );
        assert!(
            result.velocity.z <= 0.0,
            "Expected vel.z <= 0, got vel.z={}",
            result.velocity.z
        );
    }
}
