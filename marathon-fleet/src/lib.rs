//! `marathon-fleet` — the out-of-process fleet daemon library.
//!
//! This crate hosts the agent-dashboard ("Mode A") pipeline that turns a live
//! fleet event feed into a desired-set of in-world agent monsters and routes
//! gated care verbs back out as signed operator hints. It runs as a separate
//! process from the simulation and must never block the sim tick loop: the sim
//! reads the latest desired-set over a latest-wins channel and tolerates a
//! dead/absent daemon by rendering an empty desired-set.
//!
//! Module map (each stage of the capture -> projection -> reconcile -> interact
//! -> replay pipeline):
//! - [`transport`]: abstract live event sources (SSE / MQTT) feeding the daemon.
//! - [`event_log`]: the append-only NDJSON event log (resumable byte cursor).
//! - [`event`]: the CloudEvents-shaped envelope and shared cross-stage types.
//! - [`projection`]: the pure deterministic event-sourced reducer.
//! - [`reconciler`]: the per-tick desired-set diff (spawn / update / despawn).
//! - [`embodiment`]: the pure per-channel body mapping (species/pose/facing/glow).
//! - [`placement`]: the pure lease-stream -> spatial-home (room/corridor) mapping.
//! - [`interaction`]: care-verb interaction and broker-signed operator hints.
//! - [`replay`]: the switchable view-clock live/replay render layer.

pub mod embodiment;
pub mod event;
pub mod event_log;
pub mod interaction;
pub mod placement;
pub mod projection;
pub mod reconciler;
pub mod replay;
pub mod transport;

pub use embodiment::{
    body_view_of, completion_beat_for, completion_beat_of, damage_flash_for, damage_flash_of,
    facing_for, facing_of, glow_for, glow_of, hitl_signal_for, hitl_signal_of, label_overlay_for,
    pose_for, pose_of, quest_status_for, quest_status_of, species_for, BodyView, CompletionBeat,
    DamageFlash, DecorativeTool, Facing, Glow, HitlBeacon, HitlSignal, LabelOverlay, LifecyclePose,
    ProgressPhase, QuestStatus, Species, SpeciesColor,
};
pub use event::{EntityDesc, EntityId, EntityKind, EntityState, EventEnvelope, GameAction};
pub use event_log::{Checkpoint, Cursor, EventLog, LogLine};
pub use placement::{lease_of, placement_for, placement_of, Lease, PlaceKind, Placement};
pub use transport::{capture, CaptureBuffer, LiveEventSource, ReplaySource};
