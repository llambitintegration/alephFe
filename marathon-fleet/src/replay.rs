//! Replay stage: the switchable view-clock live/replay render layer.
//!
//! Drives one render loop whose `render_time` comes from a single `view_clock`
//! with two modes — live (`now - INTERP_DELAY`) and replay (`scrub_T`) — over a
//! per-entity keyframe interpolation buffer, consuming geometry from
//! `decouple-tick-snapshot`'s `render_snapshot()`. Stub — no behavior yet.
