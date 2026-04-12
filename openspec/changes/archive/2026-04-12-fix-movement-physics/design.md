## Context

The simulation runs at 30 ticks/second matching original Marathon. Player physics are governed by `PhysicsConstants` parsed from Marathon data files as 16.16 fixed-point values. The current implementation has four issues causing sluggish controls:

1. **Walking-only physics**: `world.rs` loads `physics[0]` (walking). Marathon has two sets: walking (index 0) and running (index 1), with running having ~2x velocity/acceleration. Players expect to move at running speed by default.

2. **Vec2 velocity model**: Our `compute_player_velocity()` uses a single `Vec2` for XY velocity with isotropic deceleration. Aleph One tracks `forward_velocity` and `perpendicular_velocity` as independent scalars, decelerating each axis independently and applying acceleration+deceleration simultaneously during direction reversals.

3. **Angular unit mismatch**: PhysicsConstants angular fields (`angular_acceleration`, `angular_deceleration`, `maximum_angular_velocity`, `maximum_elevation`, etc.) are stored in Marathon angle units (512 = full circle). After `fixed_to_f32`, the values are in Marathon angle fractions. Our sim treats them as radians (2π = full circle). The conversion factor `2π/512 ≈ 0.01227` is never applied.

4. **Mouse input latency**: Mouse deltas accumulate between render frames, get applied at the next simulation tick, then the camera interpolates toward the new state. This adds ~33ms of perceived lag. Additionally, `marathon-game` (native) still converts mouse deltas to binary TURN_LEFT/TURN_RIGHT flags, discarding magnitude entirely.

## Goals / Non-Goals

**Goals:**
- Player movement that matches the feel and responsiveness of Aleph One
- Correct unit conversion for all physics constants
- Running speed by default (matching Aleph One's default behavior)
- Reduced mouse-to-screen latency
- Axis-decomposed velocity matching Marathon's physics model

**Non-Goals:**
- Walk/run toggle key (can default to running for now; toggle is future work)
- Mouse acceleration curves (Aleph One supports this but it's configurable and optional)
- Configurable sensitivity UI (hardcoded constant is acceptable)
- Changing the 30Hz simulation tick rate
- Gamepad input handling

## Decisions

### Decision 1: Default to running physics, skip walk/run toggle

**Choice:** Load `physics[1]` (running) when available, fall back to `physics[0]` (walking). Do not implement a walk/run toggle key.

**Alternative considered:** Implement the full walk/run toggle with key binding — rejected as scope creep. The vast majority of Marathon gameplay is done at running speed. Walk mode is rarely used and can be added later.

**Rationale:** In Aleph One, the run key defaults to "always on" in modern configurations. Loading index 1 immediately doubles movement responsiveness with a one-line change.

### Decision 2: Axis-decomposed velocity model

**Choice:** Rewrite `compute_player_velocity()` to track forward and perpendicular velocity as separate f32 scalars, matching Aleph One's `physics.cpp` approach. Each axis decelerates independently. When input opposes current velocity direction, apply `acceleration + deceleration` combined for snappier reversals.

**Alternative considered:** Keep Vec2 model but decompose at deceleration time — rejected because the decomposition is the natural state for Marathon's physics and converting back and forth adds complexity.

**Rationale:** Marathon's physics model was designed with axis decomposition in mind. The separate scalars mean stopping a strafe doesn't affect forward speed, and direction reversals feel immediate. The velocity is recomposed into Vec2/Vec3 only when applying to position.

### Decision 3: Convert angular units at load time in `from_physics_constants()`

**Choice:** Apply the conversion factor `value * (2π / 512)` to all angular fields in `PlayerPhysicsParams::from_physics_constants()`. This includes: `angular_acceleration`, `angular_deceleration`, `max_angular_velocity`, `maximum_elevation`, `angular_recentering_velocity`, `fast_angular_velocity`, `fast_angular_maximum`, `external_angular_deceleration`.

**Alternative considered:** Convert at use sites (in `compute_facing`, `compute_vertical_look`) — rejected because it's error-prone; every new use site would need to remember the conversion.

**Rationale:** Converting once at the boundary (data loading) keeps the sim internals consistently in radians. All downstream code can assume radian units without per-call conversion.

### Decision 4: Camera-immediate mouse application for reduced latency

**Choice:** In the render loop, apply mouse delta directly to the camera's yaw/pitch for the current frame's rendering, before the next sim tick processes it. The sim still receives the same delta and applies it authoritatively on the next tick. On tick completion, the camera snaps to the sim's authoritative state.

**Alternative considered:** Run sim ticks at higher frequency (60Hz or variable) — rejected because it changes game behavior and diverges from Marathon's 30Hz design. Another alternative: don't interpolate camera rotation — rejected because it causes visual jitter at low frame rates.

**Rationale:** This is the standard approach used by modern source ports (including Aleph One's SDL mouse code). The camera preview is cosmetic — the sim remains authoritative. At 60fps with 30Hz ticks, this eliminates one frame (~16ms) of perceived mouse lag.

## Risks / Trade-offs

- [Velocity model change] Rewriting the core velocity computation touches all movement tests and could introduce regression in collision behavior → Mitigation: existing collision tests remain valid (they test position/velocity output); update velocity computation tests to use axis-decomposed API.
- [Camera desync] Camera-immediate mouse creates a brief mismatch between rendered camera angle and sim state → Mitigation: The mismatch is at most one tick (33ms) and gets corrected on every tick. This is standard practice in interpolated game engines.
- [Angular conversion precision] Float conversion of angle units could introduce rounding errors vs. original fixed-point math → Mitigation: Marathon's angle precision is limited to 9 bits (512 values); f32 has more than enough precision. This is not a concern in practice.
- [Physics index out of bounds] Some physics data files may have only one entry (no running physics) → Mitigation: Fall back to index 0 when index 1 is not available. Already handled by `.get(1).or_else(|| p.first())` pattern.
