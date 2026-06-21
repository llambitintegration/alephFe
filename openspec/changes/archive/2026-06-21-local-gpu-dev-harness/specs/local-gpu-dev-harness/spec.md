## ADDED Requirements

### Requirement: Local web app serving via dev compose profile

The harness SHALL provide a Docker Compose configuration that builds and serves the `marathon-web` WASM app locally by reusing the existing `data` and `web` services, publishing nginx on a configurable host port bound to `127.0.0.1`. The default published port MUST NOT be `8080` (occupied on the IO host) and SHALL be overridable without editing the committed file.

#### Scenario: Stack serves the WASM app on the chosen port

- **WHEN** the developer brings up the dev compose profile
- **THEN** the `data` service fetches Marathon 2 game data and exits successfully
- **AND** the `web` service reports healthy and returns HTTP 200 at `http://127.0.0.1:<port>/`
- **AND** `/pkg/marathon_web_bg.wasm` is served with `Content-Type: application/wasm` and `/data/Map.sceA`, `/data/Shapes.shpA`, `/data/Physics.phyA` return HTTP 200

#### Scenario: Default port avoids the occupied host port

- **WHEN** the dev compose profile is started with no port override on a host where `127.0.0.1:8080` is already allocated
- **THEN** the `web` container binds its published port successfully without a "port is already allocated" error

### Requirement: GPU adapter verification probe

The harness SHALL provide a documented, repeatable way to verify whether the browser under investigation has a real GPU adapter, reporting both the WebGPU adapter status and the unmasked WebGL2 renderer string.

#### Scenario: Probe distinguishes software from hardware rendering

- **WHEN** the verification probe is run against the served app in a given browser
- **THEN** it reports whether `navigator.gpu.requestAdapter()` returns an adapter or NULL
- **AND** it reports the unmasked WebGL2 renderer string, making a SwiftShader (software) result distinguishable from an NVIDIA (hardware) result

### Requirement: Optional host-headed Playwright investigation harness

The harness MAY provide a Playwright configuration variant and npm scripts that run Chromium in headed and UI modes against the locally served app. This path requires a graphical (X/Wayland) session on the host; where none exists (e.g. the SSH-only IO workstation), the GPU-passthrough container path is the primary GPU path and this host-headed path is deferred until a graphical session is available. When a graphical session is present, the headed variant SHALL make a real GPU adapter available to the browser.

#### Scenario: Headed investigation surfaces a hardware GPU adapter (when a graphical session exists)

- **WHEN** the developer runs the headed/UI investigation script against `http://localhost:<port>` from a graphical session on the IO workstation
- **THEN** Playwright launches a visible Chromium instance pointed at the locally served app
- **AND** the GPU verification probe reports a hardware GPU adapter (NVIDIA) rather than SwiftShader

#### Scenario: Investigation tooling for diagnosing rendering

- **WHEN** the developer invokes the trace, UI, or codegen scripts
- **THEN** Playwright produces an inspectable trace / UI session / generated script enabling step-by-step investigation of the WASM rendering pipeline

### Requirement: GPU-passthrough container path (primary GPU path)

The harness SHALL provide a GPU-passthrough Playwright runner that uses the host's NVIDIA Container Toolkit / CDI and ANGLE/Vulkan Chromium flags to run headless with hardware acceleration, so it is usable in environments without a graphical session. This is the primary path for obtaining a real GPU adapter on hosts that lack a graphical session (such as the SSH-only IO workstation). The GPU verification probe SHALL gate this path, failing loudly when the unmasked renderer is software (SwiftShader) rather than NVIDIA.

#### Scenario: Headless container obtains a hardware adapter

- **WHEN** the GPU-passthrough Playwright container is run with NVIDIA devices exposed and ANGLE/Vulkan flags set
- **THEN** the GPU verification probe reports a hardware WebGL2 renderer (NVIDIA) rather than SwiftShader
- **AND** the run does not require a graphical desktop session on the host

#### Scenario: Probe gates against a silent software fallback

- **WHEN** the GPU-passthrough container runs but the NVIDIA Vulkan ICD is not injected or the flags are wrong, so Chromium falls back to SwiftShader
- **THEN** the gating GPU probe fails the run rather than reporting a passing software render

### Requirement: WebGL2 reliability with WebGPU as a stretch target

The harness documentation SHALL state that hardware WebGL2 is the reliable rendering target on the available GPUs and that WebGPU availability must be confirmed per environment, so that a NULL WebGPU adapter on Pascal/Maxwell hardware is not mistaken for a harness failure.

#### Scenario: WebGPU unavailability is documented, not a failure

- **WHEN** WebGPU `requestAdapter()` returns NULL on the IO GPUs while hardware WebGL2 is available
- **THEN** the documented expectation identifies this as a known hardware limitation
- **AND** the harness still enables hardware WebGL2 investigation of the rendering pipeline

### Requirement: No production or CI regressions

This change SHALL be limited to local development and investigation tooling and documentation, making no `marathon-web` source changes, no changes to the production netcup deployment, and no changes to the existing GitHub CI SwiftShader e2e job.

#### Scenario: CI and production remain unchanged

- **WHEN** this change is applied
- **THEN** the GitHub CI e2e job continues to run unchanged against software rendering
- **AND** the netcup production host configuration for serving `marathon.llambit.io` is unaffected
- **AND** no files under `marathon-web/src/` are modified
