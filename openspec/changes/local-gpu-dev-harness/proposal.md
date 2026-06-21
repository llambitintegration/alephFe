## Why

Interactive Playwright investigation of `marathon-web` rendering bugs is blocked: the deployment/CI runs on a headless netcup VPS with **no GPU adapter**, so `wgpu`'s WebGPU backend never initializes (`requestAdapter()` → NULL, `No available adapters.`). A Phase 0 spike on the IO workstation reproduced the exact wall *on the GPU box itself*: the existing `docker-compose.e2e.yml` stack serves the WASM app fine, and Playwright renders the **WebGL2 path via SwiftShader** (a recognizable textured frame was captured) — but WebGPU still returns NULL because the browser is launched headless with no GPU passthrough. The wall is therefore about **how the browser is launched**, not the host hardware. We need a documented, repeatable local harness that points a browser at a real NVIDIA GPU for interactive WebGPU/hardware investigation.

## What Changes

- Add a **local dev compose profile** that runs the existing `data` + `web` (nginx/WASM) services with a published `localhost` port, avoiding the host's already-occupied `:8080` (Phase 0 hit this — `:8090` is free).
- Add a **host-headed / `--ui` Playwright investigation harness**: an `e2e/` config variant (`headless: false`, `BASE_URL=http://localhost:<port>`) plus npm scripts (`test:ui`, `test:headed`, `trace`, `codegen`) that drive Chromium on IO's Quadro M2000 X session so a **real GPU adapter** is present.
- Add an **optional GPU-passthrough container path** (Phase 3) using the already-installed `nvidia-container-toolkit`/CDI (`nvidia.com/gpu=all`) plus ANGLE/Vulkan Chromium flags (`--use-gl=angle --use-angle=vulkan --enable-features=Vulkan --ignore-gpu-blocklist`) for headless hardware-accelerated runs that don't depend on a desktop session.
- Add **documentation** (TESTING.md or a dev README) covering port selection, GPU verification (`chrome://gpu`, `requestAdapter()` probe), and the WebGL2-vs-WebGPU expectation.
- **No `marathon-web` source changes** — tooling and docs only (mirrors how `web-e2e-tests` was scoped).
- **Unchanged**: the GitHub CI SwiftShader e2e job stays as-is (a useful no-GPU floor); netcup remains the production host (browser-side rendering needs no server GPU).

## Capabilities

### New Capabilities
- `local-gpu-dev-harness`: A repeatable local development and investigation harness on the IO workstation that serves the `marathon-web` WASM build and drives a Playwright/Chromium browser against a real NVIDIA GPU adapter (host-headed primary path; GPU-passthrough container optional), with documented GPU verification and WebGL2/WebGPU expectations.

### Modified Capabilities
<!-- None. The existing `web-e2e-testing` capability's requirements are unchanged; this adds a parallel local-investigation harness rather than altering CI e2e behavior. -->

## Impact

- **New files**: a dev compose file/profile (e.g. `docker-compose.dev.yml`), an `e2e/` headed/UI Playwright project + npm scripts, dev/GPU documentation.
- **Host prerequisites (already satisfied on IO)**: Docker 29 + Compose v5, Node 24, NVIDIA driver 580, `nvidia-container-toolkit` 1.19 with Docker CDI runtime, `/dev/dri` render nodes, 3× NVIDIA GPUs (2× Tesla P40, 1× Quadro M2000).
- **Crates affected**: `marathon-web` (investigation target only — no source changes).
- **Risk to validate** *(resolved 2026-06-20)*: WebGPU does **not** enable on the Tesla P40 (`requestAdapter()` → NULL); **hardware WebGL2 is the confirmed reliable target** (`ANGLE (NVIDIA, Vulkan 1.4.312 …)`). Also resolved: IO is **SSH-only**, so the **GPU-passthrough container path is the day-one primary** and the host-headed path is deferred until a graphical session exists (inverts the original path priority).
- **Incidental findings to triage separately**: `Failed to load sprite collection 13 / 20 / 25 — out of range (0-31)` surfaced during the spike (not in scope here).
