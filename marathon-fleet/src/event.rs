//! Event stage: the CloudEvents-shaped envelope and shared cross-stage types.
//!
//! These types are the contract every pipeline stage speaks: the wire envelope
//! that carries each captured signal, the `EntityDesc` desired-set element the
//! reconciler diffs, the `GameAction` the interaction surface emits, and the
//! `EntityKind`/`EntityState` taxonomies. The variant sets here are deliberately
//! small starting points and are expected to expand as later stages land.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Opaque identity of an entity addressed by a [`GameAction`].
///
/// A `String` for now (lane ids and subjects are already strings on the wire);
/// documented so a future switch to a numeric id is a localized change.
pub type EntityId = String;

/// CloudEvents-shaped event envelope carrying one captured signal.
///
/// `data` is an arbitrary JSON payload. The on-wire `type` field is a Rust
/// keyword, so it is stored as `event_type` and renamed back to `"type"` for
/// (de)serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique event id (dedupe key — at-least-once delivery applies once).
    pub id: String,
    /// Producer-owned monotonic sequence number (the ordering authority).
    pub seq: u64,
    /// Producer event time.
    pub time: String,
    /// Time the consumer ingested the event.
    pub ingest_time: String,
    /// Subject the event is about (per-entity fold key).
    pub subject: String,
    /// CloudEvents `type` (Rust-keyword-safe alias).
    #[serde(rename = "type")]
    pub event_type: String,
    /// Arbitrary event payload.
    pub data: Value,
    /// Correlation id linking related events.
    pub correlation_id: String,
    /// Causation id of the event that caused this one.
    pub causation_id: String,
}

/// One element of the desired-set the reconciler diffs each tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDesc {
    /// Opaque lane identity — the sole reconcile key (never label/attribute).
    #[serde(rename = "laneId")]
    pub lane_id: String,
    /// Coarse species/category this entity embodies.
    pub kind: EntityKind,
    /// Human-facing label (may change without changing identity).
    pub label: String,
    /// Lifecycle state.
    pub state: EntityState,
    /// Free-form metadata bag.
    pub meta: HashMap<String, String>,
}

/// A gated action targeting an entity, emitted by the interaction surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameAction {
    /// Inspect/check-in on an entity (ungated, read-only).
    Inspect { id: EntityId },
    /// Poke an entity (offer-help / ask-to-break class).
    Poke { id: EntityId },
    /// Kill/retire an entity (send-home / retire class).
    Kill { id: EntityId },
}

/// Coarse species/category of an entity.
///
/// A small starting taxonomy; expected to expand as embodiment lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityKind {
    /// A fleet agent (a lane).
    Agent,
    /// A non-agent hostile.
    Monster,
    /// Kind not yet determined.
    Unknown,
}

/// Lifecycle state of an entity.
///
/// A small starting set; expected to expand (e.g. blocked/finished) as the
/// reconciler and embodiment stages land.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityState {
    /// Entering the world.
    Spawning,
    /// Live and working.
    Active,
    /// Live but idle.
    Idle,
    /// Leaving the world.
    Despawning,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelope() -> EventEnvelope {
        EventEnvelope {
            id: "evt-1".to_string(),
            seq: 7,
            time: "2026-06-17T00:00:00Z".to_string(),
            ingest_time: "2026-06-17T00:00:01Z".to_string(),
            subject: "lane-abc".to_string(),
            event_type: "fleet.delta".to_string(),
            data: serde_json::json!({ "k": "v", "n": 3 }),
            correlation_id: "corr-1".to_string(),
            causation_id: "cause-1".to_string(),
        }
    }

    #[test]
    fn envelope_serde_round_trip() {
        let original = sample_envelope();
        let json = serde_json::to_string(&original).expect("serialize");
        let back: EventEnvelope = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, back);
    }

    #[test]
    fn envelope_type_field_serializes_as_type_key() {
        let env = sample_envelope();
        let value: Value = serde_json::to_value(&env).expect("to_value");
        let obj = value.as_object().expect("object");
        assert!(obj.contains_key("type"), "expected wire key `type`");
        assert!(
            !obj.contains_key("event_type"),
            "internal name must not leak to the wire"
        );
        assert_eq!(obj["type"], serde_json::json!("fleet.delta"));
    }

    #[test]
    fn entity_desc_constructs_and_round_trips() {
        let mut meta = HashMap::new();
        meta.insert("persona".to_string(), "durandal".to_string());
        let desc = EntityDesc {
            lane_id: "lane-abc".to_string(),
            kind: EntityKind::Agent,
            label: "build the thing".to_string(),
            state: EntityState::Active,
            meta,
        };
        let json = serde_json::to_string(&desc).expect("serialize");
        // camelCase rename is in effect on the wire.
        assert!(json.contains("\"laneId\""));
        let back: EntityDesc = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(desc, back);
    }

    #[test]
    fn game_action_variants_match_and_carry_id() {
        let actions = [
            GameAction::Inspect {
                id: "a".to_string(),
            },
            GameAction::Poke {
                id: "b".to_string(),
            },
            GameAction::Kill {
                id: "c".to_string(),
            },
        ];
        for action in actions {
            let id = match &action {
                GameAction::Inspect { id } => id,
                GameAction::Poke { id } => id,
                GameAction::Kill { id } => id,
            };
            assert!(!id.is_empty());
        }
    }

    #[test]
    fn entity_kind_and_state_round_trip() {
        for kind in [EntityKind::Agent, EntityKind::Monster, EntityKind::Unknown] {
            let json = serde_json::to_string(&kind).expect("serialize kind");
            let back: EntityKind = serde_json::from_str(&json).expect("deserialize kind");
            assert_eq!(kind, back);
        }
        for state in [
            EntityState::Spawning,
            EntityState::Active,
            EntityState::Idle,
            EntityState::Despawning,
        ] {
            let json = serde_json::to_string(&state).expect("serialize state");
            let back: EntityState = serde_json::from_str(&json).expect("deserialize state");
            assert_eq!(state, back);
        }
    }
}
