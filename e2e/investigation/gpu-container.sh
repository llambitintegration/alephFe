#!/usr/bin/env bash
# GPU-passthrough container runner (boxes 5.1-5.3) — the PRIMARY GPU path.
# Runs the GPU probe inside the official Playwright image with NVIDIA devices
# passed through and ANGLE/Vulkan Chromium flags, headless (no graphical session
# needed). EXPECT_HARDWARE=1 makes the probe a gating step (box 5.3): a silent
# SwiftShader fallback FAILS the run.
#
#   WEB_PORT=8090   dev server port on the host (default 8090)
#   GPU=all         NVIDIA devices to expose (e.g. all | 0 | 2); default all
#   PW_IMAGE=...     Playwright image (must match e2e @playwright/test version)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WEB_PORT="${WEB_PORT:-8090}"
GPU="${GPU:-all}"
TRACE="${TRACE:-0}"   # TRACE=1 -> record a Playwright trace headless and copy it out (box 3.4)
PW_IMAGE="${PW_IMAGE:-mcr.microsoft.com/playwright:v1.48.0-jammy}"

# Reachability check for the dev server (must be up: docker-compose.dev.yml).
if ! curl -sf -o /dev/null "http://localhost:${WEB_PORT}/"; then
  echo "!! Dev server not reachable at http://localhost:${WEB_PORT}/" >&2
  echo "   Start it: docker compose -f docker-compose.e2e.yml -f docker-compose.dev.yml up -d data web" >&2
  exit 1
fi

# Host-side deps (version-matched to the image); browsers come from the image.
if [ ! -d "$REPO_ROOT/e2e/node_modules/@playwright" ]; then
  ( cd "$REPO_ROOT/e2e" && npm ci )
fi

STAMP="$(date +%Y%m%d-%H%M%S)"
EVID_REL="investigation/evidence/container-${STAMP}"
mkdir -p "$REPO_ROOT/e2e/${EVID_REL}"
PW_TRACE_ARG=""
[ "$TRACE" = 1 ] && { PW_TRACE_ARG="--trace on"; echo ">> TRACE on (box 3.4): recording a Playwright trace headless"; }
echo ">> Container GPU probe via $PW_IMAGE (GPU=$GPU); evidence -> e2e/${EVID_REL}"

# --gpus exposes NVIDIA devices; CDI alternative: --device nvidia.com/gpu=all
# --network host so localhost:${WEB_PORT} (the dev server) is reachable.
set +e
docker run --rm \
  --gpus "$GPU" \
  -e NVIDIA_DRIVER_CAPABILITIES=all \
  -e NVIDIA_VISIBLE_DEVICES="$GPU" \
  --network host \
  --user "$(id -u):$(id -g)" \
  -e HOME=/tmp \
  -e PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
  -e HEADED=0 \
  -e EXPECT_HARDWARE=1 \
  -e WEB_PORT="$WEB_PORT" \
  -e BASE_URL="http://localhost:${WEB_PORT}" \
  -e EVID_DIR="/work/e2e/${EVID_REL}" \
  -e PW_EXTRA_ARGS="--use-gl=angle,--use-angle=vulkan,--enable-features=Vulkan,--no-sandbox" \
  -v "$REPO_ROOT:/work" \
  -w /work/e2e \
  "$PW_IMAGE" \
  npx playwright test -c /work/e2e/investigation/playwright.gpu.config.ts $PW_TRACE_ARG
RC=$?
set -e

# Copy the trace out of test-results into the evidence dir (box 3.4).
if [ "$TRACE" = 1 ]; then
  TRACE_ZIP="$(find "$REPO_ROOT/e2e/test-results" -name trace.zip -print 2>/dev/null | head -1)"
  if [ -n "$TRACE_ZIP" ]; then
    cp "$TRACE_ZIP" "$REPO_ROOT/e2e/${EVID_REL}/trace.zip"
    echo ">> trace -> e2e/${EVID_REL}/trace.zip   view: npx playwright show-trace e2e/${EVID_REL}/trace.zip"
    echo "   (remote: npx playwright show-trace --host 0.0.0.0 --port 9323 e2e/${EVID_REL}/trace.zip, then port-forward 9323)"
  else
    echo "!! TRACE=1 but no trace.zip found under e2e/test-results" >&2
  fi
fi

echo
echo ">> PASS = hardware NVIDIA confirmed headless. FAIL ('SOFTWARE' verdict) =>"
echo "   NVIDIA Vulkan ICD not injected or flags wrong (check NVIDIA_DRIVER_CAPABILITIES / CDI)."
echo "   Evidence: e2e/${EVID_REL}/gpu-probe.json"
exit $RC
