## Context

The sim already computes a complete world on every tick, but that world has **no programmatic surface**. The only way to read it or touch it is a human at a browser keyboard driving the renderer. An autonomous agent cannot peek into a level or act on it without someone first authoring a bespoke client *and* a bespoke wire protocol — a transport, a schema, a session model — none of which exist yet. The world is computed and immediately consumed by the GPU; nothing else can see it.

`decouple-tick-snapshot` (Phase 0, merged as PR #12) changes the shape of that core: it delivers a headless `marathon-sim` `tick(inputs)` driven by a `TickInput` intent struct, plus a serializable `render_snapshot() -> WorldSnapshot` that captures the world as plain data decoupled from rendering. That is the precondition for *any* non-GPU consumer. Mode A (`agent-dashboard-mode-a`) is the first consumer — it injects desired state through `TickInput` and reads back through `render_snapshot()`. This change is the **second** consumer of the exact same two seams, exposed not to a dashboard daemon but to the open MCP protocol.

The cross-repo contract for that exposure is **frozen**. `sync/CONTRACT.md` §8 (FROZEN) pins the consumer MCP surface byte-for-byte against the producer's: observations are MCP **resources** (`world://self|nearby|summary|events`), actions are MCP **tools** (`move/turn/use/say`), **there is no `fire` tool — no weapons, by constitution**, the **observation payload is a trust boundary** (not merely the action API), and the obs scope is **LOS/AOI on the consumer** (mirroring the producer's lease/partition AOI), with **obs-scope == action-auth scope**. This design must land *inside* that frozen seam; it does not get to redefine it.

Constraints this design inherits and does not relitigate:
- The sim owns the clock. Nothing the gateway does may block, stall, or back-pressure the tick loop (CONTRACT §8; Mode A Decision 4).
- MCP is request/response and trending toward a stateless core (Tasks-via-polling; sampling/SSE-push deprecated). The gateway is **deliberative, not real-time** — real-time clients stay on the custom WS/WebTransport surface owned by `implement-networking-multiplayer`.
- The verb set is constitutional: collaborative/embodied, never combat. `fire` is excluded at the contract level.

## Goals / Non-Goals

**Goals:**
- A **zero-custom-client** MCP gateway over the headless snapshot/intent core: **tools = actions, observations = resources**, so any MCP-capable agent (Claude, Cursor, etc.) joins the world over the standard protocol with no bespoke client and no bespoke wire format.
- Reuse the *existing* sim seams — Mode A's `TickInput` intent injection and `render_snapshot()`/`WorldSnapshot` read-back — rather than standing up a parallel read/write mechanism.
- Hold the **snapshot/intent contract** end to end: reads come from the latest snapshot; writes are intents applied on the sim's own schedule, with explicit `t+1` action latency.
- Make every observation resource a **trust boundary**, LOS/AOI-scoped to what the requesting agent's avatar can legitimately perceive, with the obs scope and the action-auth scope identical.

**Non-Goals:**
- **Rendering / GPU** of any kind — the gateway never touches a renderer; it reads `WorldSnapshot` data and emits `TickInput` intents.
- **Real-time netcode / per-frame push** — that is `implement-networking-multiplayer`'s custom WS/WebTransport surface. The two are independent surfaces over one sim core.
- **Auth, per-agent rate-limiting, fairness envelopes** beyond observation-scoping — these are later full Mode-B hardening (`agent-environment-mode-b`). This change ships only the obs-scope trust boundary.
- **Any weapon / `fire` tool.** Excluded by constitution at the frozen-contract level (§8). The verb surface is `move/turn/use/say` and nothing else.

## Decisions

### Decision 1: Build on the `rmcp` Rust SDK with `#[tool]` schema auto-generation, not a bespoke wire protocol

The gateway is an `rmcp` (official MCP Rust SDK) server: tools are Rust functions annotated with `#[tool]`, whose JSON input schemas are **auto-generated** from the argument types via `schemars`. The whole value proposition of this change is "agents touch the world with **zero new infrastructure**" — that only holds if we adopt the protocol an agent already speaks rather than inventing one.

*Rationale.* A bespoke wire protocol (custom framing + a hand-written schema + a client SDK we publish and version) is exactly the cost MCP exists to eliminate. With `rmcp`, the protocol, the transport, the capability advertisement, and the input-schema generation are all off-the-shelf; our job shrinks to *binding* a curated tool/resource set onto the sim seams. The macro-derived schemas also keep the tool surface and its documentation in one place, so the contract an agent sees is generated from the same Rust types the sim validates against — no drift between "what we documented" and "what we accept."

*Alternatives considered.* (a) **Bespoke WS/WebTransport protocol** — rejected for this surface; it is precisely the real-time path that `implement-networking-multiplayer` owns, and forcing deliberative agents onto it reintroduces the custom-client cost. (b) **Hand-rolled JSON-RPC without the SDK** — rejected: we would re-implement MCP's capability handshake and resource/subscription semantics by hand and lose interop with the existing agent ecosystem for no benefit.

### Decision 2: tools = actions, resources = observations, mapped onto the *existing* sim seams (not a parallel mechanism)

MCP's two primitives map cleanly onto what the sim already exposes after Phase 0: **MCP tools** are the curated high-level *actions* (`move`/`turn` intent, `use`/action-key, `say`), and **MCP resources** are the *observations* (`world://self`, `world://nearby`, `world://summary`, `world://events`). Both are bound to the seams Mode A introduced: tools enqueue into the **same `TickInput` intent pipeline** at `tick.rs`; resources are projected from the **same `render_snapshot()` `WorldSnapshot`**.

*Rationale.* Reuse over reinvention is a hard requirement (proposal Impact; CONTRACT §8 alignment row). A parallel read path or a parallel write path would be a second source of truth for "what the agent did" and "what the agent saw," doubling the surface that has to stay consistent with the tick loop and inviting exactly the kind of duplicate-mechanism drift the project has already been bitten by elsewhere. Binding to `TickInput`/`render_snapshot` means MCP agents and the Mode A dashboard observe the identical world and act through the identical intent gate — they are two *clients* of one core, which is the whole "two surfaces over one sim" thesis.

*Alternatives considered.* A dedicated MCP-only intent queue and MCP-only world view — rejected; it would let the MCP surface diverge from the rendered/dashboard world and force per-surface validation logic.

### Decision 3: Snapshot/intent model with explicit `t+1` action latency — the sim owns the clock and is never blocked

Reads are served from the **most recent** `WorldSnapshot`; writes are **intents** that the gateway enqueues and then returns from immediately. An intent observed/submitted at tick *t* applies **no earlier than `t+1`**; this latency is part of the contract, surfaced to the agent, not hidden behind a fake synchronous response.

*Rationale.* The non-negotiable constraint is that the gateway never blocks the tick (CONTRACT §8; Mode A Decision 4). If a tool call awaited the effect of its own intent, a slow or absent tick would stall the agent and — worse — a chatty agent could back-pressure the sim. Decoupling read-from-latest-snapshot from write-as-future-intent makes the gateway structurally incapable of stalling the loop: the sim advances on its own schedule regardless of how many tools are mid-flight. Making `t+1` *explicit* rather than papering over it gives deliberative agents an honest model — they know an action they just submitted will not be reflected in the snapshot they can read this instant.

*Alternatives considered.* Synchronous "apply-and-return-the-result" tools — rejected; that is the psDooM-style render-thread shell-out mistake Mode A already rejected, just relocated into MCP: a slow caller stalls the world.

### Decision 4: Deliberative request/response, not real-time; defer per-frame push, offer only low-frequency resource-subscription doorbells

The gateway is **pull-based deliberation**: the agent reads a resource when it wants to think, submits a tool when it wants to act. There is **no per-frame push channel**. The one push affordance is **low-frequency MCP resource-subscription doorbells** (`world://events`): "something significant happened (took damage, geometry changed, target spotted) — come read the resource," explicitly *not* a stream of per-tick state.

*Rationale.* MCP itself is trending stateless (Tasks-via-polling; sampling/SSE-push deprecated), so designing a high-frequency push channel on top of it fights the protocol and would age badly. Deliberative agents reason in seconds-to-minutes, not frames; a per-frame firehose is both wrong for the consumer and a standing back-pressure risk against the tick. A subscription *doorbell* preserves "you don't have to poll blindly for the important stuff" without committing to streaming world state — it is a notification, and the payload is still fetched via a normal scoped resource read (so the trust boundary of Decision 5 still applies to it).

*Alternatives considered.* Per-tick SSE/streaming push of the snapshot — rejected; it couples to a deprecated MCP mechanism, invites tick back-pressure, and is the real-time surface owned by `implement-networking-multiplayer`.

### Decision 5: Observation payload is a trust boundary — LOS/AOI-scoped, with obs-scope == action-auth scope

Every observation resource is scoped server-side to **only what the requesting agent's avatar can legitimately perceive** (line-of-sight / area-of-interest), and the **same scope governs which actions that agent may take** (CONTRACT §8: obs-scope = action-auth scope; obs payload = trust boundary). The gateway never serves the full world to an agent and never trusts the agent to self-limit.

*Rationale.* The contract is explicit that the trust boundary is the *observation* payload, not merely the action API — i.e. leaking what an agent can *see* is itself a security failure, independent of what it can *do*. Scoping at the server, on the way out of the snapshot, is the only place that holds: a client-side filter is no filter. Tying action authorization to the same AOI computation means an agent cannot act on what it could not have seen, which closes the obvious "act through walls" gap and keeps a single scope computation as the one enforcement point (mirroring the producer's lease/partition AOI, so the two repos enforce the same shape on either side of the seam).

*Alternatives considered.* Serve the full snapshot and document that clients "should" only use their local view — rejected outright; that is a non-boundary and contradicts §8. Scope actions but not observations — rejected; §8 names the obs payload specifically as the trust boundary.

### Decision 6: No `fire` tool — `move/turn/use/say` only, by constitution

The tool surface is exactly `move`, `turn`, `use`, and `say`. There is **no `fire` / weapon tool**. This is fixed by the frozen CONTRACT §8 ("`move/turn/use/say` … **no `fire`** — no weapons, by constitution"), consistent with Mode A's collaborative-only verb set.

*Note — supersession.* The change's own `proposal.md` mentions `fire` in a couple of places (What Changes; the `mcp-action-tools` capability blurb list "movement/turn intent, action-key/`use`, fire, say"). That mention is **superseded**: it predates the freeze of CONTRACT §8, which is the authority and which excludes `fire`. The generated spec must bind to §8, i.e. omit `fire`. Treat the proposal's `fire` reference as historical, not normative.

*Rationale.* The constitution is the guardrail against metaphor drift back toward combat (Mode A Decision 6 / Risks). Excluding the verb at the *contract* level — rather than leaving it in and "choosing not to wire it" — means there is no fire affordance to accidentally enable later, and the producer and consumer surfaces stay symmetric (the producer's mirror tools are `check_in/offer_help/ask_to_break/send_home/retire`, equally weaponless).

## Risks / Trade-offs

- **A tool call could block the tick loop** → The snapshot/intent split (Decision 3) makes blocking structurally impossible: tools enqueue an intent and return; the sim drains intents on its own schedule; reads come from the latest already-computed snapshot. No gateway code path awaits a tick.
- **Observation leak past AOI (agent sees/acts through walls)** → Scope server-side on the way out of the snapshot, never client-side; bind action-auth to the same AOI so an agent cannot act on what it could not see (Decision 5). The obs payload is treated as the trust boundary per §8.
- **MCP protocol churn (deprecation of push/sampling, Tasks-vs-streaming flux)** → Stay on the stateless/deliberative core and the official `rmcp` SDK (Decisions 1, 4); avoid building on deprecated push primitives; the only push affordance is a low-frequency doorbell that degrades to plain polling if subscriptions are unavailable.
- **`rmcp` SDK maturity / API instability** → Confine `rmcp` to a single new crate/module behind the tool/resource bindings (Migration Plan); the sim core depends on none of it, so an SDK break is contained to the gateway and the fallback is simply not starting the server.
- **Tool surface drift vs. what the sim actually validates** → Auto-generate input schemas from the same Rust types the sim validates against (`#[tool]` + `schemars`, Decision 1), so the advertised contract and the accepted contract are one artifact.
- **Metaphor drift back toward combat (re-adding `fire`)** → Exclude `fire` at the frozen-contract level, not at the wiring level (Decision 6); there is no weapon tool to enable.

## Migration Plan

The change is **purely additive**. It introduces a **new crate / module** hosting the `rmcp` (tokio-async) server and the tool/resource bindings, plus the `rmcp` dependency (and its tokio/serde/schemars transitive deps) in the workspace. It **modifies no existing capability** — it adds a new external surface that *reads* `render_snapshot()` and *writes* `TickInput` through seams that already exist.

- **Gate:** hard-gated on `decouple-tick-snapshot` (**merged**, PR #12). The gateway is built entirely on that change's headless `tick(inputs)` + serializable, scoped `WorldSnapshot`; it does not re-spec or modify them.
- **Rollout:** start the `rmcp` server alongside the sim. Because it is out-of-band of the tick loop (Decision 3), enabling it cannot affect a running sim's frame timing.
- **Rollback:** **don't start the server.** With the server stopped, the sim, the renderer, and the Mode A dashboard are byte-for-byte unaffected — there is nothing to revert in the core.

## Open Questions

- **Doorbell event taxonomy** — what is the minimal significant-event set behind `world://events` (took-damage, geometry-change, target-spotted, …), and is it derived from `SimEvent` already emitted by `decouple-tick-snapshot` or a gateway-side classifier over snapshot deltas?
- **AOI radius / LOS defaults** — what are the default perception bounds (radius, occlusion model) for the scope computation, and should they be per-`world://` resource (e.g. `summary` coarser than `nearby`) or uniform? How do these align numerically with the producer's lease/partition AOI so both sides enforce the same envelope?
- **Intent-queue backpressure** — if a single agent (or many) submit intents faster than the sim drains them, what is the policy: bounded per-agent queue with drop-oldest, reject-with-error, or coalesce-to-latest per intent kind? This is the one place an agent could indirectly pressure the loop, and it sits at the edge of the "no auth/rate-limiting in this change" non-goal.
- **`say` payload scope** — is `say` proximity/AOI-scoped on emission (only nearby avatars receive it) the same way observations are scoped, and does that reuse the Decision 5 AOI computation?
- **Snapshot freshness contract** — does a resource read always serve the latest committed snapshot, or is a small staleness window acceptable to avoid contending with the writer? What does the agent get told about the snapshot's tick number so `t+1` latency is legible?
