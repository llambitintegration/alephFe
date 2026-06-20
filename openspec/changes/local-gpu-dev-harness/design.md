## Context

`marathon-web` compiles to WASM via `wasm-pack`, is served by nginx (`Dockerfile.web`), and renders through `wgpu` v24 with both `webgpu` and `webgl` backends enabled. The existing `web-e2e-tests` change already provides a complete `docker-compose.e2e.yml` stack (`data` → `web` → `tests`) that runs in GitHub CI against software rendering.

The blocker is that interactive investigation of rendering bugs has been attempted on a headless netcup VPS with no GPU. A Phase 0 spike on the IO workstation reproduced and clarified the wall:

- The compose stack serves the app fine on a local port (`:8090`, since the host's `:8080` is already allocated).
- Playwright/Chromium launched **headless with no GPU passthrough** falls back to `ANGLE (… SwiftShader driver)` and `navigator.gpu.requestAdapter()` returns **NULL** (`No available adapters.`) — identical to netcup, *even on the GPU box*.
- Under that software path the WebGL2 fallback still rendered a recognizable textured Marathon frame (loading hidden, canvas 1280×720).

Conclusion: the wall is **browser launch configuration**, not host hardware. IO has the hardware to fix it — 3× NVIDIA GPUs (2× Tesla P40, 1× Quadro M2000), driver 580, `/dev/dri` render nodes, and `nvidia-container-toolkit` 1.19 already wired into Docker via CDI (`nvidia.com/gpu=all`).

## Goals / Non-Goals

**Goals:**
- A repeatable local way to serve `marathon-web` and drive a browser against it with a **real NVIDIA GPU adapter**, for interactive WebGPU/hardware investigation.
- A primary path that needs no container-GPU plumbing (host-headed Chromium on IO's Quadro X session).
- A documented GPU verification probe so "did I actually get hardware?" is a one-command check, not a guess.
- An optional headless GPU-container path for SSH-only / reproducible runs.

**Non-Goals:**
- No `marathon-web` source changes (tooling + docs only).
- No change to the GitHub CI SwiftShader e2e job — it stays as a no-GPU floor.
- No change to the netcup production deployment (client browsers render; the server needs no GPU).
- Not a fast edit→reload dev loop (hot WASM rebuild) — deferred; this harness investigates the current build.
- No guarantee of WebGPU on these specific GPUs (see Risks).

## Decisions

### 1. Reuse the existing compose stack via an override, not a fork

**Choice**: Add a thin dev profile (`docker-compose.dev.yml` or a Compose `profiles:` entry) that composes with `docker-compose.e2e.yml` and only adds `ports: "127.0.0.1:${WEB_PORT:-8090}:80"` to the `web` service. Drop the `tests` service from the dev path.

**Why**: The `data` + `web` services already build the exact prod artifact. Phase 0 proved a stdin override (`-f docker-compose.e2e.yml -f -`) works; a committed file just makes it ergonomic. Binding to `127.0.0.1` keeps the dev server off the network.

**Alternative considered**: A standalone dev compose file duplicating the services — rejected as drift risk against the prod-parity `Dockerfile.web`.

### 2. Port is overridable; default avoids 8080

**Choice**: `WEB_PORT` env var, default `8090`. Bind explicitly to `127.0.0.1`.

**Why**: Phase 0 hit `Bind for 127.0.0.1:8080 failed: port is already allocated` — the host runs another service there. A default that "just works" plus an override covers contention.

### 3. Host-headed Chromium is the primary GPU path (Path 1)

**Choice**: Add a Playwright project / config variant with `headless: false` and `BASE_URL=http://localhost:${WEB_PORT}`, run via host Node (v24 present) directly against `e2e/`. Provide npm scripts: `test:headed`, `test:ui` (`--ui`), `trace`, `codegen`.

**Why**: A headed browser on IO's graphical session uses the Quadro M2000 (the display card) directly — real GPU with zero container-GPU plumbing. `--ui` / trace viewer / codegen are exactly the interactive-investigation surface requested. The Playwright MCP in-session can also drive `localhost:${WEB_PORT}`.

**Alternative considered**: xvfb + host Chromium — gives software GL again (defeats the purpose).

### 4. GPU-passthrough container is the optional secondary path (Path 3)

**Choice**: An optional Playwright runner variant that requests `gpus: all` (or a specific CDI device), injects the NVIDIA userspace/Vulkan ICD into the `mcr.microsoft.com/playwright` image, and launches Chromium with `--use-gl=angle --use-angle=vulkan --enable-features=Vulkan --ignore-gpu-blocklist --no-sandbox`. "Seeing" happens via trace viewer / video / screenshots (optionally a noVNC sidecar).

**Why**: Works over SSH with no desktop session and is reproducible/CI-able on IO later. The toolkit + CDI are already installed, so this is wiring, not setup.

**Alternative considered**: Make this the primary path — rejected; headed-on-host is simpler and the explicit goal is *interactive* investigation, where a live window beats trace replay.

### 5. GPU verification probe shipped as a reusable snippet

**Choice**: Package the Phase 0 probe (reports `requestAdapter()` adapter-vs-NULL and the unmasked WebGL2 `RENDERER`) as a tiny Playwright test/script plus a documented `chrome://gpu` check.

**Why**: SwiftShader vs NVIDIA is the single fact that determines whether a session is meaningful. It must be trivially checkable before any rendering investigation. This directly backs the "GPU adapter verification probe" requirement.

### 6. Documentation states WebGL2-reliable / WebGPU-stretch explicitly

**Choice**: Dev/GPU docs (extend `TESTING.md` or a new dev README) record the expectation: hardware WebGL2 is the reliable target; a NULL WebGPU adapter on Pascal/Maxwell is a known hardware limit, not a harness bug.

**Why**: Prevents future confusion where someone reads `requestAdapter() → NULL` on a hardware session and concludes the harness is broken.

## Risks / Trade-offs

- **WebGPU may not enable on Pascal/Maxwell (Tesla P40 / Quadro M2000)** → Treat hardware **WebGL2** as the success criterion; verify WebGPU per-environment via `chrome://gpu` in Phase 0/2 and document the result. The app's maintained WebGL2 fallback means hardware WebGL2 already satisfies the investigation goal.
- **Container-GPU path: missing NVIDIA libs / wrong flags → silent SwiftShader fallback** → Make the verification probe a gating step of the container path; fail loud if the unmasked renderer is SwiftShader.
- **Host-headed path requires a graphical session** → That's why Path 3 exists as the SSH-only fallback; "both/flexible" host access makes this acceptable.
- **Port contention recurs** → `WEB_PORT` override + `127.0.0.1` binding; document how to pick a free port.
- **Old GPUs / driver 580 quirks under headless EGL** → Tesla P40 has no display engine; prefer the Quadro for headed, use `renderD*` nodes for the container EGL path; capture working flag set in docs.

## Migration Plan

Additive and reversible — no rollback concerns:

1. Land the dev compose profile + `WEB_PORT` override.
2. Land the host-headed Playwright project + npm scripts + the verification probe.
3. Document GPU verification and the WebGL2/WebGPU expectation.
4. (Optional) Land the GPU-passthrough container variant.

CI and netcup are untouched at every step; the harness can be removed by deleting the added files.

## Open Questions

- Does WebGPU actually initialize on the Quadro M2000 / Tesla P40 under driver 580, or do we settle on hardware WebGL2? (Resolve empirically in Phase 2 via `chrome://gpu`.)
- Headed primary path: Quadro M2000 X session confirmed available, or is the container path the day-one default until a graphical session is set up?
- Is a noVNC sidecar wanted for the container path, or are trace viewer + screenshots sufficient for "interactive enough"?
- Should the dev compose profile live in a new file or as a `profiles:` block inside the existing e2e compose?
