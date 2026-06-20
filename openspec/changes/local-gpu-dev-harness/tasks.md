> **Implementer notes (read before running dev-ops).**
> This change is **tooling + docs only** — no files under `marathon-web/src/`, no CI changes, no netcup changes (see group 6).
> Tasks split into two execution classes:
> - **[agent]** — fully automatable by a headless dev-ops agent (write compose/profile, npm scripts, the probe script, docs, container wiring; HTTP/200 and content-type checks).
> - **[human-gated]** — requires a real GPU adapter, which a **headless** agent cannot obtain (it will fall back to SwiftShader, reproducing the Phase 0 wall). These need either a graphical X session on IO's Quadro M2000 (Path 1) or a GPU-passthrough container run on the IO host with CDI (Path 3). A headless agent should *implement* the harness and stop at the hardware-adapter confirmation, leaving it for a human/host run.
> Order of value: groups 1–2 + 3.1–3.2 ([agent]) stand up the serving + probe + headed config; the [human-gated] confirmations (3.3–3.4, 4.3, 5.4) are run from a GPU session.

## 1. Dev compose profile (Phase 1)

- [ ] 1.1 **[agent]** Add a dev compose profile that reuses the `data` + `web` services from `docker-compose.e2e.yml` and publishes nginx as `127.0.0.1:${WEB_PORT:-8090}:80` (default avoids the occupied `:8080`); exclude the `tests` service from this path
- [ ] 1.2 **[agent]** Document the `WEB_PORT` override and the `up`/`down` commands for the dev profile
- [ ] 1.3 **[agent]** Verify: bring the profile up, confirm `data` exits 0, `web` is healthy, `http://127.0.0.1:${WEB_PORT}/` returns HTTP 200, `/pkg/marathon_web_bg.wasm` is `application/wasm`, and all three `/data/*` files return 200
- [ ] 1.4 **[agent]** Verify: with a service already bound to `127.0.0.1:8080`, the dev profile starts with no "port is already allocated" error

## 2. GPU verification probe (Phase 2)

- [ ] 2.1 **[agent]** Package the Phase 0 probe as a runnable Playwright script/test that reports WebGPU `requestAdapter()` adapter-vs-NULL and the unmasked WebGL2 `RENDERER`
- [ ] 2.2 **[agent]** Make the probe classify the result as software (SwiftShader) vs hardware (NVIDIA) and exit/report distinctly
- [ ] 2.3 **[agent]** Document the manual `chrome://gpu` cross-check alongside the probe

## 3. Host-headed Playwright investigation harness (Phase 2)

- [ ] 3.1 **[agent]** Add a headed/UI Playwright project variant in `e2e/` with `headless: false` and `BASE_URL=http://localhost:${WEB_PORT}`
- [ ] 3.2 **[agent]** Add npm scripts: `test:headed`, `test:ui` (`--ui`), `trace`, `codegen`
- [ ] 3.3 **[human-gated]** Verify: run the headed script from a graphical session on IO; confirm a visible Chromium opens against the served app and the probe reports a hardware NVIDIA adapter (not SwiftShader)
- [ ] 3.4 **[human-gated]** Verify: `test:ui`, `trace`, and `codegen` each produce an inspectable artifact/session for stepping through the WASM rendering pipeline

## 4. Documentation & expectations (Phase 2)

- [ ] 4.1 **[agent]** Extend `TESTING.md` (or add a dev README) covering: dev profile usage, port selection, GPU verification, and the headed-investigation workflow
- [ ] 4.2 **[agent]** Document the WebGL2-reliable / WebGPU-stretch expectation so a NULL WebGPU adapter on Pascal/Maxwell is recorded as a known hardware limit, not a harness failure
- [ ] 4.3 **[human-gated]** Record the empirical Phase 2 result (does WebGPU initialize on Quadro M2000 / Tesla P40 under driver 580, or do we settle on hardware WebGL2?) — depends on the 3.3 GPU-session run

## 5. Optional GPU-passthrough container path (Phase 3)

- [ ] 5.1 **[agent]** Add a GPU-passthrough Playwright runner variant that exposes NVIDIA devices via `gpus: all`/CDI and injects the NVIDIA Vulkan ICD into the Playwright image
- [ ] 5.2 **[agent]** Set Chromium flags `--use-gl=angle --use-angle=vulkan --enable-features=Vulkan --ignore-gpu-blocklist --no-sandbox` for the container runner
- [ ] 5.3 **[agent]** Make the GPU probe a gating step that fails loudly if the unmasked renderer is SwiftShader instead of NVIDIA
- [ ] 5.4 **[human-gated]** Verify: run the container variant headless (no graphical session) and confirm the probe reports a hardware NVIDIA renderer (needs the IO host's GPU + CDI; not reproducible inside a generic worktree agent)
- [ ] 5.5 **[agent]** (Optional) Add a noVNC sidecar or document trace/screenshot review as the "view" mechanism for the container path

## 6. Regression guardrails

- [ ] 6.1 **[agent]** Confirm no files under `marathon-web/src/` are modified by this change
- [ ] 6.2 **[agent]** Confirm the GitHub CI SwiftShader e2e job and `docker-compose.e2e.yml` `tests` flow are unchanged
- [ ] 6.3 **[agent]** Confirm the netcup production deployment config for `marathon.llambit.io` is untouched
- [ ] 6.4 **[agent]** Clean up Phase 0 spike artifacts (e.g., `phase0-swiftshader-render.png` in repo root) or relocate them as change evidence — *already done during prep: moved to `evidence/phase0-swiftshader-render.png`; `/logs/` added to `.gitignore`*
