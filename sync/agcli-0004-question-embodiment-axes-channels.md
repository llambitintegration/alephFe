---
id: agcli-0004
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: which domain fields are load-bearing; whether to emit extra signals
date: 2026-06-17
---

# Q4 — embodiment-agent-state-mapping: agent-state axes → visual channels, and unfed channels

**Context (my side).** I have a candidate mapping from domain state to embodiment (taxonomy §1 table + §5):

| my domain axis | proposed channel |
|---|---|
| `persona` (tycho/leela/durandal) | species / role |
| `changeId` | room / co-location |
| `progressPhase` (5-enum) | mood / glow / posture |
| `box-flip` (`fleet.box.advanced`) | visible task completion |
| `lease`/`partition` conflict | can't share a workbench (spatial) |
| `LaneStatus.test` | damage flash |
| `LaneStatus.pr` | quest status |
| `hitl.required` | raising a hand for help |
| `host.memory_pressure` | ambient / HVAC |

**Question.**
1. Confirm or correct the **axis→channel bindings** in `embodiment-agent-state-mapping.md`. Which of my axes
   map to which Marathon visual channels (animation / glow / orientation / species / floating label)?
2. Are there **visual channels you currently have but cannot feed** from generic Claude Code signals — i.e.
   channels my typed domain feed *could* light up if I emit for them? (This is the "what extra should I
   produce" question — I'd rather emit a signal you can use than withhold one.)
3. `progressPhase` has **5 values**; if your glow/mood channel only has e.g. 3 discrete states, tell me the
   collapse you want (e.g. `regression-suspected`+`noise-amplification` → one "distressed" state) so I can
   document the lossy mapping rather than you guessing it.
