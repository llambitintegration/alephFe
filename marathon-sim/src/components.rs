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

/// Angular velocity for turning (radians per tick).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AngularVelocity(pub f32);

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
    /// Number of contrail effects spawned so far.
    pub contrails_spawned: u16,
    /// Ticks this projectile has been alive.
    pub ticks_alive: u16,
    /// Current polygon index for spatial queries.
    pub current_polygon: usize,
}

/// Tracks which entity fired a projectile (for friendly-fire tracking).
/// Not serialized since Entity IDs are ephemeral.
#[derive(Component, Debug, Clone, Copy)]
pub struct ProjectileSource(pub bevy_ecs::entity::Entity);

/// Homing target position for guided projectiles.
/// Only present on projectiles with the GUIDED flag.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HomingTarget(pub Vec3);

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

/// Platform behavior type (Marathon platform definition type field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformType {
    ExtendsFloorToCeiling = 0,
    ExtendsCeilingToFloor = 1,
    ExtendsFloorAndCeiling = 2,
    FromFloor = 3,
    FromCeiling = 4,
    Teleporter = 5,
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
    Random,
    Fluorescent,
}

/// The phase of a light's activation state machine.
///
/// Lights cycle through six states. The activation half
/// (`BecomingActive`, `PrimaryActive`, `SecondaryActive`) ramps a light up and
/// holds it lit; the deactivation half (`BecomingInactive`, `PrimaryInactive`,
/// `SecondaryInactive`) ramps it down and holds it dark. The cycle wraps from
/// `SecondaryInactive` back to `BecomingActive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightState {
    BecomingActive,
    PrimaryActive,
    SecondaryActive,
    BecomingInactive,
    PrimaryInactive,
    SecondaryInactive,
}

impl LightState {
    /// Returns the next state in the activation cycle.
    ///
    /// `BecomingActive → PrimaryActive → SecondaryActive → BecomingInactive →
    /// PrimaryInactive → SecondaryInactive → BecomingActive`.
    pub fn next_state(self) -> LightState {
        match self {
            LightState::BecomingActive => LightState::PrimaryActive,
            LightState::PrimaryActive => LightState::SecondaryActive,
            LightState::SecondaryActive => LightState::BecomingInactive,
            LightState::BecomingInactive => LightState::PrimaryInactive,
            LightState::PrimaryInactive => LightState::SecondaryInactive,
            LightState::SecondaryInactive => LightState::BecomingActive,
        }
    }
}

/// High-level category of a light, mirroring Alephone's light kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightType {
    Normal,
    Strobe,
    Media,
}

/// Per-state lighting function parameters.
///
/// Each of a light's six states carries one of these specs describing how the
/// intensity behaves while the light is in that state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LightFunctionSpec {
    /// The animation function used while in this state.
    pub function: LightFunction,
    /// Base duration of this state in ticks.
    pub period: u16,
    /// Random additional duration (0..=delta_period) applied per transition.
    pub delta_period: u16,
    /// Base target intensity (0.0 to 1.0).
    pub intensity: f32,
    /// Random additional intensity applied per transition.
    pub delta_intensity: f32,
}

/// Light flag: the light starts in the active half of the cycle.
pub const LIGHT_IS_INITIALLY_ACTIVE: u16 = 0x0001;
/// Light flag: secondary states reuse the primary states' intensity values.
pub const LIGHT_HAS_SLAVED_INTENSITIES: u16 = 0x0002;
/// Light flag: the light does not run the activation state machine.
pub const LIGHT_IS_STATELESS: u16 = 0x0004;

// ─── Media Components ──────────────────────────────────────────────────────

/// Liquid media state.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    /// Index in map_data.media (for polygon lookup).
    pub index: usize,
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
        // New variants (box 1.4 / 1.8).
        assert_ne!(LightFunction::Random, LightFunction::Fluorescent);
        assert_ne!(LightFunction::Random, LightFunction::Flicker);
        assert_ne!(LightFunction::Fluorescent, LightFunction::Constant);
    }

    #[test]
    fn light_state_variants() {
        assert_ne!(LightState::BecomingActive, LightState::PrimaryActive);
        assert_ne!(LightState::PrimaryInactive, LightState::SecondaryInactive);
        assert_eq!(LightState::BecomingActive, LightState::BecomingActive);
    }

    #[test]
    fn light_state_next_state_cycle() {
        // The cycle must visit all six states and wrap back to the start.
        let mut state = LightState::BecomingActive;
        let expected = [
            LightState::PrimaryActive,
            LightState::SecondaryActive,
            LightState::BecomingInactive,
            LightState::PrimaryInactive,
            LightState::SecondaryInactive,
            LightState::BecomingActive,
        ];
        for want in expected {
            state = state.next_state();
            assert_eq!(state, want);
        }
        // After six transitions we are back where we started.
        assert_eq!(state, LightState::BecomingActive);
    }

    #[test]
    fn light_type_variants() {
        assert_ne!(LightType::Normal, LightType::Strobe);
        assert_ne!(LightType::Strobe, LightType::Media);
        assert_eq!(LightType::Media, LightType::Media);
    }

    #[test]
    fn light_function_spec_construction() {
        let spec = LightFunctionSpec {
            function: LightFunction::Smooth,
            period: 30,
            delta_period: 5,
            intensity: 0.75,
            delta_intensity: 0.25,
        };
        assert_eq!(spec.function, LightFunction::Smooth);
        assert_eq!(spec.period, 30);
        assert_eq!(spec.delta_period, 5);
        assert_eq!(spec.intensity, 0.75);
        assert_eq!(spec.delta_intensity, 0.25);

        let copied = spec; // Copy
        assert_eq!(copied, spec);
    }

    #[test]
    fn light_flag_constants() {
        assert_eq!(LIGHT_IS_INITIALLY_ACTIVE, 0x0001);
        assert_eq!(LIGHT_HAS_SLAVED_INTENSITIES, 0x0002);
        assert_eq!(LIGHT_IS_STATELESS, 0x0004);
        // Flags are independent bits.
        assert_eq!(
            LIGHT_IS_INITIALLY_ACTIVE & LIGHT_HAS_SLAVED_INTENSITIES,
            0
        );
        assert_eq!(LIGHT_HAS_SLAVED_INTENSITIES & LIGHT_IS_STATELESS, 0);
    }

    #[test]
    fn light_state_variants_distinct_and_copy_debug() {
        // All 6 variants exist.
        let all = [
            LightState::BecomingActive,
            LightState::PrimaryActive,
            LightState::SecondaryActive,
            LightState::BecomingInactive,
            LightState::PrimaryInactive,
            LightState::SecondaryInactive,
        ];

        // Pairwise distinct.
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }

        // Explicit spot-check on a key transition pair.
        assert_ne!(LightState::BecomingActive, LightState::PrimaryActive);

        // Copy + Clone + Debug smoke check.
        let a = LightState::SecondaryInactive;
        let b = a; // Copy
        let c = a.clone(); // Clone
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(
            format!("{:?}", LightState::PrimaryInactive),
            "PrimaryInactive"
        );
    }

    #[test]
    fn platform_type_discriminants_clone_copy_eq_and_serde() {
        // Explicit discriminants.
        assert_eq!(PlatformType::ExtendsFloorToCeiling as i32, 0);
        assert_eq!(PlatformType::ExtendsCeilingToFloor as i32, 1);
        assert_eq!(PlatformType::ExtendsFloorAndCeiling as i32, 2);
        assert_eq!(PlatformType::FromFloor as i32, 3);
        assert_eq!(PlatformType::FromCeiling as i32, 4);
        assert_eq!(PlatformType::Teleporter as i32, 5);

        // Copy + Clone + PartialEq behavior.
        let a = PlatformType::Teleporter;
        let b = a; // Copy
        let c = a.clone(); // Clone
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_ne!(PlatformType::FromFloor, PlatformType::FromCeiling);

        // Serde round-trip (bincode is the serde codec available in this crate;
        // serde_json is not a dependency and may not be added by this box).
        let bytes = bincode::serialize(&PlatformType::FromCeiling).unwrap();
        let back: PlatformType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back, PlatformType::FromCeiling);
    }
}
