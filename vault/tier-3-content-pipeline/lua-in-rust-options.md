---
tags: [tier-3, content-pipeline, lua, scripting, wasm, research]
status: research-complete
created: 2026-04-12
---

# Lua-in-Rust Options: Detailed Comparison

## Context

The [[lua-vm-integration]] note establishes that Aleph One uses Lua 5.3 for three script types (solo, HUD, stats) with a large API surface. The [[community-content-ecosystem]] shows major scenarios depend heavily on Lua scripting. Our engine targets both native desktop and browser (via `wasm32-unknown-unknown` + wasm-bindgen in [[marathon-web|marathon-web]]).

The critical constraint: **marathon-web uses `wasm32-unknown-unknown`** with wasm-bindgen, web-sys, and wgpu. This target has no libc, no setjmp/longjmp, and is incompatible with `wasm32-unknown-emscripten`. Any solution must either work on `wasm32-unknown-unknown` directly or use a split-backend architecture.

---

## Option 1: mlua (C Lua Bindings)

**Repository:** [mlua-rs/mlua](https://github.com/mlua-rs/mlua)
**Version:** v0.11.6 (January 2026)
**License:** MIT
**Stars:** ~3k+ | **Downloads:** High (most popular Lua crate)

### Lua Version Support
- Lua 5.5, 5.4, 5.3, 5.2, 5.1 (via feature flags)
- LuaJIT (via `luajit` feature)
- Luau (via `luau` feature)

### WASM Support
- **wasm32-unknown-emscripten:** Supported for all Lua/Luau versions (excluding JIT)
- **wasm32-unknown-unknown:** NOT supported. Lua's C source depends on `setjmp`/`longjmp` for error handling, which has no implementation on this target.
- **wasm32-wasi:** NOT supported. wasi-libc does not fully implement the libc calls Lua requires.

### Key Problem for Us
`wasm-bindgen` (used by marathon-web, wgpu, web-sys) **only works with `wasm32-unknown-unknown`**. It is fundamentally incompatible with `wasm32-unknown-emscripten`. Switching marathon-web to emscripten would break our entire web rendering stack (wgpu, web-sys, wasm-bindgen-futures).

### API Ergonomics
Excellent. mlua is the gold standard for Rust-Lua interop:
- `UserData` trait with fields/methods macros
- Async/await support
- Serde integration for serialization
- Scope-based temporary callbacks
- Thread safety via `send` feature
- Comprehensive error handling (wraps all longjmp points in pcall)

### Performance
Near-native Lua performance. Benchmarks (script-bench-rs) show mlua adds minimal overhead over raw Lua C API calls. Lua 5.4 runs ~2x faster than Rhai for compute-heavy workloads.

### Verdict
**Best API, best compatibility, cannot target wasm32-unknown-unknown.** Ideal for native-only builds. For a dual-target architecture, mlua can serve as the native backend only.

---

## Option 2: Piccolo (Pure Rust Lua VM)

**Repository:** [kyren/piccolo](https://github.com/kyren/piccolo)
**Version:** v0.3.3 (June 2024)
**License:** MIT or CC0 1.0 (dual-licensed)
**Stars:** ~1.8k

### Lua Version Support
Targets Lua 5.3/5.4 behavior pragmatically. Not a strict 1:1 implementation -- error message formatting and iteration order may differ from PUC-Rio Lua.

### WASM Support
- **wasm32-unknown-unknown:** YES. Pure Rust, no C dependencies. Confirmed working in browser -- kyren's blog post embeds interactive Piccolo REPLs that run in the browser via WASM.
- Fish Folk / Jumpy uses Piccolo for game scripting **including web builds**: "Since it's written in Rust, Piccolo makes it possible for us to embed Lua scripting even in the web builds of our game."

### Standard Library Completeness (CRITICAL LIMITATION)

**Implemented:**
- Core: assert, type, pairs, ipairs, next, rawget/rawset, select, tonumber, tostring, getmetatable, setmetatable, print, pcall, error
- Coroutines: create, resume, yield, status, running (plus non-standard `coroutine.yieldto`)
- Math: All trig functions, log, floor, ceil, random
- Table: insert, remove, concat, pack, unpack, sort, move
- String: byte, char, len, lower, upper, sub, reverse

**NOT implemented (major gaps):**
- **string.find, string.match, string.gmatch, string.gsub** -- Lua pattern matching (used extensively in Aleph One scripts)
- **string.format** -- Printf-style formatting (used extensively)
- **string.rep, string.dump**
- io, file, os libraries (not needed for game scripting)
- package/require system (not needed -- we control script loading)
- debug library (not needed)
- load, loadfile, dofile (dynamic code loading)
- `_G` global table

### Architecture
Stackless "trampoline" interpreter design. Execution is reified into pollable objects (like Rust futures). This enables:
- Pre-emptive multitasking via fuel-based execution limiting
- Cancellation without hooks
- GC between polling cycles (via gc-arena's "mutation XOR collection" invariant)

### Performance
Slower than PUC-Rio Lua. The stackless design adds overhead. No JIT. Prioritizes safety and sandboxing over raw speed. No published benchmarks against PUC-Rio Lua.

### Production Use
- **Fish Folk / Jumpy:** Game scripting (including WASM web builds)
- **Ruffle:** ActionScript VM uses the underlying gc-arena crate

### Verdict
**The only mature pure-Rust Lua VM that works on wasm32-unknown-unknown.** But the missing stdlib (especially string pattern matching and string.format) is a showstopper for Aleph One compatibility. Community scripts use `string.format` and `string.find` extensively. Would require significant contribution to the project or a custom stdlib implementation.

---

## Option 3: lua-rs (CppCXY) -- Pure Rust Lua 5.5

**Repository:** [CppCXY/lua-rs](https://github.com/CppCXY/lua-rs)
**Version:** v0.18.1 (April 2026)
**License:** MIT
**Stars:** ~44

### Lua Version Support
Lua 5.5 (faithfully ported from official C source architecture)

### WASM Support
- **wasm32-unknown-unknown:** YES. Dedicated `luars_wasm` crate builds with `wasm-pack --target web`. Uses wasm-bindgen and js-sys.
- Browser-safe runtime with platform time shims
- Full stdlib except filesystem/process/FFI operations

### Standard Library Completeness
Comprehensive. Passes **28 of 30 official Lua 5.5 test files** (all.lua). Includes string library with pattern matching, formatting, table library, math library, coroutines, etc. This is a major advantage over Piccolo.

### Architecture
Register-based VM faithfully porting PUC-Rio's architecture to Rust. Incremental/generational GC, string interning. 80.3% Rust, 17.4% Lua (test files). No C code.

### API Ergonomics
- `load().exec()`, `eval()`, `eval_multi()`
- `register_function()` for Rust callbacks
- `LuaUserData` derive macros
- `register_type()` for exposing Rust types
- TableBuilder for constructing tables
- Async function support via coroutine bridging

### Performance
Acknowledged to be "significantly slower than the native C version." This is expected for a direct port without C-level optimizations. No published benchmark numbers.

### Maturity Concerns
- Very new project (44 stars, 1 fork)
- Single maintainer
- "Experimental and contains many bugs -- use with caution" per author
- Not battle-tested in production
- 2 of 30 test files still failing

### Production Use
Used by EmmyLua Analyzer for parsing Lua configuration files.

### Verdict
**Most promising pure-Rust option for our needs.** Lua 5.5 compatibility with strong stdlib coverage AND working wasm32-unknown-unknown support. The main risks are maturity (very new, single maintainer) and performance (slower than C Lua). The Lua 5.5 version is slightly ahead of Aleph One's Lua 5.3, but 5.5 is backward-compatible. Worth monitoring closely and potentially contributing to.

---

## Option 4: Rhai (Rust-Native Scripting)

**Repository:** [rhaiscript/rhai](https://github.com/rhaiscript/rhai)
**Version:** v1.24.0 (January 2026)
**License:** MIT or Apache-2.0
**Stars:** ~5.3k

### Lua Compatibility
**None.** Rhai is its own language with JavaScript/Rust-like syntax. No Lua compatibility whatsoever. No transpiler from Lua to Rhai exists.

### WASM Support
Excellent. Supports all three targets:
- **wasm32-unknown-unknown:** YES (with `wasm-bindgen` feature)
- **wasm32-wasi:** YES
- **Raw wasm32:** YES (with static hashing, no default features)
- Compiles to <400KB WASM (non-gzipped)
- ~30% slower than native optimized builds

### Performance
~2x slower than Python 3. Significantly slower than Lua. Best used as a thin scripting layer over Rust code, not for heavy computation.

### API Ergonomics
Very good Rust integration. Custom types, getters/setters, operator overloading, module system, closures, AST caching.

### Verdict
**Excellent WASM support but fundamentally incompatible with our requirements.** We need to run existing Aleph One community Lua scripts. There is no path from Lua to Rhai -- the languages are too different, and no transpiler exists. Rewriting decades of community content is not feasible.

---

## Option 5: rlua (Older C Lua Bindings)

**Repository:** [mlua-rs/rlua](https://github.com/mlua-rs/rlua)
**License:** MIT

### Status
Predecessor to mlua, same author. Less maintained, fewer features. The author recommends using mlua instead.

### WASM Support
None. Same C dependency issues as mlua but without the emscripten workaround.

### Verdict
**Obsolete. Use mlua instead.** No advantage over mlua for any use case.

---

## Option 6: silt-lua (Pure Rust Lua Superset)

**Repository:** [auxnon/silt-lua](https://github.com/auxnon/silt-lua)
**Stars:** ~30

### Lua Version Support
Lua-like superset with extensions (bang operators, underscore numbers, implicit returns). Not strictly Lua compatible -- "Source code was not based on lua's source code, so the VM will always have some noticeable differences."

### WASM Support
- **wasm32-unknown-unknown:** YES. Explicit design goal. Live demo at MakeAvoy.com.

### Limitations
- No garbage collector (basic reference counting only)
- Incomplete multiple-return support
- Minimal stdlib (print, clock)
- No releases published
- No metamethods
- 30 stars, 0 forks

### Verdict
**Too immature and incomplete.** Not Lua-compatible enough for Aleph One scripts. Interesting proof-of-concept but not usable for production.

---

## Option 7: Luau via mlua

Luau (Roblox's Lua variant) is accessible through mlua's `luau` feature flag.

### WASM Support
Same as mlua: `wasm32-unknown-emscripten` only. Same incompatibility with our wasm-bindgen stack.

### Advantages
- Type annotations
- Better sandboxing
- Performance optimizations (Luau VM is faster than PUC-Rio Lua for many workloads)

### Verdict
**Same WASM limitation as mlua.** Interesting for native builds but does not solve the browser problem.

---

## Option 8: moonlift (Pure Rust Lua 5.4 + Cranelift JIT)

**Repository:** [HellButcher/moonlift](https://github.com/HellButcher/moonlift)
**Stars:** ~15 | **License:** MIT or Apache-2.0

### Status
Very early development. 14 commits. UTF-8 default strings (differs from standard Lua). Uses Cranelift for JIT compilation.

### WASM Support
Not mentioned. Cranelift JIT would not work in browser WASM.

### Verdict
**Too early-stage and JIT incompatible with WASM.** Not viable.

---

## Option 9: Wasmoon / Fengari (JavaScript-side Lua)

### Wasmoon
- Lua 5.4 VM compiled to WASM via Emscripten, with JavaScript bindings
- npm package, runs in browser/Node/Deno
- Good performance (faster than Fengari)
- JavaScript-side only -- cannot be called from Rust WASM directly

### Fengari
- Lua 5.3 VM rewritten entirely in JavaScript ES6
- Slower than Wasmoon
- Complete Lua C API reimplemented in JS
- Last meaningful update was years ago

### Verdict
**Could serve as a fallback for the web target** if we expose a JavaScript bridge from our Rust WASM code. The architecture would be: Rust WASM calls into JS (via wasm-bindgen), JS calls Wasmoon Lua VM, results flow back. This adds latency and complexity but is technically feasible. See [[lua-wasm-architecture]] for details.

---

## Option 10: mluau (mlua Fork for Luau)

**Repository:** [mluau/mluau](https://github.com/mluau/mluau)

Fork of mlua focused on Luau ecosystem compatibility. Removes async, adds Luau continuations and iterator support. Same WASM limitation as mlua (emscripten only).

### Verdict
**Same blocking issue as mlua.** Not viable for wasm32-unknown-unknown.

---

## Option 11: rluau (Archived)

**Repository:** [vurvdev/rluau](https://github.com/vurvdev/rluau)
Archived April 2022. Author discontinued because "mlua now supports it as a much more viable alternative."

### Verdict
**Dead project.** Do not use.

---

## Option 12: luallaby (Pure Rust Lua)

**Repository:** crates.io/crates/luallaby

Pure Rust Lua compiler and interpreter. Work in progress. Aims for full compliance including Lua test suite. Limited documentation available.

### Verdict
**Insufficient information to evaluate thoroughly.** Appears early-stage. Monitor but do not depend on.

---

## Comparison Table

| Option | Lua Compat | wasm32-unknown-unknown | Stdlib Complete | Maturity | Performance | License |
|---|---|---|---|---|---|---|
| **mlua** | 5.1-5.5, LuaJIT, Luau | NO (emscripten only) | Full (C Lua) | Production | Excellent | MIT |
| **Piccolo** | ~5.3/5.4 partial | YES | NO (string.find/format missing) | Experimental | Moderate | MIT/CC0 |
| **lua-rs** | 5.5 (28/30 tests) | YES (luars_wasm) | Near-complete | Very new | Slower than C | MIT |
| **Rhai** | None (own language) | YES | N/A | Production | Slow | MIT/Apache |
| **silt-lua** | Lua-like superset | YES | NO (minimal) | Very early | Unknown | Unspecified |
| **moonlift** | 5.4 (early) | NO (JIT) | Unknown | Very early | Unknown | MIT/Apache |
| **Wasmoon** | 5.4 | JS-side only | Full (C Lua) | Stable (npm) | Good | MIT |
| **Fengari** | 5.3 | JS-side only | Full (JS Lua) | Stale | Moderate | MIT |

---

## Key Lua Features Required by Aleph One Scripts

Based on the [[lua-vm-integration]] API reference and typical community script usage:

| Feature | Required? | Piccolo | lua-rs | Notes |
|---|---|---|---|---|
| Tables, metatables, metamethods | Critical | YES | YES | Core game object system |
| Closures, upvalues | Critical | YES | YES | Trigger callbacks |
| Coroutines | Important | YES | YES | Camera paths, sequences |
| pcall/xpcall | Critical | YES | YES | Error handling |
| string.format | Critical | NO | YES | HUD text, debug output |
| string.find/match/gmatch/gsub | Critical | NO | YES | String parsing in scripts |
| table.insert/remove/sort | Critical | YES | YES | Inventory management |
| math library | Critical | YES | YES | Geometry calculations |
| tonumber/tostring | Critical | YES | YES | Type conversion |
| pairs/ipairs/next | Critical | YES | YES | Iteration |
| UserData with fields+methods | Critical | Partial | YES (derive macro) | Exposing game objects |
| Sandboxing (no io/os/debug) | Important | YES (by omission) | YES (configurable) | Security |
| Saved game state serialization | Important | Unknown | Unknown | Serde support varies |

---

## Sources

- [mlua on GitHub](https://github.com/mlua-rs/mlua)
- [Piccolo on GitHub](https://github.com/kyren/piccolo)
- [Piccolo blog post](https://kyju.org/blog/piccolo-a-stackless-lua-interpreter/)
- [lua-rs on GitHub](https://github.com/CppCXY/lua-rs)
- [Rhai documentation](https://rhai.rs/book/start/builds/wasm.html)
- [Fish Folk Lua scripting blog](https://fishfolk.org/blog/introducing-lua-scripting-in-jumpy/)
- [Lua in Browser with Rust/WASM guide](https://bytedream.github.io/litbwraw/)
- [Wasmoon on GitHub](https://github.com/ceifa/wasmoon)
- [Fengari on GitHub](https://github.com/fengari-lua/fengari)
- [silt-lua on GitHub](https://github.com/auxnon/silt-lua)
- [Rust scripting benchmarks](https://github.com/khvzak/script-bench-rs)
- [wasm32-unknown-unknown vs emscripten discussion](https://users.rust-lang.org/t/wasm32-unknown-unknown-vs-wasm32-unknown-emscripten-in-2024/112183)
- [bevy_mod_scripting WASM issue](https://github.com/makspll/bevy_mod_scripting/issues/166)
- [Piccolo COMPATIBILITY.md](https://github.com/kyren/piccolo/blob/master/COMPATIBILITY.md)
