## Why

The sim already computes a complete world every tick, but the only way to read or touch that world is a human at a browser keyboard. There is no programmatic surface, so an autonomous agent cannot peek into the level or act on it without someone first writing a bespoke client and wire protocol.

MCP closes that gap with almost no new infrastructure. Its primitives map cleanly onto what the sim already exposes — **tools = actions, resources = observations** — and the official Rust SDK (`rmcp`) auto-generates input schemas from `#[tool]` macros, so wiring `marathon-sim` actions to tools and observations to resources is low-boilerplate. The payoff: **any MCP-capable agent (Claude, Cursor, etc.) joins the world with zero custom client.** This is the cheapest "agents touch the world" path — a Mode-B-lite gateway that ships *before* full netcode, reusing Mode A's intent/observation plumbing rather than building a transport stack. See `vault/agentic-frontend/mode-b-multiclient-access.md` §3 and `vault/agentic-frontend/roadmap-mode-a-to-b.md` (Phase MCP).

The model is deliberately **deliberative, not real-time**: MCP is request/response / client-pull, and the protocol is trending toward a stateless core (Tasks-via-polling, sampling/SSE-push deprecated). So the sim owns the clock and never blocks — the world keeps ticking, the agent reads the latest observation resource, submits an intent via a tool, and the sim applies it on its own schedule. MCP is for interoperable deliberative agents; real-time clients stay on the custom WS/WebTransport surface (`implement-networking-multiplayer`). Two surfaces over one sim core.

## What Changes

- Stand up an `rmcp`-based MCP server that runs **alongside** the sim, exposing the running world to any MCP client over the standard protocol — no custom client, no custom wire format.
- Map a curated set of high-level sim **actions to MCP tools** (e.g. move/turn intent, `use`/action-key, fire, say), each validated server-side; tools enqueue **intents** into the existing tick pipeline and return immediately — they never block on a tick.
- Map **observations to MCP resources** served from the latest serializable `WorldSnapshot`: a self-state resource, a nearby-entities resource (type / bearing / numeric relative distance), and an optional NL summary rendered over the same struct, so an agent can pull current world state on demand.
- Adopt the **snapshot/intent model** end to end: reads come from the most recent snapshot, writes are intents applied on the sim's schedule; action latency is explicit (an intent observed at tick *t* applies no earlier than *t+1*).
- Scope every observation resource to what the requesting agent's avatar can legitimately perceive (LOS/AOI), treating the **observation payload as a trust boundary**, not just the action API.
- Offer low-frequency MCP **resource-subscription doorbells** for significant events (took damage, geometry change, target spotted) — a "come read it" notification, explicitly not a per-frame push channel.

## Capabilities

### New Capabilities
- `mcp-world-server`: The `rmcp`-based MCP server process/handle that runs alongside the sim, advertises the tool + resource surface, owns the snapshot/intent contract (reads from latest snapshot, writes as non-blocking intents, explicit `t+1` action latency), and never blocks the tick loop.
- `mcp-observation-resources`: The set of MCP resources exposing world observations from the latest `WorldSnapshot` — self-state, nearby entities with numeric relative geometry, optional NL summary, and low-frequency event-subscription doorbells — each LOS/AOI-scoped per requesting agent as a security boundary.
- `mcp-action-tools`: The set of MCP tools exposing high-level, server-validated sim actions (movement/turn intent, action-key/`use`, fire, say) that enqueue intents into the existing tick pipeline and return without blocking.

### Modified Capabilities
<!-- None: this change adds a new external surface over the sim; it does not change the requirements of any existing capability. -->

## Impact

- **Depends on `decouple-tick-snapshot`** (hard prerequisite): the gateway is built entirely on the headless `tick(inputs)` + serializable, scoped `WorldSnapshot` that that change establishes. This proposal does not re-spec the tick/snapshot decoupling — it consumes it.
- **Reuses `agent-dashboard-mode-a` plumbing**: the same intent-injection and observation-projection seams Mode A introduces (`TickInput`, `snapshot()`/`render_snapshot`, entity readout) back the MCP tools and resources, rather than a parallel mechanism.
- **Complements, does not replace, `implement-networking-multiplayer`**: MCP is the deliberative, request/response agent gateway; the custom WS/WebTransport netcode remains the real-time client surface. The two are independent surfaces over the same sim core and can land in either order after the snapshot prerequisite.
- New crate / module hosting the `rmcp` server (tokio-async) and the tool/resource bindings; adds `rmcp` (and its tokio/serde/schemars deps) to the workspace.
- No rendering, no GPU, and no browser-transport work in this change; security hardening beyond observation scoping (auth, per-agent rate limiting, fairness envelopes) is deferred to the later full Mode-B phases.
