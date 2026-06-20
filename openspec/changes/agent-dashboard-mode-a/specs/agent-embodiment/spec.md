## ADDED Requirements

### Requirement: Persona-Driven Stable Species

The embodiment layer MUST map a lane's `persona` onto a Marathon species/skin (`EntityKind`), and that species MUST be a stable function of a stable input so the same agent reads as the same creature across its whole life and across renders (CONTRACT §5; design Decision 5). Where a species is selected by hashing, the hash input MUST be a stable identifier (`hash(session_id)` / `laneId`) so the skin never flaps between ticks. Marathon's color=rank convention MUST encode tier, and allied species MUST be reserved for green/healthy/merged states.

#### Scenario: Same persona renders the same species across ticks

- **WHEN** an agent-monster for a given `laneId` is embodied on two separate ticks with the same `persona`
- **THEN** the species/skin selected on both ticks is identical

#### Scenario: A merged PR adopts an allied skin

- **WHEN** an agent-monster's `pr` axis transitions to `merged`
- **THEN** its skin is drawn from the allied species set reserved for healthy/successful states, not a hostile species

### Requirement: Discrete Lifecycle Pose From Work State

The embodiment layer MUST render the lane's `work` state (`spawning | working | idle | blocked | finished`) as a discrete monster pose/animation `frame` — the lifecycle posture — and a change in `work` state MUST drive a discrete pose change (CONTRACT §5; design Decision 5). The pose MUST be selected from `work` state and the animation clock only; it MUST NOT be inferred from continuous overlays such as glow.

#### Scenario: Work-state change drives a discrete pose change

- **WHEN** a monster's `work` axis changes from `working` to `blocked`
- **THEN** the monster's rendered pose changes to a distinct discrete pose for `blocked`

#### Scenario: Unchanged work state holds the pose

- **WHEN** a monster's `EntityDesc` is republished with the same `work` state
- **THEN** the monster retains its current lifecycle pose (only the animation clock advances)

### Requirement: Attention Drives Orientation

The embodiment layer MUST map a monster's attention target onto its orientation/lean (`facing`), so the monster visibly turns toward what its agent is attending to (CONTRACT §3, §5; design Decision 5). Orientation MUST be carried by motion (face-free, motion-first), and the attention channel MUST be optional — when no attention hint is present the monster MUST hold a stable default facing without error.

#### Scenario: Attention target reorients the monster

- **WHEN** a monster receives an attention hint pointing at a different target than its current facing
- **THEN** the monster's `facing` orients/leans toward the new attention target

#### Scenario: Absent attention hint holds a stable facing

- **WHEN** a monster has no attention hint on this tick
- **THEN** the monster retains a stable default facing and no error is raised

### Requirement: Discrete Event And Status Channels

The embodiment layer MUST render the remaining §5 channel bindings as distinct, testable signals: a `box.advanced` event as a discrete completion beat (with `append` rounds rendered as repeated beats); a `test` result as a one-shot damage flash; the `pr` axis as a floating-label quest status; and `hitl.required` as a raise-a-hand pose plus a beacon that also serves as the ack/resurrect surface (CONTRACT §5; design Decision 5, Decision 6). Each of these channels MUST render independently of the others.

#### Scenario: A failed test produces a one-shot damage flash

- **WHEN** a monster's `test` axis reports `failed`
- **THEN** the monster plays a single damage-flash, not a sustained state

#### Scenario: A required HITL gate raises a hand and a beacon

- **WHEN** a monster's `hitl.required` axis becomes set for a gate
- **THEN** the monster adopts a raise-a-hand pose and surfaces a beacon usable as the ack/resurrect surface

#### Scenario: An append round repeats the completion beat

- **WHEN** a `box.advanced` event arrives with `append:true` for a box that already beat
- **THEN** the monster plays a repeated completion beat rather than treating it as a new box

### Requirement: Spatial Placement From Lease Stream

The embodiment layer MUST place a monster spatially from the lease stream rather than from any per-sprite tint: a monster's body MUST live in the room derived from its `changeId`/`lease_key` (room-per-change), a `keyType:path` lease MUST map to a corridor/zone-per-domain, and a lease collision MUST place the blocked monster in a queue at the occupied workbench (CONTRACT §5, §6; design Decision 5). The room/corridor label MUST be derived from the stripped lease attribute, with the `key` treated as stable and opaque for grouping (CONTRACT §6).

#### Scenario: A change-lease places the monster in its room

- **WHEN** a monster holds a `changeId`/`lease_key` for a given change
- **THEN** the monster is placed in the room derived from that change, and the room label is taken from the stripped attribute, not the opaque key

#### Scenario: A lease collision queues the blocked monster

- **WHEN** a `fleet.lease.collision` blocks one lane against a workbench occupied by another
- **THEN** the blocked monster is placed in a queue at the occupied workbench rather than being tinted in place

### Requirement: Monster Is A Faithful Debugger View

A monster MUST read as a faithful debugger view of its agent: a lifecycle-state change MUST drive a discrete pose change and the attention target MUST drive orientation/lean, such that an observer can infer the agent's lifecycle state and current attention from the body alone (CONTRACT §5; design Decision 5). The embodiment layer MUST NOT introduce combat affordances (no weapons that kill); decorative equipment MAY indicate the active tool only.

#### Scenario: Lifecycle and attention are both legible from the body

- **WHEN** an agent transitions lifecycle state and shifts its attention target on the same tick
- **THEN** the monster simultaneously changes to the discrete pose for the new lifecycle state and reorients toward the new attention target

#### Scenario: No combat affordance is rendered

- **WHEN** any monster is embodied in any state
- **THEN** no kill-weapon affordance is rendered; any equipment shown is decorative and indicates the active tool only

### Requirement: Glow Channel Graceful Degradation

The glow/confidence channel SHALL ship DARK/NEUTRAL until the producer's per-lane `progress` classifier lands (CONTRACT §5, §9 graceful-degradation guarantee). EVERY OTHER embodiment channel SHALL render correctly without it: the absence of a `progress` value MUST NOT blank, error, or suppress the species, pose, orientation, event, status, or spatial channels. When the `progress` value is present, all five `progressPhase` enum values MUST be rendered distinctly with no collapse (CONTRACT §5).

#### Scenario: Glow ships dark without a progress value

- **WHEN** an `EntityDesc` carries no resolved `progress` value
- **THEN** the monster's glow channel renders dark/neutral and no error is raised

#### Scenario: Other channels render fully while glow is dark

- **WHEN** glow is dark because no `progress` value is present, and the same `EntityDesc` carries `persona`, `work`, `pr`, `test`, and a lease
- **THEN** the species, lifecycle pose, quest-label, damage-flash, and spatial placement all render correctly and unaffected by the missing glow

#### Scenario: All five progress phases render distinctly

- **WHEN** five monsters each carry a different `progressPhase` value (`productive`, `plateau`, `regression-suspected`, `noise-amplification`, `exhausted`)
- **THEN** each renders a visually distinct glow/saturation/flicker with no two phases collapsed to the same appearance
