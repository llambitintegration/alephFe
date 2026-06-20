## ADDED Requirements

### Requirement: Latest-Wins Desired-Set Channel

The reconciler MUST consume a desired-set of `EntityDesc` published on a latest-wins channel, where each newly published snapshot fully supersedes the previous one. The reconciler MUST read at most the most recent snapshot per game tick, coalescing any number of intervening external changes into a single diff, and MUST tolerate an empty or stale desired-set without error (CONTRACT §4; design Decision 4). The reconcile key MUST be the opaque `laneId` carried on each `EntityDesc`, never a label or attribute (CONTRACT §3).

#### Scenario: Multiple publishes between ticks coalesce to one diff

- **WHEN** three distinct desired-set snapshots are published on the channel between two consecutive game ticks
- **THEN** the reconciler reads only the most recent snapshot on the next tick and produces exactly one diff against the live monster set, ignoring the two superseded snapshots

#### Scenario: Empty desired-set is tolerated

- **WHEN** the channel currently holds an empty desired-set (no agents)
- **THEN** the reconciler completes the tick without error and despawns or sweeps any live agent-monsters per its normal diff rules

#### Scenario: Reconcile is keyed on laneId not label

- **WHEN** an `EntityDesc` for an existing `laneId` is republished with a changed `label` and a changed `persona`-derived attribute
- **THEN** the reconciler matches it to the same live monster by `laneId` and does not treat it as a new entity

### Requirement: Per-Tick Diff — Spawn, Update-In-Place, Smooth-Despawn

On each game tick the reconciler MUST diff the latest desired-set against the live agent-monster set and act by category: spawn a monster for each desired entity with no live counterpart; update an existing monster in place when its `EntityDesc` state changed; and smooth-despawn (death/leave animation, not an instant removal) a live monster whose entity is absent from the desired-set. The reconciler MUST NOT despawn-then-respawn a monster on a mere state change, because that flickers (design Decision 4; CONTRACT §3).

#### Scenario: Newcomer is spawned

- **WHEN** the desired-set contains a `laneId` that has no corresponding live monster
- **THEN** the reconciler spawns a new monster for that `laneId` on this tick

#### Scenario: State change updates in place without respawn

- **WHEN** a live monster's `EntityDesc` is republished with a changed `work` state but the same `laneId`
- **THEN** the reconciler updates the existing monster in place and does not despawn or respawn it

#### Scenario: Vanished agent is smooth-despawned

- **WHEN** a `laneId` that had a live monster is absent from the newest desired-set
- **THEN** the reconciler triggers a smooth despawn (leave/death animation) for that monster rather than an instantaneous removal

### Requirement: Per-Tick Spawn Cap

The reconciler MUST cap the number of monsters spawned on any single game tick at 8. When the diff calls for more than 8 spawns, the reconciler MUST spawn 8 this tick and queue the remainder to subsequent ticks, so that a 0→N spawn storm never stalls a single frame (CONTRACT §4; design Decision 4).

#### Scenario: Spawn storm is rate-limited

- **WHEN** the desired-set jumps from 0 to 20 agents in one publish and the reconciler diffs it on the next tick
- **THEN** the reconciler spawns exactly 8 monsters this tick and defers the remaining 12 to following ticks

#### Scenario: Overflow drains across subsequent ticks

- **WHEN** 12 spawns remain queued after a capped tick and the desired-set does not change further
- **THEN** subsequent ticks spawn up to 8 per tick until all 20 desired monsters are live, with no further desired-set publish required

#### Scenario: Below-cap diff spawns all immediately

- **WHEN** the diff calls for 5 spawns on a tick
- **THEN** the reconciler spawns all 5 on that tick and queues nothing

### Requirement: Stable Slot Assignment Per Lane

The reconciler MUST assign a stable slot to each `laneId`, held 1:1 for the monster's life, so that an agent which flaps (disappears then reappears) is rendered in the same slot it previously occupied (CONTRACT §1, §3; design Decision 4). The slot MUST be derived from the `laneId` and MUST NOT be reassigned to a different `laneId` while the original monster is live.

#### Scenario: Flapping agent reappears in the same slot

- **WHEN** a `laneId` despawns and the same `laneId` reappears in a later desired-set
- **THEN** the reconciler reassigns it the same stable slot it held before, so it reappears in place

#### Scenario: Distinct lanes get distinct slots

- **WHEN** two different `laneId`s are present in the same desired-set
- **THEN** the reconciler assigns each its own slot and never collides them onto the same slot while both are live

### Requirement: Grace-Debounced Despawn

The reconciler MUST debounce despawns with a short grace timer: when a `laneId` first goes absent from the desired-set, the reconciler MUST start a grace timer rather than despawning immediately, and MUST cancel the pending despawn if the same `laneId` reappears before the timer elapses (design Decision 4; CONTRACT §4 — flapping/append churn). Only after the grace window elapses without reappearance does the smooth despawn proceed.

#### Scenario: Brief absence does not despawn

- **WHEN** a `laneId` is absent from one tick's desired-set but reappears within the grace window
- **THEN** the reconciler cancels the pending despawn and keeps the monster live in its existing slot

#### Scenario: Sustained absence despawns after grace window

- **WHEN** a `laneId` remains absent from the desired-set for the full grace window
- **THEN** the reconciler proceeds with the smooth despawn after the window elapses

### Requirement: m_del Operator-Retire vs Self-Departure Split

The reconciler MUST distinguish an operator-acted retirement from a self-departure, replicating the `m_del_from_pid_list` disambiguation (CONTRACT §3, §5; design Decision 6). It MUST emit a `kill` `GameAction` ONLY when the operator deliberately retires or sends home a monster that is still present in the last desired-set snapshot. A lane that departed on its own — signalled by a `final` flag and/or bare absence from the desired-set — MUST be swept silently with a smooth despawn and NO `kill` callback.

#### Scenario: Operator retire of a still-present monster emits kill

- **WHEN** the operator issues a retire/send-home care verb against a monster whose `laneId` is still present in the last desired-set snapshot
- **THEN** the reconciler emits a `kill` `GameAction` targeting that `laneId`

#### Scenario: Self-departed lane is swept with no callback

- **WHEN** a lane departs on its own (emits `final:true` and/or vanishes from the desired-set) without any operator care verb
- **THEN** the reconciler smooth-despawns the monster and emits no `kill` `GameAction`

#### Scenario: Absence alone never emits kill

- **WHEN** a `laneId` is absent from the newest desired-set and no operator retire/send-home was issued for it
- **THEN** the reconciler treats it as self-departure, sweeps it silently, and emits no `GameAction`
