//! Projection stage: the pure deterministic event-sourced reducer.
//!
//! Hosts `apply(state, event) -> state` with no clock, no RNG, and no I/O, plus
//! snapshot-anchor reconstruction and restart-by-replay recovery. The same
//! ordered log must fold to a byte-identical `WorldState`. Stub — no behavior yet.
