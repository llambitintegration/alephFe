use glam::{Vec2, Vec3};

use crate::collision::{find_polygon_for_point, segment_intersection, slide_along_wall, wall_normal};
use crate::tick::ActionFlags;
use crate::world::MapGeometry;

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

    // Iterate up to 3 times for multi-wall slides
    for _ in 0..3 {
        let mut blocked = false;

        for &(line_idx, adj) in &geometry.polygon_adjacency[poly] {
            let (la, lb) = geometry.line_endpoints[line_idx];

            // Check if movement crosses this line
            if let Some(_hit) = segment_intersection(old_2d, pos_2d, la, lb) {
                let can_pass = if let Some(adj_idx) = adj {
                    // Check step delta and ceiling clearance
                    let adj_floor = geometry.floor_heights[adj_idx];
                    let adj_ceiling = geometry.ceiling_heights[adj_idx];
                    let cur_floor = geometry.floor_heights[poly];
                    let floor_diff = adj_floor - cur_floor;
                    let player_z = z.max(cur_floor);
                    let clearance = adj_ceiling - adj_floor;

                    floor_diff <= params.step_delta && clearance >= params.height
                        && (adj_ceiling - player_z.max(adj_floor)) >= params.height
                } else {
                    false
                };

                if can_pass {
                    let adj_idx = adj.unwrap();
                    let adj_floor = geometry.floor_heights[adj_idx];
                    let cur_floor = geometry.floor_heights[poly];

                    // Step up if needed
                    if adj_floor > cur_floor {
                        z = z.max(adj_floor);
                    }
                    poly = adj_idx;
                } else {
                    // Slide along wall
                    let normal = wall_normal(la, lb);
                    let movement = pos_2d - old_2d;
                    let slid = slide_along_wall(movement, normal);
                    pos_2d = old_2d + slid;

                    // Also project velocity
                    let vel_2d = Vec2::new(vel.x, vel.y);
                    let slid_vel = slide_along_wall(vel_2d, normal);
                    vel = Vec3::new(slid_vel.x, slid_vel.y, vel.z);

                    blocked = true;
                    break;
                }
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
                    (1, None),  // bottom wall
                    (0, Some(1)), // shared line -> poly 1
                    (2, None),  // top wall
                    (3, None),  // left wall
                ],
                vec![
                    (4, None),  // bottom wall
                    (5, None),  // right wall
                    (6, None),  // top wall
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
        // Try to walk into the left wall (solid, line index 3)
        let result = apply_player_collision(
            Vec3::new(0.2, 0.5, 0.0),
            Vec3::new(-0.2, 0.6, 0.0),
            Vec3::new(-0.4, 0.1, 0.0),
            0,
            &params,
            &geometry,
        );
        // Should be blocked from going through the wall
        assert!(result.position.x >= 0.0);
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
}
