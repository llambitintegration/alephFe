# marathon-web GPU / rendering investigation harness

A local harness for serving the `marathon-web` WASM build and driving Chromium
against a **real NVIDIA GPU** for interactive WebGPU/hardware rendering
investigation. Tooling + docs only — no `marathon-web/src`, CI, or production
changes.

> **Primary path on IO is the GPU-passthrough container** (`npm run gpu:container`).
> IO is SSH-only (no graphical session), so the host-headed path is deferred —
> see "Host-headed path" below.

## 1. Serve the app (dev compose profile)

Composes with `docker-compose.e2e.yml`; adds only a `127.0.0.1`-published port to
the existing `web` service (the `tests` service is excluded). From the repo root:

```bash
# up (data fetch + nginx/WASM web), default port 8090
docker compose -f docker-compose.e2e.yml -f docker-compose.dev.yml up -d data web

# down
docker compose -f docker-compose.e2e.yml -f docker-compose.dev.yml down
```

**Port selection** — default `8090` (the host's `:8080` is already allocated).
Override without editing the file:

```bash
WEB_PORT=9001 docker compose -f docker-compose.e2e.yml -f docker-compose.dev.yml up -d data web
```

Verify: `http://127.0.0.1:${WEB_PORT}/` → 200, `/pkg/marathon_web_bg.wasm` →
`application/wasm`, `/data/Map.sceA|Shapes.shpA|Physics.phyA` → 200.

## 2. GPU verification probe — "did I actually get hardware?"

The single fact that determines whether a session is meaningful: SwiftShader
(software) vs NVIDIA (hardware). The probe reports the WebGPU
`requestAdapter()` adapter-vs-NULL result and the **unmasked WebGL2 renderer**,
classifies the result, and (with `EXPECT_HARDWARE=1`) **fails** on a software
fallback. Run from `e2e/`:

```bash
npm ci                  # one-time
npm run gpu:probe       # headless probe against the dev server
```

Output: a verdict block in the log plus `investigation/evidence/<run>/gpu-probe.json`
and a screenshot.

**`chrome://gpu` cross-check** (manual, needs a browser/display): open
`chrome://gpu` and read the *Graphics Feature Status* block — `WebGL` / `WebGL2`
hardware-accelerated, `WebGPU` / `Vulkan` status. Use it to corroborate the probe
when a display is available.

## 3. GPU-passthrough container path (primary)

Headless, no graphical session needed. Exposes NVIDIA devices (`--gpus`) + the
NVIDIA Vulkan ICD and sets ANGLE/Vulkan Chromium flags; the probe gates the run
(fails loudly on SwiftShader). From `e2e/`:

```bash
npm run gpu:container              # all GPUs
GPU=2 npm run gpu:container        # pin a specific GPU (e.g. the Quadro)
TRACE=1 npm run gpu:container      # also record a Playwright trace headless
```

With `TRACE=1`, a `trace.zip` is recorded on the real GPU and copied into the
run's evidence dir. Step through it (locally or remotely) with:

```bash
npx playwright show-trace e2e/investigation/evidence/<run>/trace.zip
# remote (SSH-only): add --host 0.0.0.0 --port 9323, then port-forward 9323
```

Requires the dev server up (step 1) and `nvidia-container-toolkit`/CDI on the
host (already present on IO). "Seeing" the run is via the probe screenshot and
Playwright traces/screenshots (a noVNC sidecar could be added if a live window is
ever wanted — trace + screenshots have been sufficient so far).

## 4. Host-headed path (deferred — needs a graphical session)

`headless: false` against `http://localhost:${WEB_PORT}`, for a visible Chromium
on the GPU's X session. **IO currently has no X/Wayland session**, so these are
for future desktop use:

```bash
npm run gpu:headed     # visible Chromium probe
npm run test:ui        # Playwright UI mode (interactive runner)
npm run trace          # run with tracing on; view with: npx playwright show-trace <trace.zip>
npm run codegen        # generate a script against the live app
```

## 5. Expectation: WebGL2 reliable, WebGPU a stretch

On the IO GPUs (Pascal-class Tesla P40, Maxwell-class Quadro M2000) **hardware
WebGL2 is the reliable target**. A **NULL WebGPU adapter is a known hardware
limit, not a harness failure** — confirmed empirically on the Tesla P40
(`requestAdapter()` → NULL while hardware WebGL2 renders via
`ANGLE (NVIDIA, Vulkan …)`). The app's maintained WebGL2 fallback means hardware
WebGL2 already satisfies the investigation goal. Confirm WebGPU per-environment
via the probe + `chrome://gpu`.

## CI / production are untouched

This harness is opt-in: the probe lives under `investigation/` with its own
config, so the CI e2e job (`playwright.config.ts`, `testDir: ./tests`) and the
`docker-compose.e2e.yml` `tests` flow are unaffected, and there are no
`marathon-web/src` or netcup production changes.
