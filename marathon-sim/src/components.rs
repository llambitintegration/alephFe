use bevy_ecs::prelude::Component;
use glam::Vec3;
use serde::{Deserialize, Serialize};

// ─── Spatial Components ────────────────────────────────────────────────────

/// World-space position.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position(pub Vec3);

/// Velocity in world units per tick.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Velocity(pub Vec3);

/// Horizontal facing angle in radians (0 = east, increases counterclockwise).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Facing(pub f32);

/// Vertical look angle in radians (positive = up).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct VerticalLook(pub f32);

/// Collision radius for entity-vs-entity and entity-vs-wall checks.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionRadius(pub f32);

/// Height of the entity for ceiling clearance checks.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntityHeight(pub f32);

/// Index of the polygon the entity currently occupies.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PolygonIndex(pub usize);

/// Whether the entity is standing on a floor (not airborne).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Grounded(pub bool);

// ─── Vitality Components ───────────────────────────────────────────────────

/// Hit points. Reaching zero triggers death.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Health(pub i16);

/// Shield/armor points. Absorbs damage before health.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Shield(pub i16);

/// Oxygen supply (for vacuum/submersion). Depletes when submerged.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Oxygen(pub i16);

// ─── Entity Type Markers ───────────────────────────────────────────────────

/// Marks the player entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct Player;

/// Marks a monster entity. Stores the monster definition index.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Monster {
    pub definition_index: usize,
}

/// Marks a projectile entity.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Projectile {
    pub definition_index: usize,
    /// Distance traveled so far (for max range check).
    pub distance_traveled: f32,
}

/// Tracks which entity fired a projectile (for friendly-fire tracking).
/// Not serialized since Entity IDs are ephemeral.
#[derive(Component, Debug, Clone, Copy)]
pub struct ProjectileSource(pub bevy_ecs::entity::Entity);

/// Marks an item entity.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Item {
    pub item_type: i16,
}

/// Marks a visual effect entity.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Effect {
    pub definition_index: usize,
    pub ticks_remaining: u16,
}

// ─── Monster AI Components ─────────────────────────────────────────────────

/// Monster behavioral state.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonsterState {
    Idle,
    Alerted,
    Attacking,
    Moving,
    Fleeing,
    Dying,
    Dead,
}

/// The entity this monster is targeting.
/// The entity this monster is targeting. Not serialized (Entity IDs are ephemeral).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Target(pub Option<bevy_ecs::entity::Entity>);

/// Cooldown timer for monster attacks (ticks until next attack).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AttackCooldown(pub u16);

/// Whether this monster can fly.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Flying {
    pub preferred_hover_height: f32,
}

// ─── Combat Components ─────────────────────────────────────────────────────

/// Damage type immunities (bitmask matching marathon-formats DamageDefinition types).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Immunities(pub u32);

/// Damage type weaknesses (bitmask).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Weaknesses(pub u32);

// ─── Rendering Hints ───────────────────────────────────────────────────────

/// Shape descriptor for rendering (collection, clut, shape index).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpriteShape(pub u16);

/// Current animation frame index.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AnimationFrame(pub u16);

// ─── Platform Components ───────────────────────────────────────────────────

/// Marks a platform entity with its movement parameters.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    /// Polygon index this platform controls.
    pub polygon_index: usize,
    /// Resting floor height.
    pub floor_rest: f32,
    /// Extended floor height.
    pub floor_extended: f32,
    /// Resting ceiling height.
    pub ceiling_rest: f32,
    /// Extended ceiling height.
    pub ceiling_extended: f32,
    /// Current floor height.
    pub current_floor: f32,
    /// Current ceiling height.
    pub current_ceiling: f32,
    /// Movement speed in world units per tick.
    pub speed: f32,
    /// Current movement state.
    pub state: PlatformState,
    /// Delay ticks before returning to rest.
    pub return_delay: u16,
    /// Current delay countdown.
    pub delay_remaining: u16,
    /// Activation type flags.
    pub activation_flags: u32,
    /// Whether this platform crushes entities.
    pub crushes: bool,
}

/// Platform movement state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformState {
    AtRest,
    Extending,
    AtExtended,
    Returning,
}

// ─── Light Components ──────────────────────────────────────────────────────

/// Light animation parameters.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Light {
    /// Light index in the map data.
    pub light_index: usize,
    /// Animation function type.
    pub function: LightFunction,
    /// Period in ticks.
    pub period: u32,
    /// Phase offset in ticks.
    pub phase: u32,
    /// Minimum intensity (0.0 to 1.0).
    pub intensity_min: f32,
    /// Maximum intensity (0.0 to 1.0).
    pub intensity_max: f32,
    /// Current computed intensity.
    pub current_intensity: f32,
}

/// Light animation function types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightFunction {
    Constant,
    Linear,
    Smooth,
    Flicker,
}

// ─── Media Components ──────────────────────────────────────────────────────

/// Liquid media state.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    /// Polygon indices this media occupies.
    pub polygon_index: usize,
    /// Media type (water=0, lava=1, goo=2, sewage=3, jjaro=4).
    pub media_type: i16,
    /// Low height bound.
    pub height_low: f32,
    /// High height bound.
    pub height_high: f32,
    /// Associated light index for height animation.
    pub light_index: usize,
    /// Current surface height.
    pub current_height: f32,
    /// Current flow direction (radians) and magnitude.
    pub current_direction: f32,
    pub current_magnitude: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_default_construction() {
        let pos = Position(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(pos.0.x, 1.0);
        assert_eq!(pos.0.y, 2.0);
        assert_eq!(pos.0.z, 3.0);
    }

    #[test]
    fn monster_state_variants() {
        let state = MonsterState::Idle;
        assert_eq!(state, MonsterState::Idle);
        assert_ne!(state, MonsterState::Alerted);
    }

    #[test]
    fn platform_state_variants() {
        assert_ne!(PlatformState::AtRest, PlatformState::Extending);
    }

    #[test]
    fn light_function_variants() {
        assert_ne!(LightFunction::Constant, LightFunction::Flicker);
    }
}
