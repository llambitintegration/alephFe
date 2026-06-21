> **Implementer notes (read before running dev-ops).**
> This change is **tooling + docs only** — no files under `marathon-web/src/`, no CI changes, no netcup changes (see group 6).
> Tasks split into two execution classes:
> - **[agent]** — fully automatable by a headless dev-ops agent (write compose/profile, npm scripts, the probe script, docs, container wiring; HTTP/200 and content-type checks).
> - **[human-gated]** — requires a real GPU adapter, which a **headless** agent cannot obtain (it will fall back to SwiftShader, reproducing the Phase 0 wall). These need either a graphical X session on IO's Quadro M2000 (Path 1) or a GPU-passthrough container run on the IO host with CDI (Path 3). A headless agent should *implement* the harness and stop at the hardware-adapter confirmation, leaving it for a human/host run.
> Order of value: groups 1–2 + 3.1–3.2 ([agent]) stand up the serving + probe + headed config; the [human-gated] confirmations (3.3–3.4, 4.3, 5.4) are run from a GPU session.
>
> **Empirical update (2026-06-20, IO host).** IO is **SSH-only — no graphical session**. The GPU-passthrough container path (group 5) was validated live: a headless container reached a real NVIDIA adapter (`ANGLE (NVIDIA, Vulkan 1.4.312 (NVIDIA Tesla P40))`, hardware WebGL2), with WebGPU `requestAdapter()` → NULL (the expected Pascal-class limit, not a failure). Consequences, reflected below:
> - The **GPU-passthrough container path (group 5) is the day-one primary path.** The **host-headed path (group 3) is deferred/optional** until IO has a graphical session (resolves the design's "X session available?" open question: not today).
> - Boxes **3.3/3.4 are re-scoped to the container path**; the literal *visible-local-window* runs are future work. Box **4.3**'s `chrome://gpu` cross-check is display-dependent and now optional — the probe JSON is the recorded evidence.
> - A scratch toolkit under `scratch/` already runs these on IO (`05-container-probe.sh`, `04-record-result.sh`); evidence lands under `scratch/evidence/` and the 4.3 write-up under this change's `evidence/`.

## 1. Dev compose profile (Phase 1)

- [ ] 1.1 **[agent]** Add a dev compose profile that reuses the `data` + `web` services from `docker-compose.e2e.yml` and publishes nginx as `127.0.0.1:${WEB_PORT:-8090}:80` (default avoids the occupied `:8080`); exclude the `tests` service from this path
- [ ] 1.2 **[agent]** Document the `WEB_PORT` override and the `up`/`down` commands for the dev profile
- [ ] 1.3 **[agent]** Verify: bring the profile up, confirm `data` exits 0, `web` is healthy, `http://127.0.0.1:${WEB_PORT}/` returns HTTP 200, `/pkg/marathon_web_bg.wasm` is `application/wasm`, and all three `/data/*` files return 200
- [ ] 1.4 **[agent]** Verify: with a service already bound to `127.0.0.1:8080`, the dev profile starts with no "port is already allocated" error

## 2. GPU verification probe (Phase 2)

- [ ] 2.1 **[agent]** Package the Phase 0 probe as a runnable Playwright script/test that reports WebGPU `requestAdapter()` adapter-vs-NULL and the unmasked WebGL2 `RENDERER`
- [ ] 2.2 **[agent]** Make the probe classify the result as software (SwiftShader) vs hardware (NVIDIA) and exit/report distinctly
- [ ] 2.3 **[agent]** Document the manual `chrome://gpu` cross-check alongside the probe

## 3. Host-headed Playwright investigation harness (Phase 2) — **deferred/optional (no graphical session on IO)**

> Re-scoped 2026-06-20: IO has no X/Wayland session, so the visible-local-window path can't run today. The hardware-adapter confirmation and interactive artifacts are satisfied via the **container path (group 5)**. 3.1–3.2 still land as [agent] config for future desktop use; 3.3–3.4 are re-pointed at the container path.

- [ ] 3.1 **[agent]** Add a headed/UI Playwright project variant in `e2e/` with `headless: false` and `BASE_URL=http://localhost:${WEB_PORT}` *(for future graphical-session use)*
- [ ] 3.2 **[agent]** Add npm scripts: `test:headed`, `test:ui` (`--ui`), `trace`, `codegen` *(for future graphical-session use)*
- [x] 3.3 **[human-gated]** ~~Verify: run the headed script from a graphical session on IO; confirm a visible Chromium opens…~~ **Re-scoped to the container path:** the hardware-NVIDIA-adapter confirmation is satisfied by box 5.4 (done 2026-06-20: Tesla P40, hardware WebGL2). The literal visible-window run is deferred until IO has a graphical session.
- [ ] 3.4 **[human-gated]** Verify via the container path: generate a Playwright **trace** headless on the real GPU (probe with `--trace on`), copy `trace.zip` out, and step through it with `playwright show-trace` (remote-viewable). `test:ui` (remote-served) optional; `codegen` (needs a visible window) is deferred to the host-headed path.

## 4. Documentation & expectations (Phase 2)

- [ ] 4.1 **[agent]** Extend `TESTING.md` (or add a dev README) covering: dev profile usage, port selection, GPU verification, and the headed-investigation workflow
- [ ] 4.2 **[agent]** Document the WebGL2-reliable / WebGPU-stretch expectation so a NULL WebGPU adapter on Pascal/Maxwell is recorded as a known hardware limit, not a harness failure
- [x] 4.3 **[human-gated]** Record the empirical Phase 2 result. **Recorded 2026-06-20** (`evidence/phase2-gpu-result.md`, container path / Tesla P40): WebGPU `requestAdapter()` → **NULL**; hardware **WebGL2** present (`ANGLE (NVIDIA, Vulkan 1.4.312 …)`). Conclusion: **settle on hardware WebGL2** (known Pascal-class limit). The `chrome://gpu` cross-check is display-dependent → optional/deferred; the probe JSON is the recorded evidence. (Quadro M2000 figures pending a graphical session or a device-pinned container run.)

## 5. GPU-passthrough container path (Phase 3) — **primary path (day-one default on IO)**

> Promoted from optional 2026-06-20: with no graphical session on IO, this is the path that actually reaches a real GPU. Validated live (box 5.4).

- [ ] 5.1 **[agent]** Add a GPU-passthrough Playwright runner variant that exposes NVIDIA devices via `gpus: all`/CDI and injects the NVIDIA Vulkan ICD into the Playwright image
- [ ] 5.2 **[agent]** Set Chromium flags `--use-gl=angle --use-angle=vulkan --enable-features=Vulkan --ignore-gpu-blocklist --no-sandbox` for the container runner
- [ ] 5.3 **[agent]** Make the GPU probe a gating step that fails loudly if the unmasked renderer is SwiftShader instead of NVIDIA
- [x] 5.4 **[human-gated]** Verify: run the container variant headless (no graphical session) and confirm the probe reports a hardware NVIDIA renderer. **Done 2026-06-20** on IO via `scratch/05-container-probe.sh`: `ANGLE (NVIDIA, Vulkan 1.4.312 (NVIDIA Tesla P40))`, gate passed (`EXPECT_HARDWARE=1`); evidence `scratch/evidence/20260620-224640-container-5.4/gpu-probe.json`.
- [ ] 5.5 **[agent]** (Optional) Add a noVNC sidecar or document trace/screenshot review as the "view" mechanism for the container path *(now the primary interactive surface — see box 3.4)*

## 6. Regression guardrails

- [ ] 6.1 **[agent]** Confirm no files under `marathon-web/src/` are modified by this change
- [ ] 6.2 **[agent]** Confirm the GitHub CI SwiftShader e2e job and `docker-compose.e2e.yml` `tests` flow are unchanged
- [ ] 6.3 **[agent]** Confirm the netcup production deployment config for `marathon.llambit.io` is untouched
- [ ] 6.4 **[agent]** Clean up Phase 0 spike artifacts (e.g., `phase0-swiftshader-render.png` in repo root) or relocate them as change evidence — *already done during prep: moved to `evidence/phase0-swiftshader-render.png`; `/logs/` added to `.gitignore`*
