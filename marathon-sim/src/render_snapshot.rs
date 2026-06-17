//! Per-frame serializable render snapshot (`decouple-tick-snapshot` step 2).
//!
//! `WorldSnapshot` is the single serializable render DTO every frontend consumes.
//! `SimWorld::render_snapshot` is a thin, read-only aggregator over the existing
//! sim accessors (`poly_dynamic_data`, `entities`, the `player_*` getters,
//! `player_weapon_state`, `drain_events`) — it does not reinvent any of them and
//! does not perturb sim state beyond draining the per-frame event queue.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::tick::{EntityRenderState, WeaponRenderState};
use crate::world::{PolyDynamicData, SimEvent, SimWorld};

/// Camera + HUD source for the local player, packed into the render snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub position: Vec3,
    pub facing: f32,
    pub vertical_look: f32,
    pub polygon_index: usize,
    pub health: i16,
    pub shield: i16,
    pub oxygen: i16,
}

/// One frame of serializable render state, aggregated from the sim's existing
/// render accessors. This is the canonical interface a frontend (or a headless
/// consumer) is handed in place of ~10 separate `&mut self` accessor calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick_count: u64,
    pub player: Option<PlayerView>,
    pub poly_dynamic: Vec<PolyDynamicData>,
    pub entities: Vec<EntityRenderState>,
    pub weapon: Option<WeaponRenderState>,
    pub events: Vec<SimEvent>,
}

impl SimWorld {
    /// Aggregate the current frame's render state into one serializable
    /// `WorldSnapshot`.
    ///
    /// This is a pure read-only aggregator over the existing render accessors;
    /// it queries the ECS but never mutates sim state, with the single
    /// documented exception that it drains the per-frame event queue (the same
    /// semantics as a frontend calling `drain_events` once per frame). `&mut
    /// self` is required only because bevy caches `QueryState` in `&mut World`.
    pub fn render_snapshot(&mut self) -> WorldSnapshot {
        // Player view: present only when a player entity exists. Every player_*
        // accessor returns Some/None together, so gate on position and unwrap
        // the rest with sensible defaults.
        let player = self.player_position().map(|position| PlayerView {
            position,
            facing: self.player_facing().unwrap_or(0.0),
            vertical_look: self.player_vertical_look().unwrap_or(0.0),
            polygon_index: self.player_polygon().unwrap_or(0),
            health: self.player_health().unwrap_or(0),
            shield: self.player_shield().unwrap_or(0),
            oxygen: self.player_oxygen().unwrap_or(0),
        });

        let poly_dynamic = self.poly_dynamic_data();
        let entities = self.entities();
        let weapon = self.player_weapon_state();
        let tick_count = self.tick_count();
        let events = self.drain_events();

        WorldSnapshot {
            tick_count,
            player,
            poly_dynamic,
            entities,
            weapon,
            events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::RenderEntityType;

    #[test]
    fn player_view_bincode_round_trip() {
        // box 2.1
        let view = PlayerView {
            position: Vec3::new(1.0, 2.0, 3.0),
            facing: 0.5,
            vertical_look: -0.25,
            polygon_index: 7,
            health: 150,
            shield: 100,
            oxygen: 90,
        };
        let bytes = bincode::serialize(&view).expect("serialize");
        let back: PlayerView = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(view, back);
    }

    #[test]
    fn world_snapshot_bincode_round_trip() {
        // box 2.2: hand-built snapshot survives bincode.
        let snap = WorldSnapshot {
            tick_count: 42,
            player: Some(PlayerView {
                position: Vec3::new(4.0, 5.0, 6.0),
                facing: 1.0,
                vertical_look: 0.0,
                polygon_index: 3,
                health: 120,
                shield: 80,
                oxygen: 70,
            }),
            poly_dynamic: vec![
                PolyDynamicData {
                    floor_height: 0.0,
                    ceiling_height: 3.0,
                    media_height: 0.0,
                    floor_light: 1.0,
                    ceiling_light: 0.5,
                },
                PolyDynamicData::default(),
            ],
            entities: vec![EntityRenderState {
                entity_type: RenderEntityType::Monster {
                    definition_index: 2,
                },
                position: Vec3::new(7.0, 8.0, 9.0),
                facing: 0.3,
                shape: 11,
                frame: 4,
            }],
            weapon: Some(WeaponRenderState {
                collection: 1,
                shape: 2,
                frame: 0,
                vertical_position: 0.5,
                horizontal_position: 0.25,
            }),
            events: vec![SimEvent::ItemPickedUp { item_type: 5 }],
        };

        let bytes = bincode::serialize(&snap).expect("serialize");
        let back: WorldSnapshot = bincode::deserialize(&bytes).expect("deserialize");

        assert_eq!(back.tick_count, snap.tick_count);
        assert_eq!(back.player, snap.player);
        assert_eq!(back.poly_dynamic, snap.poly_dynamic);
        assert_eq!(back.poly_dynamic.len(), 2);
        assert_eq!(back.entities.len(), 1);
        assert_eq!(back.entities[0].position, Vec3::new(7.0, 8.0, 9.0));
        assert!(back.weapon.is_some());
        assert_eq!(back.events.len(), 1);
        match back.events[0] {
            SimEvent::ItemPickedUp { item_type } => assert_eq!(item_type, 5),
            _ => panic!("wrong event variant"),
        }
    }
}
