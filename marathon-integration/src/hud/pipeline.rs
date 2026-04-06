use super::{HudLayout, HudState};
use super::health::HealthBar;
use super::inventory::InventoryPanel;
use super::motion_sensor::MotionSensor;
use super::oxygen::OxygenMeter;
use super::weapon::WeaponDisplay;

/// RGBA color represented as [r, g, b, a] with values in 0.0..1.0.
pub type Color = [f32; 4];

/// Colors used for HUD rendering, matching Marathon's original palette.
pub mod colors {
    use super::Color;

    pub const HEALTH_BAR: Color = [0.0, 0.8, 0.0, 1.0];
    pub const SHIELD_SINGLE: Color = [0.0, 0.6, 1.0, 1.0];
    pub const SHIELD_DOUBLE: Color = [1.0, 1.0, 0.0, 1.0];
    pub const SHIELD_TRIPLE: Color = [1.0, 0.0, 0.0, 1.0];
    pub const SHIELD_NONE: Color = [0.3, 0.3, 0.3, 1.0];
    pub const OXYGEN_NORMAL: Color = [0.0, 0.6, 1.0, 1.0];
    pub const OXYGEN_CRITICAL: Color = [1.0, 0.2, 0.2, 1.0];
    pub const BAR_BACKGROUND: Color = [0.1, 0.1, 0.1, 0.8];
    pub const RADAR_BACKGROUND: Color = [0.0, 0.15, 0.0, 0.8];
    pub const RADAR_ENEMY: Color = [1.0, 0.0, 0.0, 1.0];
    pub const RADAR_ALLY: Color = [0.0, 1.0, 0.0, 1.0];
    pub const RADAR_ITEM: Color = [1.0, 1.0, 0.0, 1.0];
    pub const AMMO_TEXT: Color = [0.0, 1.0, 0.0, 1.0];
    pub const INVENTORY_BACKGROUND: Color = [0.1, 0.1, 0.1, 0.6];
}

/// A 2D quad to be rendered in the HUD overlay pass.
#[derive(Debug, Clone)]
pub struct HudQuad {
    /// Screen-space rectangle [x, y, width, height].
    pub rect: [f32; 4],
    /// Fill color.
    pub color: Color,
}

/// A circle to be rendered in the HUD overlay pass.
#[derive(Debug, Clone)]
pub struct HudCircle {
    /// Screen-space center [x, y].
    pub center: [f32; 2],
    /// Radius in screen pixels.
    pub radius: f32,
    /// Fill color.
    pub color: Color,
}

/// A small dot (for radar entities).
#[derive(Debug, Clone)]
pub struct HudDot {
    /// Screen-space position [x, y].
    pub position: [f32; 2],
    /// Dot radius in screen pixels.
    pub radius: f32,
    /// Fill color.
    pub color: Color,
}

/// A sprite reference for the HUD (weapon icon, inventory items).
#[derive(Debug, Clone)]
pub struct HudSprite {
    /// Screen-space rectangle [x, y, width, height].
    pub rect: [f32; 4],
    /// Shape collection index to look up in the interface collection.
    pub shape_index: u16,
}

/// Text to render on the HUD (ammo counts, inventory counts).
#[derive(Debug, Clone)]
pub struct HudText {
    /// Screen-space position [x, y].
    pub position: [f32; 2],
    /// Text content.
    pub text: String,
    /// Text color.
    pub color: Color,
    /// Font size in pixels (scaled).
    pub font_size: f32,
}

/// All draw commands for a single HUD frame, produced by the HUD pipeline.
///
/// The wgpu render pass consumes these commands to draw the HUD overlay
/// on top of the 3D scene framebuffer.
#[derive(Debug, Clone, Default)]
pub struct HudDrawList {
    pub quads: Vec<HudQuad>,
    pub circles: Vec<HudCircle>,
    pub dots: Vec<HudDot>,
    pub sprites: Vec<HudSprite>,
    pub texts: Vec<HudText>,
}

/// The HUD render pipeline. Reads simulation state and produces draw commands
/// for the wgpu 2D overlay pass.
pub struct HudPipeline {
    layout: HudLayout,
}

impl HudPipeline {
    /// Create a new HUD pipeline for the given resolution.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layout: HudLayout::for_resolution(width, height),
        }
    }

    /// Update the layout when the window is resized.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.layout = HudLayout::for_resolution(width, height);
    }

    /// Produce the full set of draw commands for the current HUD state.
    pub fn build_draw_list(&self, state: &HudState) -> HudDrawList {
        let mut list = HudDrawList::default();

        self.build_health_bars(state, &mut list);
        self.build_oxygen_meter(state, &mut list);
        self.build_weapon_display(state, &mut list);
        self.build_motion_sensor(state, &mut list);
        self.build_inventory_panel(state, &mut list);

        list
    }

    fn build_health_bars(&self, state: &HudState, list: &mut HudDrawList) {
        let bar = HealthBar::compute(state.health, state.max_health, state.shield, &self.layout);

        // Health bar background
        list.quads.push(HudQuad {
            rect: bar.health_rect,
            color: colors::BAR_BACKGROUND,
        });
        // Health bar fill
        let mut fill_rect = bar.health_rect;
        fill_rect[2] *= bar.health_fraction;
        list.quads.push(HudQuad {
            rect: fill_rect,
            color: colors::HEALTH_BAR,
        });

        // Shield bar background
        list.quads.push(HudQuad {
            rect: bar.shield_rect,
            color: colors::BAR_BACKGROUND,
        });
        // Shield bar fill
        let shield_color = match bar.shield_tier {
            super::ShieldTier::None => colors::SHIELD_NONE,
            super::ShieldTier::Single => colors::SHIELD_SINGLE,
            super::ShieldTier::Double => colors::SHIELD_DOUBLE,
            super::ShieldTier::Triple => colors::SHIELD_TRIPLE,
        };
        let mut shield_fill = bar.shield_rect;
        shield_fill[2] *= bar.shield_fraction;
        list.quads.push(HudQuad {
            rect: shield_fill,
            color: shield_color,
        });
    }

    fn build_oxygen_meter(&self, state: &HudState, list: &mut HudDrawList) {
        let meter = OxygenMeter::compute(
            state.oxygen,
            state.max_oxygen,
            state.in_vacuum,
            &self.layout,
        );
        if !meter.visible {
            return;
        }

        // Background
        list.quads.push(HudQuad {
            rect: meter.rect,
            color: colors::BAR_BACKGROUND,
        });
        // Fill
        let mut fill = meter.rect;
        fill[2] *= meter.oxygen_fraction;
        let color = if meter.critical {
            colors::OXYGEN_CRITICAL
        } else {
            colors::OXYGEN_NORMAL
        };
        list.quads.push(HudQuad {
            rect: fill,
            color,
        });
    }

    fn build_weapon_display(&self, state: &HudState, list: &mut HudDrawList) {
        let display = WeaponDisplay::compute(
            state.weapon_icon_index,
            state.primary_ammo,
            state.secondary_ammo,
            &self.layout,
        );

        if let Some(icon_idx) = display.weapon_icon_index {
            list.sprites.push(HudSprite {
                rect: display.icon_rect,
                shape_index: icon_idx,
            });
        }

        let font_size = 14.0 * self.layout.scale;
        if let Some(ammo) = display.primary_ammo {
            list.texts.push(HudText {
                position: display.primary_ammo_pos,
                text: ammo.to_string(),
                color: colors::AMMO_TEXT,
                font_size,
            });
        }
        if let Some(ammo) = display.secondary_ammo {
            list.texts.push(HudText {
                position: display.secondary_ammo_pos,
                text: ammo.to_string(),
                color: colors::AMMO_TEXT,
                font_size,
            });
        }
    }

    fn build_motion_sensor(&self, state: &HudState, list: &mut HudDrawList) {
        use super::RadarEntityType;

        let sensor = MotionSensor::compute(
            state.player_x,
            state.player_y,
            state.player_facing,
            &state.nearby_entities,
            &self.layout,
        );

        // Radar background circle
        list.circles.push(HudCircle {
            center: sensor.center,
            radius: sensor.radius,
            color: colors::RADAR_BACKGROUND,
        });

        // Entity dots
        let dot_radius = 3.0 * self.layout.scale;
        for dot in &sensor.dots {
            let color = match dot.entity_type {
                RadarEntityType::Enemy => colors::RADAR_ENEMY,
                RadarEntityType::Ally => colors::RADAR_ALLY,
                RadarEntityType::Item => colors::RADAR_ITEM,
            };
            list.dots.push(HudDot {
                position: [
                    sensor.center[0] + dot.offset_x,
                    sensor.center[1] + dot.offset_y,
                ],
                radius: dot_radius,
                color,
            });
        }
    }

    fn build_inventory_panel(&self, state: &HudState, list: &mut HudDrawList) {
        let panel = InventoryPanel::compute(&state.inventory_items, &self.layout);
        if !panel.visible {
            return;
        }

        for slot in &panel.items {
            // Slot background
            list.quads.push(HudQuad {
                rect: slot.rect,
                color: colors::INVENTORY_BACKGROUND,
            });
            // Item icon
            list.sprites.push(HudSprite {
                rect: slot.rect,
                shape_index: slot.icon_index,
            });
            // Item count
            if slot.count > 1 {
                let font_size = 10.0 * self.layout.scale;
                list.texts.push(HudText {
                    position: [
                        slot.rect[0] + slot.rect[2] - font_size,
                        slot.rect[1] + slot.rect[3] - font_size,
                    ],
                    text: slot.count.to_string(),
                    color: colors::AMMO_TEXT,
                    font_size,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{InventoryItem, RadarEntity, RadarEntityType};

    fn sample_hud_state() -> HudState {
        HudState {
            health: 100,
            max_health: 150,
            shield: 150,
            oxygen: 80,
            max_oxygen: 100,
            in_vacuum: false,
            weapon_icon_index: Some(5),
            primary_ammo: Some(42),
            secondary_ammo: Some(3),
            inventory_items: vec![
                InventoryItem { icon_index: 1, count: 2 },
            ],
            player_x: 0,
            player_y: 0,
            player_facing: 0,
            nearby_entities: vec![
                RadarEntity { x: 10, y: 0, entity_type: RadarEntityType::Enemy },
            ],
        }
    }

    #[test]
    fn pipeline_produces_draw_list() {
        let pipeline = HudPipeline::new(640, 480);
        let state = sample_hud_state();
        let list = pipeline.build_draw_list(&state);

        // Should have health bg + health fill + shield bg + shield fill = 4 quads minimum
        assert!(list.quads.len() >= 4);
        // Weapon sprite
        assert!(!list.sprites.is_empty());
        // Ammo text
        assert!(!list.texts.is_empty());
        // Radar circle
        assert_eq!(list.circles.len(), 1);
        // Radar dot for the enemy
        assert_eq!(list.dots.len(), 1);
    }

    #[test]
    fn oxygen_hidden_when_not_in_vacuum() {
        let pipeline = HudPipeline::new(640, 480);
        let mut state = sample_hud_state();
        state.in_vacuum = false;
        let list = pipeline.build_draw_list(&state);

        // Only health + shield bars (4 quads) + inventory bg (1 quad) = 5 quads
        // No oxygen quads since not in vacuum
        assert_eq!(list.quads.len(), 5);
    }

    #[test]
    fn oxygen_visible_in_vacuum() {
        let pipeline = HudPipeline::new(640, 480);
        let mut state = sample_hud_state();
        state.in_vacuum = true;
        let list = pipeline.build_draw_list(&state);

        // Health + shield (4) + oxygen bg + oxygen fill (2) + inventory bg (1) = 7 quads
        assert_eq!(list.quads.len(), 7);
    }

    #[test]
    fn resize_updates_layout() {
        let mut pipeline = HudPipeline::new(640, 480);
        let state = sample_hud_state();
        let list_640 = pipeline.build_draw_list(&state);

        pipeline.resize(1920, 1080);
        let list_1920 = pipeline.build_draw_list(&state);

        // Health bar should be wider at higher resolution
        assert!(list_1920.quads[0].rect[2] > list_640.quads[0].rect[2]);
    }
}
