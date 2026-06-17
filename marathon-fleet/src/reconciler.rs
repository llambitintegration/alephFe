//! Reconciler stage: the per-tick desired-set diff.
//!
//! Consumes the desired-set on a latest-wins channel and, keyed strictly on the
//! opaque `laneId`, spawns newcomers, updates existing agents in place, and
//! smooth-despawns vanished ones under a per-tick spawn cap. Stub — no behavior yet.
