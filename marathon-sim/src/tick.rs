use crate::player::movement::{
    apply_player_collision, compute_facing, compute_player_velocity, compute_vertical_look,
    velocity_local_to_world, velocity_world_to_local, PlayerPhysicsParams,
};
use crate::world::{MapGeometry, SimWorld, TickCounter};

/// Action flags consumed by the simulation each tick.
/// Mirrors marathon-integration's ActionFlags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionFlags {
    bits: u32,
}

impl ActionFlags {
    pub const MOVE_FORWARD: u32 = 1 << 0;
    pub const MOVE_BACKWARD: u32 = 1 << 1;
    pub const STRAFE_LEFT: u32 = 1 << 2;
    pub const STRAFE_RIGHT: u32 = 1 << 3;
    pub const TURN_LEFT: u32 = 1 << 4;
    pub const TURN_RIGHT: u32 = 1 << 5;
    pub const LOOK_UP: u32 = 1 << 6;
    pub const LOOK_DOWN: u32 = 1 << 7;
    pub const FIRE_PRIMARY: u32 = 1 << 8;
    pub const FIRE_SECONDARY: u32 = 1 << 9;
    pub const ACTION: u32 = 1 << 10;
    pub const CYCLE_WEAPON_FWD: u32 = 1 << 11;
    pub const CYCLE_WEAPON_BACK: u32 = 1 << 12;
    pub const TOGGLE_MAP: u32 = 1 << 13;
    pub const MICROPHONE: u32 = 1 << 14;

    pub fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub fn contains(&self, flag: u32) -> bool {
        self.bits & flag != 0
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

/// Per-tick input resource injected before systems run.
#[derive(Debug, Default, bevy_ecs::prelude::Resource)]
pub struct TickInput {
    pub action_flags: ActionFlags,
    /// Mouse yaw delta in radians (positive = turn left / counter-clockwise).
    pub mouse_yaw: f32,
    /// Mouse pitch delta in radians (positive = look up).
    pub mouse_pitch: f32,
}

impl From<ActionFlags> for TickInput {
    fn from(action_flags: ActionFlags) -> Self {
        TickInput {
            action_flags,
            mouse_yaw: 0.0,
            mouse_pitch: 0.0,
        }
    }
}

impl SimWorld {
    /// Advance the simulation by one tick (1/30th of a second).
    ///
    /// Systems execute in order:
    /// 1. Input processing
    /// 2. Player physics
    /// 3. Monster AI
    /// 4. Weapon/combat
    /// 5. Projectile physics
    /// 6. Damage resolution
    /// 7. World mechanics (platforms, lights, media, items)
    /// 8. Cleanup
    pub fn tick(&mut self, input: TickInput) {
        // Store input for this tick
        self.world.insert_resource(input);

        // 1. Player physics
        self.run_player_physics();

        // Advance tick counter
        self.world.resource_mut::<TickCounter>().0 += 1;
    }

    fn run_player_physics(&mut self) {
        let tick_input = self.world.resource::<TickInput>();
        let flags = tick_input.action_flags;
        let mouse_yaw = tick_input.mouse_yaw;
        let mouse_pitch = tick_input.mouse_pitch;

        let Some(params) = self.world.get_resource::<PlayerPhysicsParams>().cloned() else {
            return;
        };
        let geometry = self.world.resource::<MapGeometry>();
        // Clone what we need from geometry to avoid borrow conflicts
        let geo_clone = MapGeometry {
            polygon_vertices: geometry.polygon_vertices.clone(),
            floor_heights: geometry.floor_heights.clone(),
            ceiling_heights: geometry.ceiling_heights.clone(),
            polygon_adjacency: geometry.polygon_adjacency.clone(),
            line_endpoints: geometry.line_endpoints.clone(),
            line_solid: geometry.line_solid.clone(),
            line_transparent: geometry.line_transparent.clone(),
        };

        let mut query = self.world.query_filtered::<(
            &mut crate::Position,
            &mut crate::Velocity,
            &mut crate::Facing,
            &mut crate::VerticalLook,
            &mut crate::AngularVelocity,
            &mut crate::PolygonIndex,
            &mut crate::Grounded,
        ), bevy_ecs::prelude::With<crate::Player>>();

        for (mut pos, mut vel, mut facing, mut vlook, mut angular_vel, mut poly_idx, mut grounded) in query.iter_mut(&mut self.world) {
            // Velocity is stored in player-local frame: x=forward, y=perp, z=vert.
            // Compute the next tick's player-local velocity from input.
            let new_local_vel =
                compute_player_velocity(vel.0, facing.0, &flags, &params, grounded.0);

            // Compute facing (turning) — mouse yaw applied directly, keyboard via angular velocity
            let (new_facing, new_angular) = compute_facing(facing.0, angular_vel.0, &flags, &params, mouse_yaw);
            facing.0 = new_facing;
            angular_vel.0 = new_angular;

            // Compute vertical look — mouse pitch applied directly
            vlook.0 = compute_vertical_look(vlook.0, &flags, &params, mouse_pitch);

            // Project player-local velocity into world-space using the NEW facing.
            // This gives us the "velocity rotates with you when turning" behavior
            // that matches Marathon's physics model.
            let world_vel = velocity_local_to_world(new_local_vel, new_facing);

            // Apply collision in world-space.
            let new_pos = pos.0 + world_vel;
            let result = apply_player_collision(
                pos.0, new_pos, world_vel, poly_idx.0, &params, &geo_clone,
            );

            pos.0 = result.position;
            // Convert the post-collision world-space velocity back to
            // player-local form for storage.
            vel.0 = velocity_world_to_local(result.velocity, new_facing);
            poly_idx.0 = result.polygon_index;
            grounded.0 = result.grounded;
        }
    }

    /// Query the player's position.
    pub fn player_position(&mut self) -> Option<glam::Vec3> {
        let mut query = self.world.query_filtered::<&crate::Position, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
    }

    /// Query the player's facing angle.
    pub fn player_facing(&mut self) -> Option<f32> {
        let mut query = self.world.query_filtered::<&crate::Facing, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|f| f.0)
    }

    /// Query the player's health.
    pub fn player_health(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Health, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|h| h.0)
    }

    /// Query the player's shield.
    pub fn player_shield(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Shield, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|s| s.0)
    }

    /// Query the player's oxygen.
    pub fn player_oxygen(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Oxygen, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|o| o.0)
    }

    /// Query the player's vertical look angle.
    pub fn player_vertical_look(&mut self) -> Option<f32> {
        let mut query = self.world.query_filtered::<&crate::VerticalLook, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|v| v.0)
    }

    /// Query the player's current polygon index.
    pub fn player_polygon(&mut self) -> Option<usize> {
        let mut query = self.world.query_filtered::<&crate::PolygonIndex, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
    }

    /// Return all active entities with their rendering state.
    ///
    /// This includes monsters (not Dead), projectiles, items, and effects.
    /// Does not include the player entity (queried separately).
    pub fn entities(&mut self) -> Vec<EntityRenderState> {
        let mut result = Vec::new();

        // Monsters (exclude Dead)
        {
            let mut query = self.world.query::<(
                &crate::Monster,
                &crate::Position,
                &crate::Facing,
                &crate::MonsterState,
                &crate::SpriteShape,
                &crate::AnimationFrame,
            )>();
            for (monster, pos, facing, state, shape, frame) in query.iter(&self.world) {
                if *state == crate::MonsterState::Dead {
                    continue;
                }
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Monster {
                        definition_index: monster.definition_index,
                    },
                    position: pos.0,
                    facing: facing.0,
                    shape: shape.0,
                    frame: frame.0,
                });
            }
        }

        // Projectiles
        {
            let mut query = self.world.query::<(
                &crate::Projectile,
                &crate::Position,
            )>();
            for (proj, pos) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Projectile {
                        definition_index: proj.definition_index,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: 0,
                    frame: 0,
                });
            }
        }

        // Items
        {
            let mut query = self.world.query::<(
                &crate::Item,
                &crate::Position,
                &crate::SpriteShape,
                &crate::AnimationFrame,
            )>();
            for (item, pos, shape, frame) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Item {
                        item_type: item.item_type,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: shape.0,
                    frame: frame.0,
                });
            }
        }

        // Effects
        {
            let mut query = self.world.query::<(
                &crate::Effect,
                &crate::Position,
            )>();
            for (effect, pos) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Effect {
                        definition_index: effect.definition_index,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: 0,
                    frame: 0,
                });
            }
        }

        result
    }
}

/// Rendering data for an active entity.
#[derive(Debug, Clone)]
pub struct EntityRenderState {
    pub entity_type: RenderEntityType,
    pub position: glam::Vec3,
    pub facing: f32,
    pub shape: u16,
    pub frame: u16,
}

/// Type of entity for rendering purposes.
#[derive(Debug, Clone)]
pub enum RenderEntityType {
    Monster { definition_index: usize },
    Projectile { definition_index: usize },
    Item { item_type: i16 },
    Effect { definition_index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_flags_contains() {
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY);
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
        assert!(flags.contains(ActionFlags::FIRE_PRIMARY));
        assert!(!flags.contains(ActionFlags::STRAFE_LEFT));
    }

    #[test]
    fn action_flags_empty() {
        let flags = ActionFlags::default();
        assert!(flags.is_empty());
    }

    #[test]
    fn tick_input_mouse_deltas_round_trip() {
        let input = TickInput {
            action_flags: ActionFlags::new(ActionFlags::MOVE_FORWARD),
            mouse_yaw: 0.1,
            mouse_pitch: -0.05,
        };
        assert_eq!(input.mouse_yaw, 0.1);
        assert_eq!(input.mouse_pitch, -0.05);
        assert!(input.action_flags.contains(ActionFlags::MOVE_FORWARD));

        // Verify it round-trips through bevy_ecs resource insertion
        let mut world = bevy_ecs::prelude::World::new();
        world.insert_resource(TickInput {
            action_flags: ActionFlags::new(ActionFlags::MOVE_FORWARD),
            mouse_yaw: 0.1,
            mouse_pitch: -0.05,
        });
        let retrieved = world.resource::<TickInput>();
        assert_eq!(retrieved.mouse_yaw, 0.1);
        assert_eq!(retrieved.mouse_pitch, -0.05);
        assert!(retrieved.action_flags.contains(ActionFlags::MOVE_FORWARD));
    }
}
