## ADDED Requirements

### Requirement: One Render Loop With A Switchable View Clock

The capability SHALL drive all rendering through a single render loop whose `render_time` is supplied by one `view_clock` abstraction with two selectable modes (per design.md Decision 7 and CONTRACT §7). In **live** mode the `view_clock` SHALL evaluate `render_time = now − INTERP_DELAY` (the stream head, held a bounded interval behind real time). In **replay** mode the `view_clock` SHALL evaluate `render_time = scrub_T`, the operator-chosen scrub position. Switching modes MUST change only the clock source; the render loop, the per-entity interpolation buffers, and the embodiment mapping MUST be identical across modes.

#### Scenario: Live mode renders behind the stream head

- **WHEN** the `view_clock` is in live mode at wall-clock instant `now`
- **THEN** the render loop receives `render_time = now − INTERP_DELAY`

#### Scenario: Replay mode renders at the scrub position

- **WHEN** the `view_clock` is in replay mode with scrub position `scrub_T`
- **THEN** the render loop receives `render_time = scrub_T` and ignores wall-clock `now`

#### Scenario: Switching modes reuses the same render path

- **WHEN** the `view_clock` is switched from live to replay and back to live
- **THEN** the render loop, interpolation buffers, and embodiment mapping used are the same in both modes, and only the source of `render_time` differs

### Requirement: Render Slightly In The Past Over A Keyframe Buffer

The capability SHALL render slightly in the past by holding `render_time` behind the newest received event by `INTERP_DELAY`, so that a jitter/keyframe buffer always contains keyframes straddling `render_time` (per design.md Decision 7 and CONTRACT §4). The projection SHALL emit one keyframe `{event_time, target}` per entity on each state change into that entity's interpolation buffer. The render loop SHALL select the two keyframes straddling `render_time` for each entity. A burst of arrivals MUST be absorbed by the buffer rather than displayed instantaneously.

#### Scenario: Straddling keyframes are selected for the render instant

- **WHEN** an entity's buffer holds keyframes at `t0` and `t1` with `t0 ≤ render_time ≤ t1`
- **THEN** the render loop selects `t0` and `t1` as the pair to interpolate between for that entity

#### Scenario: Arrival burst is absorbed by the delay buffer

- **WHEN** several keyframes for one entity arrive together within a span shorter than `INTERP_DELAY`
- **THEN** the render loop continues to advance `render_time` by `INTERP_DELAY` behind the newest arrival and plays the keyframes in `event_time` order rather than displaying the burst on a single frame

### Requirement: Smooth Bounded-Travel-Time Interpolation Without Hard Snaps

The capability SHALL convert bursty, irregularly spaced per-tick deltas (seconds-to-minutes apart) into smooth 60 fps motion by interpolating each entity between its straddling keyframes (per design.md Decision 7 and CONTRACT §4). For irregular spacing the capability SHALL use a bounded travel-time tween (move toward the target over a fixed window of roughly 0.5–2 s) with idle animation filling the gaps, rather than a wall-clock lerp that crawls across long dead time. Interpolated quantities MUST be flat scalars (such as x, y, angle) with no nested fields in the hot buffer. The capability MUST NOT hard-snap an entity to a new target; on a new keyframe it SHALL converge smoothly toward it.

#### Scenario: Bounded travel-time tween over a long gap

- **WHEN** an entity receives a new target keyframe after a multi-minute idle gap
- **THEN** the entity tweens to the target over the bounded travel window and plays idle animation for the remainder of the gap, rather than crawling proportionally to the full wall-clock gap

#### Scenario: New keyframe converges without a hard snap

- **WHEN** a new keyframe arrives for an entity that is mid-tween toward a prior target
- **THEN** the entity converges smoothly onto the new target and never instantaneously jumps its position or angle

#### Scenario: Hot buffer carries only flat scalars

- **WHEN** an entity's keyframe is enqueued into the interpolation buffer
- **THEN** the buffered interpolated quantities are flat scalars (such as x, y, angle) with no nested structured fields

### Requirement: Scrub-To-T Equals A Live Run That Reached T

The capability SHALL guarantee that scrubbing to an arbitrary time `T` produces the same world state as a live run that played forward and reached `T`, because both are the same pure deterministic fold of the event prefix (per design.md Decision 3/7 and CONTRACT §7). The replay clock SHALL resolve `scrub_T` against the nearest file-resident snapshot anchor whose `asOf ≤ T`, then fold the replay tail (events whose `seq > anchor.lastSeq` and whose `time ≤ T`). Choosing a different valid anchor `≤ T` MUST NOT change the resolved state.

#### Scenario: Scrub matches a forward live run at the same T

- **WHEN** the operator scrubs the `view_clock` to time `T`
- **THEN** the resolved world state is identical to the state a live run produces when it has played forward and reached `T`

#### Scenario: Nearest anchor plus replay tail

- **WHEN** scrubbing to `T` with the nearest snapshot anchor having `asOf ≤ T` and `lastSeq = L`
- **THEN** the capability folds that anchor and then applies exactly the events whose `seq > L` and whose `time ≤ T`

#### Scenario: Anchor choice does not change the result

- **WHEN** the same `T` is resolved twice, once from one valid anchor `asOf ≤ T` and once from an earlier valid anchor `asOf ≤ T`
- **THEN** both resolutions yield an identical world state

### Requirement: Consume render_snapshot From decouple-tick-snapshot

The capability SHALL consume the serializable `render_snapshot() -> WorldSnapshot` produced by the merged `decouple-tick-snapshot` change as a hard dependency, and SHALL NOT re-specify the tick/snapshot decoupling (per proposal.md Impact, design.md Context, and CONTRACT §9). The render loop SHALL read entity geometry and render state from the consumed `WorldSnapshot` for both live and replay modes; the capability MUST add only the `view_clock` and interpolation layer on top of that seam.

#### Scenario: Render reads from the consumed WorldSnapshot

- **WHEN** the render loop renders a frame in either live or replay mode
- **THEN** it reads the per-entity render state from a `WorldSnapshot` returned by `render_snapshot()` rather than from any re-defined snapshot type

#### Scenario: Capability adds only the clock and interpolation layer

- **WHEN** the capability is implemented on top of `decouple-tick-snapshot`
- **THEN** it introduces only the `view_clock` and the interpolation buffer behavior, and it does not redefine `tick`, `WorldSnapshot`, or the headless tick loop
