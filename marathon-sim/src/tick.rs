use crate::world::{SimWorld, TickCounter};

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
    pub fn tick(&mut self, action_flags: ActionFlags) {
        // Store input for this tick
        self.world.insert_resource(TickInput { action_flags });

        // Run systems in order
        // TODO: Wire up actual system functions as they're implemented.
        // For now, just advance the tick counter.

        // Advance tick counter
        self.world.resource_mut::<TickCounter>().0 += 1;
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
}
