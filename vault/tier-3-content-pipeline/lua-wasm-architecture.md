---
tags: [tier-3, content-pipeline, lua, scripting, wasm, architecture]
status: research-complete
created: 2026-04-12
---

# Lua + WASM Architecture: Recommended Approach

## Problem Statement

Marathon-web targets `wasm32-unknown-unknown` with wasm-bindgen, web-sys, and wgpu. The standard Rust Lua binding (mlua) wraps Lua's C source, which depends on `setjmp`/`longjmp` -- unavailable on `wasm32-unknown-unknown`. mlua only supports `wasm32-unknown-emscripten`, which is incompatible with wasm-bindgen and our entire web rendering stack.

We need Lua scripting on both native and browser targets to maintain compatibility with the [[community-content-ecosystem]] (decades of community Lua scripts).

See [[lua-in-rust-options]] for the full survey of all options evaluated.

---

## Recommended Architecture: Unified Pure-Rust VM (lua-rs)

### Primary Recommendation

Use **lua-rs** (CppCXY/lua-rs) as a single Lua VM for both native and WASM targets.

```
+------------------------------------------+
|           marathon-scripting crate        |
|                                           |
|   +-----------------------------------+  |
|   |         Scripting Facade           |  |
|   |  (trait-based API for game engine) |  |
|   +-----------------------------------+  |
|                    |                      |
|   +-----------------------------------+  |
|   |            lua-rs (luars)          |  |
|   |     Pure Rust Lua 5.5 Runtime     |  |
|   |   Works on native + wasm32-u-u    |  |
|   +-----------------------------------+  |
|                                           |
+------------------------------------------+
```

### Why lua-rs

| Criterion | lua-rs | Alternatives |
|---|---|---|
| wasm32-unknown-unknown | YES (dedicated luars_wasm crate) | mlua: NO, Piccolo: YES but incomplete stdlib |
| Lua stdlib completeness | 28/30 official tests pass | Piccolo: missing string.find/format/match |
| string.format | YES | Piccolo: NO (showstopper) |
| string pattern matching | YES | Piccolo: NO (showstopper) |
| Single codebase | YES (same Rust code on all targets) | Split architecture adds complexity |
| Lua version | 5.5 (backward-compat with 5.3 scripts) | Piccolo: ~5.3/5.4 partial |
| UserData support | YES (derive macros) | Piccolo: partial |

### Risks and Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| lua-rs is very new (44 stars, single maintainer) | HIGH | Fork early, contribute upstream, maintain our fork |
| Performance slower than C Lua | MEDIUM | Profile on real scripts; game scripts are typically lightweight per-tick |
| 2 of 30 test files still failing | MEDIUM | Identify which tests fail; verify they don't affect Aleph One API surface |
| API may change (pre-1.0) | MEDIUM | Pin version; wrap in our own facade trait |
| Single maintainer could abandon | MEDIUM | Fork + our own maintenance; pure Rust is auditable |

### Implementation Plan

#### Phase 1: Evaluation Sprint (1-2 weeks)
1. Fork lua-rs, build against `wasm32-unknown-unknown` in our Docker setup
2. Run representative Aleph One Lua scripts through it (Cheats.lua, sample HUD scripts)
3. Verify string.format, string.find, string.match work correctly
4. Benchmark tick-loop overhead: how long does `idle()` dispatch take?
5. Test UserData registration for a simple game object (Player with fields + methods)

#### Phase 2: Scripting Facade Crate
Create `marathon-scripting` crate with a trait-based abstraction:

```rust
pub trait LuaBackend {
    fn new_sandboxed() -> Result<Self>;
    fn load_script(&mut self, source: &str) -> Result<()>;
    fn call_trigger(&mut self, name: &str, args: &[LuaValue]) -> Result<Option<LuaValue>>;
    fn register_userdata<T: GameUserData>(&mut self) -> Result<()>;
    fn set_global(&mut self, name: &str, value: LuaValue) -> Result<()>;
    fn get_global(&mut self, name: &str) -> Result<LuaValue>;
    fn serialize_state(&self) -> Result<Vec<u8>>;
    fn deserialize_state(&mut self, data: &[u8]) -> Result<()>;
}
```

This facade allows swapping backends later without engine-wide changes.

#### Phase 3: Game Object Bindings
Implement the [[lua-vm-integration]] API surface:
- Mnemonic registry (MonsterTypes, ProjectileTypes, etc.)
- Player, Monster, Projectile, Item, Effect UserData
- Polygon, Line, Side, Endpoint, Platform, Light geometry objects
- Game, Level, Music global objects
- Trigger dispatch (idle, postidle, damage hooks, switch hooks, etc.)

#### Phase 4: HUD Lua
- Screen, Fonts, Images, Shapes drawing API
- Bridge to wgpu render pipeline
- Frame-rate independent draw() calls

---

## Fallback Architecture A: mlua (Native) + Piccolo (WASM)

If lua-rs proves too immature or too slow, use a split backend:

```
+------------------------------------------+
|           marathon-scripting crate        |
|                                           |
|   +-----------------------------------+  |
|   |         Scripting Facade           |  |
|   |  (trait-based API for game engine) |  |
|   +----------------+------------------+  |
|                    |                      |
|   #[cfg(not(      |     #[cfg(target_   |
|    target_arch =   |      arch =         |
|    "wasm32"))]     |      "wasm32")]     |
|   +------------+   |   +--------------+  |
|   |   mlua     |   |   |   Piccolo    |  |
|   | Lua 5.4    |   |   | Pure Rust    |  |
|   | C bindings |   |   | Lua ~5.3/5.4 |  |
|   +------------+   |   +--------------+  |
|                                           |
+------------------------------------------+
```

### Advantages
- mlua is production-grade on native (best API, best perf)
- Piccolo works on wasm32-unknown-unknown
- Each backend is well-suited to its target

### Disadvantages
- **Piccolo's missing stdlib is still a problem.** string.format and pattern matching are absent. We would need to:
  - Contribute string library implementations upstream to Piccolo
  - Or implement them ourselves using the [lua-patterns](https://github.com/stevedonovan/lua-patterns) crate
  - Or accept reduced script compatibility on web
- Two backends means testing every script on both VMs
- Behavioral differences between Lua 5.4 (mlua) and Piccolo's ~5.3/5.4 approximation
- Double the maintenance burden for the scripting facade
- Subtle bugs from VM behavioral differences are hard to catch

### When to Choose This
- lua-rs performance is unacceptable (>1ms per idle() call)
- lua-rs has critical bugs that block Aleph One script compatibility
- mlua's native performance is essential for the desktop target

---

## Fallback Architecture B: mlua (Native) + Wasmoon via JS Bridge (WASM)

Use mlua for native and delegate to Wasmoon (Lua 5.4 compiled to WASM, JS API) on the web target:

```
+------------------------------------------+
|           marathon-scripting crate        |
|                                           |
|   #[cfg(not(wasm32))]  | #[cfg(wasm32)] |
|   +------------+        | +------------+ |
|   |   mlua     |        | |  JS Bridge | |
|   | Lua 5.4    |        | | (wasm-     | |
|   | C bindings |        | |  bindgen)  | |
|   +------------+        | +-----+------+ |
|                         |       |        |
|                         | +-----v------+ |
|                         | |  Wasmoon   | |
|                         | | (npm, JS)  | |
|                         | | Lua 5.4 VM | |
|                         | +------------+ |
+------------------------------------------+
```

### How It Works
1. On web: Rust WASM calls JavaScript functions via wasm-bindgen `#[wasm_bindgen]` extern blocks
2. JavaScript code uses Wasmoon (npm package) to manage a Lua 5.4 VM
3. Script loading, trigger dispatch, and value passing go through the JS bridge
4. Results flow back from JS to Rust WASM

### Advantages
- Full Lua 5.4 compatibility on BOTH targets
- Wasmoon is stable and actively maintained
- Complete stdlib including string patterns, formatting
- mlua is production-grade on native

### Disadvantages
- Complex architecture with three language boundaries (Rust -> JS -> Lua)
- Performance overhead from Rust-WASM <-> JS <-> Lua-WASM marshalling
- Debugging is significantly harder across the bridge
- Wasmoon is an npm dependency (adds JS build tooling)
- Value conversion overhead for every trigger call (30+ times per second for idle/postidle)
- Promise/async complexity for Wasmoon's coroutine model

### When to Choose This
- Full Lua compatibility on web is absolutely required
- Performance of per-tick calls is acceptable (needs benchmarking)
- The JS bridge complexity is manageable

---

## Fallback Architecture C: No Lua on Web

Defer Lua scripting to native-only. Web builds run without script support.

### Advantages
- Simplest architecture
- No compromises on native Lua quality (mlua)
- Web builds still playable for vanilla Marathon content

### Disadvantages
- Major feature gap: many community scenarios are unplayable on web
- Community expectation is that web builds "just work"
- Undermines the value proposition of the web target

### When to Choose This
- As a temporary measure while building toward a full solution
- If web target is primarily a demo/preview and not the primary platform

---

## Decision Matrix

| Architecture | Lua Compat (Web) | Complexity | Performance (Web) | Risk Level |
|---|---|---|---|---|
| **lua-rs unified** | 5.5 (28/30 tests) | Low | Moderate | HIGH (maturity) |
| mlua + Piccolo split | ~5.3/5.4 partial | High | Moderate | HIGH (missing stdlib) |
| mlua + Wasmoon bridge | 5.4 full | Very High | Lower (JS bridge) | MEDIUM |
| mlua native only | None on web | Low | N/A | LOW |

---

## Recommendation

### Short-Term (Next 3 Months)
1. **Evaluate lua-rs immediately.** Fork it, run Aleph One scripts, benchmark.
2. If lua-rs works: proceed with unified architecture.
3. If lua-rs has blocking issues: implement mlua for native as Phase 1, defer web Lua.

### Medium-Term (3-9 Months)
4. If lua-rs is adopted: contribute bug fixes upstream, pin our fork.
5. If lua-rs is rejected: evaluate contributing string stdlib to Piccolo OR building the Wasmoon JS bridge.

### Long-Term (9+ Months)
6. Monitor pure-Rust Lua ecosystem. Both Piccolo and lua-rs are actively developing.
7. The facade trait design means we can swap backends with minimal engine changes.

### Why Not Start with mlua + Piccolo Split?
The missing `string.format` and `string.find`/`string.match`/`string.gsub` in Piccolo are genuinely blocking for Aleph One scripts. These are not obscure stdlib functions -- they are used pervasively in HUD scripts (formatting health/ammo displays), solo scripts (parsing text), and utility code. Contributing a full Lua pattern matching implementation to Piccolo is a significant undertaking (Lua's pattern engine is ~500 lines of tricky C code). lua-rs already has this working.

### Why Not Start with the Wasmoon Bridge?
The JS bridge architecture is the highest-complexity option. Every value passed between Rust and Lua goes through two serialization boundaries. For a game running `idle()` every tick (30 times per second) with potential access to dozens of game objects, the marshalling overhead could be significant. It should be a last resort.

---

## Aleph One Lua Feature Priority for VM Evaluation

When evaluating any Lua VM, test these features in priority order:

1. **Tables with metatables** -- game object proxies
2. **string.format** -- HUD text rendering, debug output
3. **string.find / string.match / string.gsub** -- text parsing
4. **pcall / error** -- error handling
5. **Closures with upvalues** -- callback registration
6. **pairs / ipairs / next** -- iteration over game collections
7. **Coroutines** -- camera paths, timed sequences
8. **math library** -- geometry, random numbers
9. **table.insert / table.remove / table.sort** -- collection management
10. **tonumber / tostring** -- type conversion
11. **UserData with __index / __newindex** -- field access on game objects
12. **State serialization** -- saved game support

---

## Related Notes

- [[lua-vm-integration]] -- Full Aleph One Lua API reference
- [[lua-in-rust-options]] -- Detailed comparison of all options evaluated
- [[community-content-ecosystem]] -- Which scenarios depend on Lua
- [[plugin-system-patching]] -- How Lua scripts are packaged and loaded
