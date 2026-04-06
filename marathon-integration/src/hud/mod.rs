mod health;
mod inventory;
mod motion_sensor;
mod oxygen;
pub mod pipeline;
mod weapon;

pub use health::HealthBar;
pub use inventory::InventoryPanel;
pub use motion_sensor::MotionSensor;
pub use oxygen::OxygenMeter;
pub use pipeline::HudPipeline;
pub use weapon::WeaponDisplay;

/// State read from the simulation each frame for HUD rendering.
#[derive(Debug, Clone)]
pub struct HudState {
    pub health: i16,
    pub max_health: i16,
    pub shield: i16,
    pub oxygen: i16,
    pub max_oxygen: i16,
    pub in_vacuum: bool,
    pub weapon_icon_index: Option<u16>,
    pub primary_ammo: Option<u16>,
    pub secondary_ammo: Option<u16>,
    pub inventory_items: Vec<InventoryItem>,
    pub player_x: i32,
    pub player_y: i32,
    pub player_facing: u16,
    pub nearby_entities: Vec<RadarEntity>,
}

/// An item in the player's inventory.
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub icon_index: u16,
    pub count: u16,
}

/// An entity visible on the motion sensor.
#[derive(Debug, Clone)]
pub struct RadarEntity {
    pub x: i32,
    pub y: i32,
    pub entity_type: RadarEntityType,
}

/// Entity type for motion sensor dot coloring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarEntityType {
    Enemy,
    Ally,
    Item,
}

/// HUD layout parameters for a given resolution.
#[derive(Debug, Clone)]
pub struct HudLayout {
    pub screen_width: u32,
    pub screen_height: u32,
    pub scale: f32,
}

impl HudLayout {
    /// Compute layout parameters for a given resolution.
    /// The base resolution is 640x480 (Marathon's original).
    pub fn for_resolution(width: u32, height: u32) -> Self {
        let scale = (width as f32 / 640.0).min(height as f32 / 480.0);
        Self {
            screen_width: width,
            screen_height: height,
            scale,
        }
    }
}

/// Shield strength tier for color selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldTier {
    None,
    Single,
    Double,
    Triple,
}

impl ShieldTier {
    pub fn from_shield_value(shield: i16) -> Self {
        match shield {
            s if s <= 0 => ShieldTier::None,
            s if s <= 150 => ShieldTier::Single,
            s if s <= 300 => ShieldTier::Double,
            _ => ShieldTier::Triple,
        }
    }
}
