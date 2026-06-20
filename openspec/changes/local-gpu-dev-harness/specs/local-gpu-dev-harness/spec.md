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

### Requirement: Host-headed Playwright investigation harness

The harness SHALL provide a Playwright configuration variant and npm scripts that run Chromium on the IO workstation's graphical session in headed and UI modes against the locally served app, such that a real GPU adapter is available to the browser.

#### Scenario: Headed investigation surfaces a hardware GPU adapter

- **WHEN** the developer runs the headed/UI investigation script against `http://localhost:<port>` from a graphical session on the IO workstation
- **THEN** Playwright launches a visible Chromium instance pointed at the locally served app
- **AND** the GPU verification probe reports a hardware GPU adapter (NVIDIA) rather than SwiftShader

#### Scenario: Investigation tooling for diagnosing rendering

- **WHEN** the developer invokes the trace, UI, or codegen scripts
- **THEN** Playwright produces an inspectable trace / UI session / generated script enabling step-by-step investigation of the WASM rendering pipeline

### Requirement: Optional GPU-passthrough container path

The GPU-passthrough container variant of the Playwright runner is optional. When the harness provides it, that variant MUST use the host's NVIDIA Container Toolkit / CDI and ANGLE/Vulkan Chromium flags to run headless with hardware acceleration, so it is usable in environments without a graphical session.

#### Scenario: Headless container obtains a hardware adapter

- **WHEN** the GPU-passthrough Playwright container is run with NVIDIA devices exposed and ANGLE/Vulkan flags set
- **THEN** the GPU verification probe reports a hardware WebGL2 renderer (NVIDIA) rather than SwiftShader
- **AND** the run does not require a graphical desktop session on the host

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
