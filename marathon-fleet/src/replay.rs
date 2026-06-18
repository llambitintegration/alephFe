//! Replay stage: the switchable view-clock live/replay render layer.
//!
//! Drives one render loop whose `render_time` comes from a single [`ViewClock`]
//! abstraction with two selectable modes — [`ViewClock::Live`]
//! (`now - INTERP_DELAY`) and [`ViewClock::Replay`] (`scrub_T`) — over a
//! per-entity [`InterpBuffer`] of flat-scalar keyframes.
//!
//! Everything here is PURE: there is no wall clock, no RNG, and no I/O. Both
//! "now" and the scrub position are explicit `f64` second values the caller
//! threads in, exactly like the explicit-tick / explicit-hash convention used by
//! `projection.rs`, `embodiment.rs`, and `reconciler.rs` in this same crate.
//!
//! What lives here (boxes 7.1–7.5):
//! - **Switchable view clock (7.1):** [`ViewClock`] resolves a single
//!   [`ViewClock::render_time`] used by both modes; the render path downstream is
//!   identical, only the clock source differs.
//! - **Live mode (7.2):** [`ViewClock::Live`] renders `now - INTERP_DELAY`, i.e.
//!   a bounded interval behind the live stream head.
//! - **Replay mode (7.3):** [`ViewClock::Replay`] renders the frozen `scrub_T`,
//!   ignoring `now` entirely.
//! - **Keyframe buffer (7.4):** [`InterpBuffer`] absorbs a burst of per-entity
//!   [`Keyframe`]s (one `{event_time, target}` per state change), orders them by
//!   `event_time`, and [`InterpBuffer::straddle`] selects the two keyframes
//!   bracketing `render_time`.
//! - **Bounded tween (7.5):** [`InterpBuffer::sample`] interpolates between the
//!   straddling keyframes over a bounded travel-time window
//!   ([`MIN_TRAVEL`]..=[`MAX_TRAVEL`]) and idles in the gaps, converging smoothly
//!   on a new keyframe — never a hard snap. Buffered quantities are FLAT scalars
//!   (`x`, `y`, `angle`); there are no nested fields.

/// How far behind the live stream head [`ViewClock::Live`] renders, in seconds.
///
/// Live render time is `now - INTERP_DELAY`. Holding the head a bounded interval
/// in the past gives the per-entity [`InterpBuffer`] a window in which a burst of
/// keyframes can settle and be played in `event_time` order (box 7.2 / 7.4).
pub const INTERP_DELAY: f64 = 0.1;

/// Lower bound of the bounded travel-time tween window, in seconds (box 7.5).
pub const MIN_TRAVEL: f64 = 0.5;

/// Upper bound of the bounded travel-time tween window, in seconds (box 7.5).
///
/// Even if two straddling keyframes are spaced further apart than this, the tween
/// completes within [`MAX_TRAVEL`] of the earlier keyframe and then idles, so the
/// body never lurches across a long gap in a single frame.
pub const MAX_TRAVEL: f64 = 2.0;

/// Which clock source feeds the single render loop (box 7.1).
///
/// Both variants resolve through one [`ViewClock::render_time`] method, so the
/// downstream render path is byte-for-byte identical regardless of mode —
/// switching modes changes ONLY the clock source.
///
/// A leaf, fully-scalar enum: `Copy`/`Eq` are intentionally *not* derived because
/// it carries an `f64` (`Replay`'s scrub position), which is not `Eq`. It stays
/// `Clone`/`PartialEq` to match the value semantics of the other shared types.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewClock {
    /// Live mode: render time tracks real time, held [`INTERP_DELAY`] behind the
    /// stream head (box 7.2).
    Live,
    /// Replay mode: render time is frozen at the scrub position `scrub_t`,
    /// ignoring wall-clock `now` (box 7.3).
    Replay {
        /// The scrub position, in seconds, that replay renders at.
        scrub_t: f64,
    },
}

impl ViewClock {
    /// Resolve the `render_time` (seconds) that drives the one render loop.
    ///
    /// This is the single resolver both modes flow through (box 7.1):
    /// - [`ViewClock::Live`] → `now - INTERP_DELAY` (box 7.2).
    /// - [`ViewClock::Replay`] → `scrub_t`, ignoring `now` (box 7.3).
    pub fn render_time(&self, now: f64) -> f64 {
        match self {
            ViewClock::Live => now - INTERP_DELAY,
            ViewClock::Replay { scrub_t } => *scrub_t,
        }
    }
}

/// One buffered keyframe: a target pose stamped with the `event_time` (seconds)
/// at which the producing state change occurred (box 7.4).
///
/// Buffered quantities are FLAT scalars (`x`, `y`, `angle`) — there are no nested
/// fields, so straddle selection and the tween operate component-wise (box 7.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
    /// Wall-event time of the state change that produced this keyframe (seconds).
    pub event_time: f64,
    /// Target world-X for this keyframe.
    pub x: f64,
    /// Target world-Y for this keyframe.
    pub y: f64,
    /// Target facing angle for this keyframe (radians).
    pub angle: f64,
}

impl Keyframe {
    /// Construct a keyframe from its flat scalar components.
    pub fn new(event_time: f64, x: f64, y: f64, angle: f64) -> Self {
        Keyframe {
            event_time,
            x,
            y,
            angle,
        }
    }

    /// Linearly blend two keyframes' flat scalars at parameter `t` in `0..=1`.
    fn lerp(&self, other: &Keyframe, t: f64) -> Pose {
        Pose {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            angle: self.angle + (other.angle - self.angle) * t,
        }
    }

    /// The frozen pose of this keyframe (used when idling on it).
    fn pose(&self) -> Pose {
        Pose {
            x: self.x,
            y: self.y,
            angle: self.angle,
        }
    }
}

/// The sampled, render-ready pose: flat scalars only (box 7.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Sampled world-X.
    pub x: f64,
    /// Sampled world-Y.
    pub y: f64,
    /// Sampled facing angle (radians).
    pub angle: f64,
}

/// A pair of keyframes bracketing a `render_time`, as selected by
/// [`InterpBuffer::straddle`] (box 7.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Straddle {
    /// `render_time` precedes every buffered keyframe: hold the earliest.
    BeforeAll(Keyframe),
    /// `render_time` is bracketed by two keyframes `prev <= render_time < next`.
    Between {
        /// The keyframe at or before `render_time`.
        prev: Keyframe,
        /// The next keyframe after `render_time`.
        next: Keyframe,
    },
    /// `render_time` is at/after every buffered keyframe: hold the latest.
    AfterAll(Keyframe),
}

/// A per-entity interpolation buffer of flat-scalar keyframes (boxes 7.4 / 7.5).
///
/// [`InterpBuffer::push`] absorbs keyframes in any arrival order (a burst); the
/// buffer keeps them sorted by `event_time` so playback is always in
/// `event_time` order. [`InterpBuffer::straddle`] selects the bracketing pair and
/// [`InterpBuffer::sample`] tweens between them over a bounded travel window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InterpBuffer {
    keyframes: Vec<Keyframe>,
}

impl InterpBuffer {
    /// An empty buffer.
    pub fn new() -> Self {
        InterpBuffer {
            keyframes: Vec::new(),
        }
    }

    /// Absorb one keyframe, keeping the buffer ordered by `event_time` (box 7.4).
    ///
    /// A burst pushed in any order ends up played in `event_time` order: each
    /// keyframe is inserted at its sorted position by `event_time`. Insertion is
    /// stable for equal `event_time`s (the later arrival sorts after).
    pub fn push(&mut self, kf: Keyframe) {
        // `partition_point` finds the first index whose event_time is strictly
        // greater than the incoming one, so equal-time keyframes preserve arrival
        // order (stable) and the vec stays sorted ascending by event_time.
        let idx = self
            .keyframes
            .partition_point(|existing| existing.event_time <= kf.event_time);
        self.keyframes.insert(idx, kf);
    }

    /// The buffered keyframes, in `event_time` order (box 7.4).
    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }

    /// Select the two keyframes straddling `render_time` (box 7.4).
    ///
    /// Returns `None` only when the buffer is empty. Otherwise it returns the
    /// bracketing pair, or a clamp ([`Straddle::BeforeAll`] /
    /// [`Straddle::AfterAll`]) when `render_time` falls outside the buffered span.
    pub fn straddle(&self, render_time: f64) -> Option<Straddle> {
        if self.keyframes.is_empty() {
            return None;
        }
        let first = self.keyframes[0];
        if render_time < first.event_time {
            return Some(Straddle::BeforeAll(first));
        }
        // `prev` = last keyframe with event_time <= render_time.
        let prev_idx = self
            .keyframes
            .partition_point(|kf| kf.event_time <= render_time)
            - 1;
        match self.keyframes.get(prev_idx + 1) {
            Some(next) => Some(Straddle::Between {
                prev: self.keyframes[prev_idx],
                next: *next,
            }),
            None => Some(Straddle::AfterAll(self.keyframes[prev_idx])),
        }
    }

    /// Sample the interpolated pose at `render_time` (box 7.5).
    ///
    /// Between two straddling keyframes the body tweens from `prev` toward `next`
    /// over a bounded travel window: the effective travel time is
    /// `clamp(next.event_time - prev.event_time, MIN_TRAVEL, MAX_TRAVEL)`. The
    /// tween parameter is `(render_time - prev.event_time) / travel`, clamped to
    /// `0..=1`, so once travel completes the body idles on `next` (no hard snap,
    /// and no lurch across a gap longer than [`MAX_TRAVEL`]). Outside the buffered
    /// span the latest/earliest keyframe pose is held.
    pub fn sample(&self, render_time: f64) -> Option<Pose> {
        match self.straddle(render_time)? {
            Straddle::BeforeAll(kf) | Straddle::AfterAll(kf) => Some(kf.pose()),
            Straddle::Between { prev, next } => {
                let gap = next.event_time - prev.event_time;
                let travel = gap.clamp(MIN_TRAVEL, MAX_TRAVEL);
                // `travel` is >= MIN_TRAVEL > 0, so the division is always safe.
                let t = ((render_time - prev.event_time) / travel).clamp(0.0, 1.0);
                Some(prev.lerp(&next, t))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Box 7.1: switching modes reuses the same render path. Both modes resolve
    // through the ONE `render_time` resolver; the downstream render is a pure
    // function of that resolved time, so we prove that feeding either mode's
    // resolved time through the same render path yields identical output.
    fn render_path(render_time: f64, buf: &InterpBuffer) -> Option<Pose> {
        // The single render path: it only sees `render_time`, never the mode.
        buf.sample(render_time)
    }

    fn sample_buffer() -> InterpBuffer {
        let mut buf = InterpBuffer::new();
        buf.push(Keyframe::new(0.0, 0.0, 0.0, 0.0));
        buf.push(Keyframe::new(1.0, 10.0, 20.0, 1.0));
        buf
    }

    #[test]
    fn switching_modes_reuses_the_same_render_path() {
        let buf = sample_buffer();
        // Construct a live and a replay clock that resolve to the SAME time.
        let now = 5.6;
        let live = ViewClock::Live;
        let live_t = live.render_time(now); // 5.6 - 0.1 = 5.5
        let replay = ViewClock::Replay { scrub_t: live_t };
        let replay_t = replay.render_time(now); // 5.5, ignoring now

        assert_eq!(live_t, replay_t, "the two modes resolve to the same time");
        // Same render path, same resolved time => byte-identical render output.
        assert_eq!(
            render_path(live_t, &buf),
            render_path(replay_t, &buf),
            "switching modes changes only the clock source, not the render path"
        );
    }

    // Box 7.2: live mode renders behind the stream head.
    #[test]
    fn live_mode_renders_behind_the_stream_head() {
        let now = 12.0;
        let rt = ViewClock::Live.render_time(now);
        assert_eq!(rt, now - INTERP_DELAY);
        assert!(rt < now, "live render time lags the stream head");
        assert!(
            INTERP_DELAY > 0.0,
            "the delay is a bounded positive interval"
        );
    }

    // Box 7.3: replay mode renders at the scrub position regardless of `now`.
    #[test]
    fn replay_mode_renders_at_scrub_position_regardless_of_now() {
        let clock = ViewClock::Replay { scrub_t: 3.25 };
        assert_eq!(clock.render_time(0.0), 3.25);
        assert_eq!(clock.render_time(1_000_000.0), 3.25);
        assert_eq!(
            clock.render_time(-42.0),
            3.25,
            "replay ignores wall-clock now entirely"
        );
    }

    // Box 7.4: a burst is absorbed and played in event_time order; straddle picks
    // the two keyframes bracketing render_time.
    #[test]
    fn burst_is_ordered_and_straddle_selects_bracketing_pair() {
        let mut buf = InterpBuffer::new();
        // A burst pushed OUT of event_time order.
        buf.push(Keyframe::new(2.0, 2.0, 0.0, 0.0));
        buf.push(Keyframe::new(0.0, 0.0, 0.0, 0.0));
        buf.push(Keyframe::new(3.0, 3.0, 0.0, 0.0));
        buf.push(Keyframe::new(1.0, 1.0, 0.0, 0.0));

        // The buffer absorbed the burst and ordered it by event_time.
        let times: Vec<f64> = buf.keyframes().iter().map(|k| k.event_time).collect();
        assert_eq!(
            times,
            vec![0.0, 1.0, 2.0, 3.0],
            "burst played in event order"
        );

        // Straddle at 1.5 brackets keyframes at 1.0 and 2.0.
        match buf.straddle(1.5).expect("non-empty") {
            Straddle::Between { prev, next } => {
                assert_eq!(prev.event_time, 1.0);
                assert_eq!(next.event_time, 2.0);
            }
            other => panic!("expected a bracketing pair, got {other:?}"),
        }

        // Clamp behavior outside the span.
        assert!(matches!(buf.straddle(-1.0), Some(Straddle::BeforeAll(_))));
        assert!(matches!(buf.straddle(9.0), Some(Straddle::AfterAll(_))));
        assert_eq!(InterpBuffer::new().straddle(0.0), None, "empty buffer");
    }

    // Box 7.5: bounded tween produces intermediate values and never hard-snaps.
    #[test]
    fn bounded_tween_produces_intermediate_values_without_hard_snap() {
        // Two keyframes spaced 1.0s apart (inside the MIN..MAX travel window).
        let mut buf = InterpBuffer::new();
        buf.push(Keyframe::new(0.0, 0.0, 0.0, 0.0));
        buf.push(Keyframe::new(1.0, 10.0, 100.0, 2.0));

        // At the start: exactly prev.
        let start = buf.sample(0.0).unwrap();
        assert_eq!(
            start,
            Pose {
                x: 0.0,
                y: 0.0,
                angle: 0.0
            }
        );

        // Midway through the 1.0s travel: a strict intermediate (no hard snap).
        let mid = buf.sample(0.5).unwrap();
        assert!(mid.x > 0.0 && mid.x < 10.0, "x is intermediate: {}", mid.x);
        assert!(mid.y > 0.0 && mid.y < 100.0, "y is intermediate: {}", mid.y);
        assert!(
            mid.angle > 0.0 && mid.angle < 2.0,
            "angle is intermediate: {}",
            mid.angle
        );
        // Linear midpoint for a 1.0s gap clamped to a 1.0s travel.
        assert!((mid.x - 5.0).abs() < 1e-9);

        // After travel completes: converged on next, then idles (no overshoot).
        let done = buf.sample(1.0).unwrap();
        assert_eq!(
            done,
            Pose {
                x: 10.0,
                y: 100.0,
                angle: 2.0
            }
        );
        let idle = buf.sample(5.0).unwrap();
        assert_eq!(idle, done, "idles on the latest keyframe past the span");
    }

    // Box 7.5 (bound): a gap longer than MAX_TRAVEL still completes within
    // MAX_TRAVEL and then idles — no lurch across the whole gap in one frame.
    #[test]
    fn travel_is_bounded_above_by_max_travel() {
        let mut buf = InterpBuffer::new();
        buf.push(Keyframe::new(0.0, 0.0, 0.0, 0.0));
        buf.push(Keyframe::new(10.0, 100.0, 0.0, 0.0)); // 10s gap >> MAX_TRAVEL

        // At render_time == MAX_TRAVEL the tween has already converged on next.
        let at_bound = buf.sample(MAX_TRAVEL).unwrap();
        assert_eq!(at_bound.x, 100.0, "tween completes within MAX_TRAVEL");

        // Before the bound it is a strict intermediate (smooth, not snapped).
        let before = buf.sample(MAX_TRAVEL / 2.0).unwrap();
        assert!(before.x > 0.0 && before.x < 100.0);
    }
}
