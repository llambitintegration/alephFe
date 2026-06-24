## Why

This is the **north star (Mode B)**: humans and autonomous LLM agents co-inhabiting one running alephone-rust world, where agents are first-class participants rather than the dashboard subjects of Mode A. The research is settled on the shape (single headless authoritative server owns the world clock; every client — human, agent, spectator — submits **intents** and receives **scoped observations**; the LLM never sits on the hot loop), but the engineering is months of distributed-systems and LLM-serving work that does not yet exist in any form. This proposal stakes out that horizon so the capabilities below have a contract, and so the layers it depends on are built toward a known destination — it is **intentionally high-level and not implementation-ready**.

The central, framing constraint: a full LLM forward pass is **300ms–13s per decision (6–133× over a 100ms FPS budget)** — you cannot put an LLM inside a 33ms tick. Every viable design decouples slow LLM cognition (async, across many frames) from a cheap reactive layer (in-tick). Add to that the welfare stance that governs the whole project: in Mode B the constitutional/collaborative posture is expressed as **fairness envelopes** — bounding agents to human-plausible reaction/input rates so the shared world is non-adversarial and positive-sum, not an arms race.

## What Changes

Phase 3 adds the **agent gateway** as a new client type layered over the scoped-obs/intent plumbing that earlier phases provide. It breaks into four sub-phases (3a–3d from the roadmap):

- **3a — Agent endpoint + obs/intent API.** A custom WS/WebTransport agent endpoint exposing a **PettingZoo-Parallel-shaped** observation/intent surface backed by an **action queue** (action from observation at tick *t* applies at *t+1*; `reset()` is respawn/init-only, never freezing the world). Validated with a scripted reflex bot before any LLM is involved.
- **3b — Per-agent Executor.** A **GOAP + behavior-tree Executor** running at tick rate that consumes **high-level intents** (`move_to(x)`, `engage(target)`, `take_cover`, `use(switch)`, `say(text)`) and **NL + tabular, event-driven observations** (short NL summary line + compact tabular entity list with numeric relative bearings/distances; dual-channel structured JSON + NL over one struct so RL and LLM agents read the same world).
- **3c — Slow Mind / Fast Mind serving stack.** The **three-tier real-time LLM serving** architecture: Tier 1 reactor (per-tick, in-loop, **no LLM** — reads the latest cached intent from a single-slot buffer); Tier 2 tactical LLM (off-loop ~1–10 Hz, one **SGLang/RadixAttention** server sharing the world-state prompt prefix across all agents, small distilled/quantized model + **EAGLE-3** speculative decoding + continuous batching); Tier 3 strategic planner (event-gated ~0.1–1 Hz for plans/dialogue). The fast loop **never `await`s** the LLM.
- **3d — Fairness envelopes + sandboxing.** Reaction-delay floor, input/turn-rate caps, per-agent-ID rate limiting, and per-agent sandboxing — the welfare expression and the abuse boundary in one.

## Capabilities

### New Capabilities
- `agent-gateway`: The agent-as-client surface — WS/WebTransport endpoint, PettingZoo-Parallel-shaped obs/intent API, action queue with explicit one-tick action latency, and intent validation, all riding the existing scoped-observation/intent plumbing without giving agents a privileged path into the sim.
- `agent-executor`: The per-agent GOAP + behavior-tree executor that turns high-level intents into per-tick atomic actions, plus the NL + tabular event-driven observation representation (dual structured/NL channel) and skill-interrupt semantics.
- `realtime-llm-serving`: The Slow Mind / Fast Mind three-tier serving stack — in-loop reactor (no LLM), off-loop tactical SGLang server with shared world-prompt prefix + EAGLE-3 + continuous batching, and event-gated strategic planner — with the invariant that the hot loop only ever reads the latest cached plan.
- `fairness-envelopes`: The constitutional welfare boundary for live agents — reaction-delay floor, input/turn-rate caps, per-agent-ID rate limiting, and per-agent sandboxing — making the shared world non-adversarial and resistant to weaponized over-broad input.

### Modified Capabilities
<!-- None. Mode B is additive: it layers a new client type onto plumbing delivered by prerequisite changes, and modifies no existing alephone-rust spec's requirements. -->

## Impact

**Dependency chain (this is the horizon — it sits at the end of a long prerequisite path):**

```
decouple-tick-snapshot          (keystone: clean tick(inputs) + serializable scoped render_snapshot)
   └──> multiplayer-foundation  (headless authoritative server + renet/WS netcode +
        │                        client-side prediction + per-client LOS/AOI scoped-obs/intent —
        │                        Phases 1+2: single then multi human)
        └──> world-mcp-gateway  (deliberative agent gateway precedent: rmcp tools=actions,
             │                   resources=observations, snapshot/intent, world never blocks)
             └──> agent-environment-mode-b  (THIS — real-time agent gateway + Executor +
                                             LLM serving + fairness; Phase 3)
```

- **Reuses, does not replace:** Mode B's agent gateway is a *new client type* over the scoped-obs/intent plumbing from `multiplayer-foundation`; it inherits the LOS/AOI observation-scoping security boundary verbatim. `world-mcp-gateway` supplies the deliberative (request/response, never-blocks) gateway pattern that 3a generalizes to a real-time push surface.
- **New external/runtime surface (none exist in the repo today):** a GPU LLM-serving box (SGLang or TensorRT-LLM), a small distilled/quantized model + EAGLE-3 draft, and an off-process async cognition tier wired to per-agent single-slot intent buffers. The sim crate gains an `update_agents()` path parallel to `update_monsters()`; the netcode gains an agent transport + action queue.
- **Open research risks (why this is not yet implementation-ready):**
  - **Latency budget** — nothing reasoning-heavy fits in 33ms; intents are stale by design and the reactor must stay coherent on a several-hundred-ms-old plan.
  - **Many-agents-share-a-world-prompt** — real shared-prefix gains are far below vendor warm-cache marketing numbers; the shared world-prompt KV must stay resident via priority eviction.
  - **Scheduling against the frame deadline** — naive EDF collapses under load; need hybrid prioritization + graceful degradation (lower tick rate / cheaper reactor / skip LLM refresh for distant agents).
  - **Speculative decoding / quantization** — EAGLE-3 is safest across batch sizes, but FP8/AWQ kernels can be *slower* than FP16 unless validated on the target GPU.
  - **Fairness is under-researched** — there is no settled reference for human-plausible envelopes against a slow-but-superhuman agent; this engine would set a reference design.
- **Out of scope here:** the prerequisite phases themselves (owned by `decouple-tick-snapshot`, `multiplayer-foundation`, `world-mcp-gateway`); Mode A (`agent-dashboard-mode-a`); and any commitment to a specific model, GPU, or serving framework — those are design/spike decisions, not proposal-level commitments.
