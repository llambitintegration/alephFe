use super::{HudLayout, RadarEntity, RadarEntityType};

/// Motion sensor (radar) rendering data.
pub struct MotionSensor {
    /// Screen-space center of the radar circle.
    pub center: [f32; 2],
    /// Screen-space radius of the radar circle.
    pub radius: f32,
    /// Radar dots to render, in screen-space relative to center.
    pub dots: Vec<RadarDot>,
}

/// A single dot on the motion sensor.
pub struct RadarDot {
    /// Offset from radar center in screen pixels.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Color based on entity type.
    pub entity_type: RadarEntityType,
}

/// Maximum world-unit range of the motion sensor.
const SENSOR_RANGE: f32 = 30.0;

impl MotionSensor {
    /// Compute motion sensor rendering data.
    ///
    /// `player_x`, `player_y`: player world position
    /// `player_facing`: player facing angle (0..65536 fixed-point, 0 = east, increases counterclockwise)
    /// `entities`: nearby entities with their world positions and types
    pub fn compute(
        player_x: i32,
        player_y: i32,
        player_facing: u16,
        entities: &[RadarEntity],
        layout: &HudLayout,
    ) -> Self {
        let radius = 48.0 * layout.scale;
        let center_x = layout.screen_width as f32 / 2.0;
        let center_y = layout.screen_height as f32 - radius - 20.0 * layout.scale;

        let facing_rad = (player_facing as f32 / 65536.0) * std::f32::consts::TAU;

        let mut dots = Vec::new();
        for entity in entities {
            let dx = (entity.x - player_x) as f32;
            let dy = (entity.y - player_y) as f32;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > SENSOR_RANGE {
                continue;
            }

            // Angle from player to entity in world space
            let angle_to_entity = dy.atan2(dx);
            // Relative angle (rotate by negative player facing)
            let relative_angle = angle_to_entity - facing_rad;

            let normalized_dist = dist / SENSOR_RANGE;
            let screen_dist = normalized_dist * radius;

            // On the radar: up is forward (negative Y in screen space)
            let offset_x = relative_angle.sin() * screen_dist;
            let offset_y = -relative_angle.cos() * screen_dist;

            dots.push(RadarDot {
                offset_x,
                offset_y,
                entity_type: entity.entity_type,
            });
        }

        Self {
            center: [center_x, center_y],
            radius,
            dots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_beyond_range_excluded() {
        let entities = vec![RadarEntity {
            x: 100,
            y: 100,
            entity_type: RadarEntityType::Enemy,
        }];
        let layout = HudLayout::for_resolution(640, 480);
        // Player at origin, entity at (100, 100) = distance ~141, well beyond range 30
        let sensor = MotionSensor::compute(0, 0, 0, &entities, &layout);
        assert!(sensor.dots.is_empty());
    }

    #[test]
    fn entity_within_range_included() {
        let entities = vec![RadarEntity {
            x: 10,
            y: 0,
            entity_type: RadarEntityType::Ally,
        }];
        let layout = HudLayout::for_resolution(640, 480);
        let sensor = MotionSensor::compute(0, 0, 0, &entities, &layout);
        assert_eq!(sensor.dots.len(), 1);
        assert_eq!(sensor.dots[0].entity_type, RadarEntityType::Ally);
    }

    #[test]
    fn radar_has_correct_center() {
        let layout = HudLayout::for_resolution(640, 480);
        let sensor = MotionSensor::compute(0, 0, 0, &[], &layout);
        assert!((sensor.center[0] - 320.0).abs() < f32::EPSILON);
    }
}
