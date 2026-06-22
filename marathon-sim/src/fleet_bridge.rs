//! Reconciler-bridge: the channel seam between the out-of-process fleet daemon
//! and the in-process sim (box 1.4; design Decision 4).
//!
//! The daemon feeds the sim two channels and the sim never blocks on the daemon:
//!
//! - **Inbound desired-set (latest-wins):** a `tokio::sync::watch` carrying the
//!   full desired-set `Vec<EntityDesc>` the projection publishes whenever it
//!   changes. `watch` is latest-wins by construction — N publishes between two
//!   game ticks coalesce into one snapshot the sim reads on the tick. The channel
//!   is seeded with an **empty** `Vec` so a dead/absent daemon (one that never
//!   publishes, or whose sender has been dropped) leaves the sim reading an empty
//!   desired-set rather than stalling — the sim keeps running with nothing to
//!   reconcile.
//! - **Outbound actions:** an `mpsc::Sender<GameAction>` the interaction surface
//!   emits care-verb actions onto, drained by the daemon. Fire-and-forget; if the
//!   daemon is gone the send simply errors and the sim carries on.
//!
//! This module is ONLY the channel seam. It deliberately does not diff the
//! desired-set or spawn/despawn anything — the per-tick `update_agents()` reconcile
//! (box 1.5+) consumes [`SimBridge`] but lives elsewhere.

use marathon_fleet::event::{EntityDesc, GameAction};
use tokio::sync::{mpsc, watch};

/// Default bound for the outbound [`GameAction`] mpsc channel.
///
/// Care actions are sparse (operator-driven); a small bound is ample and keeps a
/// stalled daemon from growing the queue without limit.
pub const OUTBOUND_ACTION_CAPACITY: usize = 64;

/// The sim-side endpoints of the fleet bridge (box 1.4).
///
/// Held by the sim/reconciler. Reads the latest desired-set off [`Self::desired`]
/// each tick and emits outbound [`GameAction`]s on [`Self::actions`]. With no
/// daemon feeding it, `desired.borrow()` yields the seeded empty `Vec`.
pub struct SimBridge {
    /// Latest-wins inbound desired-set. Seeded with an empty `Vec`, so a
    /// dead/absent daemon leaves the sim with an empty desired-set.
    pub desired: watch::Receiver<Vec<EntityDesc>>,
    /// Outbound care-verb actions, drained by the daemon.
    pub actions: mpsc::Sender<GameAction>,
}

/// The daemon-side endpoints of the fleet bridge (box 1.4).
///
/// Held by the out-of-process daemon adapter. Publishes the desired-set on
/// [`Self::publish`] and drains outbound actions off [`Self::actions`]. Dropping
/// this whole struct (a dead/absent daemon) leaves the sim's [`SimBridge`] reading
/// the last value — which, if nothing was ever published, is the seeded empty set.
pub struct DaemonBridge {
    /// Publishes the full desired-set; latest-wins coalesces bursts.
    pub publish: watch::Sender<Vec<EntityDesc>>,
    /// Receives outbound care-verb actions emitted by the sim.
    pub actions: mpsc::Receiver<GameAction>,
}

/// Construct the bridge, returning the [`SimBridge`] (sim side) and
/// [`DaemonBridge`] (daemon side) halves wired to the same channels (box 1.4).
///
/// The inbound desired-set watch is seeded with an **empty** `Vec` so that, until
/// the daemon publishes (or if it never does / has died), the sim reads an empty
/// desired-set and keeps running. Uses [`OUTBOUND_ACTION_CAPACITY`] for the
/// outbound mpsc bound.
pub fn channel() -> (SimBridge, DaemonBridge) {
    channel_with_capacity(OUTBOUND_ACTION_CAPACITY)
}

/// Like [`channel`] but with an explicit outbound mpsc `capacity` (box 1.4).
pub fn channel_with_capacity(capacity: usize) -> (SimBridge, DaemonBridge) {
    // Seed the latest-wins watch with an empty desired-set: a dead/absent daemon
    // yields an empty set, never a stall.
    let (publish, desired) = watch::channel(Vec::<EntityDesc>::new());
    let (action_tx, action_rx) = mpsc::channel::<GameAction>(capacity);

    let sim = SimBridge {
        desired,
        actions: action_tx,
    };
    let daemon = DaemonBridge {
        publish,
        actions: action_rx,
    };
    (sim, daemon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use marathon_fleet::event::{EntityKind, EntityState};
    use std::collections::HashMap;

    fn sample_desc(lane: &str) -> EntityDesc {
        EntityDesc {
            lane_id: lane.to_string(),
            kind: EntityKind::Agent,
            label: "build the thing".to_string(),
            state: EntityState::Active,
            meta: HashMap::new(),
        }
    }

    /// Box 1.4 core: with NO daemon feeding it (dead/absent), the sim reads an
    /// EMPTY desired-set (latest-wins watch seeded with an empty `Vec`), and the
    /// outbound `mpsc::Sender<GameAction>` exists and can be obtained.
    #[test]
    fn test_fleet_bridge_dead_daemon_yields_empty_desired_set() {
        let (sim, daemon) = channel();

        // No publish has happened — the sim's latest desired-set is empty.
        assert!(
            sim.desired.borrow().is_empty(),
            "absent daemon must leave an empty desired-set"
        );

        // Simulate the daemon dying/absent: drop its whole half.
        drop(daemon);

        // The sim keeps running and still reads an empty desired-set.
        assert!(
            sim.desired.borrow().is_empty(),
            "dead daemon must leave an empty desired-set, not a stall"
        );

        // The outbound GameAction sender exists and is obtainable.
        let _action_sender: &mpsc::Sender<GameAction> = &sim.actions;
    }

    /// A publish from the daemon is observed latest-wins by the sim side, and the
    /// sim can emit an outbound `GameAction` the daemon drains. Driven with the
    /// sync `watch`/`try_send`/`try_recv` surface so the test needs no async
    /// runtime (deps stay scoped to `tokio/sync`).
    #[test]
    fn test_fleet_bridge_publish_and_emit_round_trip() {
        let (sim, mut daemon) = channel();

        // Daemon publishes a desired-set; latest-wins coalesces.
        daemon
            .publish
            .send(vec![sample_desc("lane-a"), sample_desc("lane-b")])
            .expect("sim side alive");
        daemon
            .publish
            .send(vec![sample_desc("lane-c")])
            .expect("sim side alive");

        // The sim reads only the most-recent snapshot (latest-wins).
        let latest = sim.desired.borrow().clone();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].lane_id, "lane-c");

        // The sim emits an outbound action; the daemon drains it.
        sim.actions
            .try_send(GameAction::Inspect {
                id: "lane-c".to_string(),
            })
            .expect("daemon side alive, capacity available");
        let got = daemon.actions.try_recv().expect("one action queued");
        assert_eq!(
            got,
            GameAction::Inspect {
                id: "lane-c".to_string()
            }
        );
    }
}
