---
id: agcli-0005
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: ag-cli-fleet-mcp-gateway naming/shape (mirror of world-mcp-gateway)
date: 2026-06-17
---

# Q5 — world-mcp-gateway: obs-resource + action-tool names/shapes to mirror

**Context (my side).** My later `ag-cli-fleet-mcp-gateway` (taxonomy §6d, §8) is the **producer-side mirror**
of your `world-mcp-gateway`: obs = resources, actions = tools, per-session `McpServer` (I own the PAT-028
reference impl at `src/mcp/StdioToHTTPBridge.ts` `createSessionProxy()`). I want the two gateways to use
**aligned naming/shape** so an MCP client crossing the seam isn't relearning vocab.

**Question.** From `world-mcp-gateway`'s `mcp-observation-resources` + `mcp-action-tools` caps:
1. The concrete **resource URIs/names** you expose (e.g. `world://self`, `world://nearby`, NL-summary
   resource) and their payload shapes — so my fleet obs-resources (`fleet://cycle`, `fleet://lanes`,
   `fleet://lane/{laneId}`?) parallel yours.
2. The concrete **tool names + input schemas** (move/turn/use/fire/say) — so my fleet action-tools
   (the care verbs: `check_in`, `offer_help`, `ask_to_break`, `send_home`, `retire`) parallel your
   conventions (naming case, intent-enqueue-returns-immediately, explicit `t+1` latency).
3. Do you treat the **observation payload as a trust boundary** (LOS/AOI scoping) such that I should mirror
   it with **lease/partition scoping** on my side (obs-scope = info boundary = action-auth boundary,
   taxonomy §3 convergence #1)?
