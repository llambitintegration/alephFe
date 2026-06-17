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
//! - [`event`]: the CloudEvents-shaped envelope and shared cross-stage types.
//! - [`projection`]: the pure deterministic event-sourced reducer.
//! - [`reconciler`]: the per-tick desired-set diff (spawn / update / despawn).
//! - [`interaction`]: care-verb interaction and broker-signed operator hints.
//! - [`replay`]: the switchable view-clock live/replay render layer.

pub mod event;
pub mod interaction;
pub mod projection;
pub mod reconciler;
pub mod replay;
pub mod transport;

pub use event::{EntityDesc, EntityId, EntityKind, EntityState, EventEnvelope, GameAction};
